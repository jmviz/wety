//! The embedding model: what decides an embedding's values. The cache's namespace is
//! the model, its revision and this file's text (`namespace`), so editing this file,
//! even to reformat it, discards the cache: hours of embedding. Everything else about
//! embeddings (the store, which texts are embedded, scoring) is in `embeddings.rs`.

use std::path::PathBuf;

use anyhow::{Error, Result};
use xxhash_rust::xxh3::xxh3_64;

#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use candle_core::{
    Device, IndexOp, Tensor,
    utils::{cuda_is_available, metal_is_available},
};
use candle_nn::VarBuilder;
use candle_transformers::models::{
    bert::{self, BertModel, DTYPE, HiddenAct},
    modernbert::{self, ModernBert},
};
use hf_hub::{Repo, RepoType, api::sync::Api};
use tokenizers::Tokenizer;

fn device() -> Result<Device> {
    if cuda_is_available() {
        println!("Running embeddings model on GPU (CUDA).");
        return Ok(Device::new_cuda(0)?);
    }
    if metal_is_available() {
        println!("Running embeddings model on GPU (Metal).");
        return Ok(Device::new_metal(0)?);
    }

    println!("Running embeddings model on CPU.");
    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "aarch64")]
        {
            println!("To run on GPU with Metal, build with `--features metal`.");
        }
        #[cfg(not(any(feature = "accelerate", target_arch = "aarch64")))]
        {
            println!("For accelerated CPU processing, build with `--features accelerate`.");
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(not(feature = "cuda"))]
        {
            println!("If you have a CUDA GPU, use it by building with `--features cuda`.");
        }
        #[cfg(all(not(feature = "mkl"), target_arch = "x86_64"))]
        {
            println!("For accelerated CPU processing, build with `--features mkl`.");
        }
    }
    Ok(Device::Cpu)
}

// The supported architectures, with the pooling their sentence-transformers
// configurations use.
enum Inner {
    // mean pooling (e.g. sentence-transformers/all-MiniLM-L12-v2)
    Bert(BertModel),
    // CLS pooling (e.g. ibm-granite/granite-embedding-small-english-r2)
    ModernBert(ModernBert),
}

pub(crate) struct Model {
    device: Device,
    inner: Inner,
    tokenizer: Tokenizer,
}

// adapted from https://github.com/huggingface/candle/blob/main/candle-examples/examples/bert/main.rs
impl Model {
    pub(crate) fn new(model_name: String, revision: String) -> Result<Self> {
        let device = device()?;

        // hf-hub 0.3 resolves cache entries through refs; a commit may have no ref
        // even when its complete immutable snapshot is already present.
        let snapshot = hf_hub::Cache::default().path()
            .join(format!("models--{}", model_name.replace('/', "--")))
            .join("snapshots").join(&revision);
        let repo = Repo::with_revision(model_name, RepoType::Model, revision);

        let file = |name: &str| -> Result<PathBuf> {
            if snapshot.join(name).is_file() {
                return Ok(snapshot.join(name));
            }
            Ok(Api::new()?.repo(repo.clone()).get(name)?)
        };
        let config_filename = file("config.json")?;
        let config = std::fs::read_to_string(config_filename)?;
        let model_type = serde_json::from_str::<serde_json::Value>(&config)?
            .get("model_type")
            .and_then(|t| t.as_str().map(str::to_owned))
            .unwrap_or_default();

        // No tokenizer padding: encode() pads each length-sorted sub-batch itself.
        // Texts are truncated to 256 tokens (glosses and etymologies rarely
        // need more).
        let mut tokenizer = Tokenizer::from_file(file("tokenizer.json")?).map_err(Error::msg)?;
        tokenizer.with_padding(None);
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: 256,
                ..Default::default()
            }))
            .map_err(Error::msg)?;

        let inner = if model_type == "modernbert" {
            let config: modernbert::Config = serde_json::from_str(&config)?;
            // candle expects the weights under "model."; sentence-transformers
            // checkpoints have them at the root.
            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[file("model.safetensors")?], DTYPE, &device)?
            }
            .rename_f(|name: &str| name.strip_prefix("model.").unwrap_or(name).to_owned());
            Inner::ModernBert(ModernBert::load(vb, &config)?)
        } else {
            let vb = VarBuilder::from_pth(file("pytorch_model.bin")?, DTYPE, &device)?;
            let mut config: bert::Config = serde_json::from_str(&config)?;
            config.hidden_act = HiddenAct::GeluApproximate;
            Inner::Bert(BertModel::load(vb, &config)?)
        };

        Ok(Self {
            device,
            inner,
            tokenizer,
        })
    }

    /// Embeds texts, in input order. Texts are encoded in sub-batches of similar
    /// length, each padded only to its own longest text, and token outputs are
    /// mean-pooled over real tokens (attention mask): an embedding does not depend
    /// on which other texts share its batch, and short texts cost little.
    pub(crate) fn encode(&self, texts: Vec<String>) -> Result<Tensor> {
        const SUB_BATCH: usize = 64;
        let n = texts.len();
        let encodings = self
            .tokenizer
            .encode_batch(texts, true)
            .map_err(Error::msg)?;
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&i| encodings[i].get_ids().len());
        let mut rows: Vec<Option<Vec<f32>>> = vec![None; n];
        for chunk in order.chunks(SUB_BATCH) {
            let width = chunk
                .iter()
                .map(|&i| encodings[i].get_ids().len())
                .max()
                .unwrap_or(1)
                .max(1);
            let mut ids = Vec::with_capacity(chunk.len() * width);
            let mut mask = Vec::with_capacity(chunk.len() * width);
            for &i in chunk {
                let tokens = encodings[i].get_ids();
                ids.extend_from_slice(tokens);
                ids.extend(std::iter::repeat_n(0u32, width - tokens.len()));
                mask.extend(std::iter::repeat_n(1u32, tokens.len()));
                mask.extend(std::iter::repeat_n(0u32, width - tokens.len()));
            }
            let ids = Tensor::from_vec(ids, (chunk.len(), width), &self.device)?;
            let mask = Tensor::from_vec(mask, (chunk.len(), width), &self.device)?;
            let pooled = match &self.inner {
                Inner::Bert(model) => {
                    let token_type_ids = ids.zeros_like()?;
                    let hidden = model.forward(&ids, &token_type_ids, Some(&mask))?;
                    let weights = mask.to_dtype(hidden.dtype())?.unsqueeze(2)?;
                    hidden
                        .broadcast_mul(&weights)?
                        .sum(1)?
                        .broadcast_div(&weights.sum(1)?)?
                }
                Inner::ModernBert(model) => {
                    let hidden = model.forward(&ids, &mask)?;
                    hidden.i((.., 0, ..))?.contiguous()?
                }
            };
            for (&i, row) in chunk.iter().zip(normalize_l2(&pooled)?.to_vec2::<f32>()?) {
                rows[i] = Some(row);
            }
        }
        let width = rows.first().and_then(|r| r.as_ref()).map_or(0, Vec::len);
        let flat: Vec<f32> = rows.into_iter().flatten().flatten().collect();
        Ok(Tensor::from_vec(flat, (n, width), &Device::Cpu)?)
    }
}

fn normalize_l2(v: &Tensor) -> Result<Tensor> {
    Ok(v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)?)
}

/// The cache namespace of a model at a revision, as embedded by this file's code.
pub(crate) fn namespace(model_name: &str, revision: &str) -> u64 {
    xxh3_64(format!("{model_name}\0{revision}\0{}", include_str!("model.rs")).as_bytes())
}
