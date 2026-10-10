//! Text embeddings of items, for choosing among homographs (see resolve.rs).
//!
//! Each item gets embeddings of each of its senses' glosses, of its glosses
//! joined, and of its etymology text; a citation may also give a gloss of the
//! cited term (t=). Embeddings are cached by text, under a namespace of the
//! model, its revision and this file.

use crate::HashMap;
use wety_core::items::{Item, ItemId};

use std::{hash::Hash, mem, path::PathBuf, rc::Rc};

use anyhow::{Error, Result};

use sled::{self, Db, IVec};
use xxhash_rust::xxh3::xxh3_64;

type Embedding = Vec<f32>;

/// For an `imputed_item` embedding, we use the embedding for
/// `imputed_item.from`, weighted by this discount factor
pub(crate) const IMPUTATION_DISCOUNT: f32 = 0.5;

// senses per item to embed, and characters of etymology text
const MAX_SENSES: usize = 12;
const MAX_ETY_CHARS: usize = 600;

pub(crate) struct ItemEmbedding {
    ety: Option<Embedding>,
    glosses: Option<Embedding>,
    senses: Vec<Embedding>,
    discount: f32,
}

type TextHash = u64;

fn text_hash(text: &str) -> TextHash {
    xxh3_64(text.as_bytes())
}

fn to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|e| e.to_be_bytes()).collect()
}

fn to_embedding(bytes: &IVec) -> Embedding {
    // the 4 here assumes Embedding elements are f32
    let (chunks, _) = bytes.as_ref().as_chunks::<4>();
    chunks.iter().map(|&b| f32::from_be_bytes(b)).collect()
}

// Texts waiting to be embedded, by key.
struct Batch<K> {
    max_size: usize,
    model: Rc<Model>,
    cache: Rc<Db>,
    keys: Vec<K>,
    texts: Vec<String>,
    text_hashes: Vec<TextHash>,
}

impl<K> Batch<K> {
    fn new(model: &Rc<Model>, size: usize, cache: &Rc<Db>) -> Self {
        Self {
            keys: Vec::with_capacity(size),
            texts: Vec::with_capacity(size),
            text_hashes: Vec::with_capacity(size),
            max_size: size,
            model: Rc::clone(model),
            cache: Rc::clone(cache),
        }
    }

    // Add a text; when the batch is full, embed and cache it, returning the
    // keys it embedded.
    fn update(&mut self, key: K, text: String, text_hash: TextHash) -> Result<Vec<(K, TextHash)>> {
        self.keys.push(key);
        self.texts.push(text);
        self.text_hashes.push(text_hash);
        if self.keys.len() >= self.max_size {
            return self.flush();
        }
        Ok(vec![])
    }

    fn flush(&mut self) -> Result<Vec<(K, TextHash)>> {
        if self.keys.is_empty() {
            return Ok(vec![]);
        }
        let keys = mem::take(&mut self.keys);
        let text_hashes = mem::take(&mut self.text_hashes);
        let texts = mem::take(&mut self.texts);
        let embeddings = self.model.encode(texts)?.to_vec2::<f32>()?;
        let mut batch = sled::Batch::default();
        for (text_hash, embedding) in text_hashes.iter().zip(&embeddings) {
            batch.insert(&text_hash.to_be_bytes(), to_bytes(embedding));
        }
        self.cache.apply_batch(batch)?;
        Ok(keys.into_iter().zip(text_hashes).collect())
    }
}

// Embeddings by key (an item, an item's sense, a text), through the cache.
struct EmbeddingsMap<K> {
    batch: Batch<K>,
    map: HashMap<K, TextHash>,
    cache: Rc<Db>,
}

impl<K: Copy + Eq + Hash> EmbeddingsMap<K> {
    fn new(model: &Rc<Model>, batch_size: usize, cache: &Rc<Db>) -> Self {
        Self {
            batch: Batch::new(model, batch_size, cache),
            map: HashMap::default(),
            cache: Rc::clone(cache),
        }
    }

    fn update(&mut self, key: K, text: String) -> Result<()> {
        let hash = text_hash(&text);
        if self.cache.contains_key(hash.to_be_bytes())? {
            self.map.insert(key, hash);
            return Ok(());
        }
        for (key, hash) in self.batch.update(key, text, hash)? {
            self.map.insert(key, hash);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        for (key, hash) in self.batch.flush()? {
            self.map.insert(key, hash);
        }
        Ok(())
    }

    fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    fn get(&self, key: K) -> Result<Option<Embedding>> {
        if let Some(hash) = self.map.get(&key)
            && let Some(bytes) = self.cache.get(hash.to_be_bytes())?
        {
            return Ok(Some(to_embedding(&bytes)));
        }
        Ok(None)
    }
}

/// For other options, see:
/// <https://huggingface.co/models?library=sentence-transformers&sort=trending>.
/// BERT (mean pooling) and ModernBERT (CLS pooling) checkpoints are supported;
/// sentence-transformers/all-MiniLM-L12-v2 is about 2.4x faster here, and
/// somewhat less accurate for choosing homographs.
pub const DEFAULT_MODEL: &str = "ibm-granite/granite-embedding-small-english-r2";
pub const DEFAULT_MODEL_REVISION: &str = "2ab6fa8ea2d674564defd37171ae19079b864b33";
pub const DEFAULT_BATCH_SIZE: usize = 800;

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

struct Model {
    device: Device,
    inner: Inner,
    tokenizer: Tokenizer,
}

// adapted from https://github.com/huggingface/candle/blob/main/candle-examples/examples/bert/main.rs
impl Model {
    fn new(model_name: String, revision: String) -> Result<Self> {
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
    fn encode(&self, texts: Vec<String>) -> Result<Tensor> {
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

pub struct Config {
    pub model_name: String,
    pub model_revision: String,
    pub batch_size: usize,
    pub cache_path: PathBuf,
}

pub(crate) struct Embeddings {
    ety: EmbeddingsMap<ItemId>,
    glosses: EmbeddingsMap<ItemId>,
    senses: EmbeddingsMap<(ItemId, u8)>,
    n_senses: HashMap<ItemId, u8>,
    texts: EmbeddingsMap<TextHash>,
    cache: Rc<Db>,
}

impl Embeddings {
    pub(crate) fn new(config: &Config) -> Result<Self> {
        anyhow::ensure!(config.model_revision.len() == 40 && config.model_revision.bytes().all(|b| b.is_ascii_hexdigit()), "Embedding model revision must be an immutable 40-character commit");

        let model = Rc::from(Model::new(
            config.model_name.clone(),
            config.model_revision.clone(),
        )?);
        let namespace = xxh3_64(format!("{}\0{}\0{}", config.model_name, config.model_revision, include_str!("embeddings.rs")).as_bytes());
        let cache = Rc::from(sled::open(config.cache_path.join(format!("{namespace:016x}")))?);
        Ok(Self {
            ety: EmbeddingsMap::new(&model, config.batch_size, &cache),
            glosses: EmbeddingsMap::new(&model, config.batch_size, &cache),
            senses: EmbeddingsMap::new(&model, config.batch_size, &cache),
            n_senses: HashMap::default(),
            texts: EmbeddingsMap::new(&model, config.batch_size, &cache),
            cache,
        })
    }

    pub(crate) fn add(
        &mut self,
        item_id: ItemId,
        item_lang: &str,
        item_term: &str,
        ety_text: Option<&str>,
        glosses: &[String],
    ) -> Result<()> {
        if let Some(ety_text) = ety_text {
            // The lang name and term are prepended to the ety text, so that an
            // etymology naming a term is near that term's own etymology.
            let ety_text: String = ety_text.chars().take(MAX_ETY_CHARS).collect();
            self.ety.update(item_id, format!("{item_lang} {item_term}. {ety_text}"))?;
        }
        let glosses = &glosses[..glosses.len().min(MAX_SENSES)];
        if !glosses.is_empty() {
            self.glosses.update(item_id, glosses.join(" "))?;
            for (n, gloss) in (0u8..).zip(glosses) {
                self.senses.update((item_id, n), gloss.clone())?;
            }
            #[allow(clippy::cast_possible_truncation)]
            self.n_senses.insert(item_id, glosses.len() as u8);
        }
        Ok(())
    }

    /// Embed a text that is not an item's, e.g. a gloss a citation gives.
    pub(crate) fn add_text(&mut self, text: &str) -> Result<()> {
        let hash = text_hash(text);
        if !self.texts.contains(&hash) {
            self.texts.update(hash, text.to_owned())?;
        }
        Ok(())
    }

    pub(crate) fn flush(&mut self) -> Result<()> {
        self.ety.flush()?;
        self.glosses.flush()?;
        self.senses.flush()?;
        self.texts.flush()?;
        self.cache.flush()?;
        Ok(())
    }

    pub(crate) fn text(&self, text: &str) -> Result<Option<Embedding>> {
        self.texts.get(text_hash(text))
    }

    pub(crate) fn get(&self, item: &Item, item_id: ItemId) -> Result<ItemEmbedding> {
        let (id, discount) = match item {
            Item::Real(_) => (item_id, 1.0),
            Item::Imputed(imputed) => (imputed.from, IMPUTATION_DISCOUNT),
        };
        let n = self.n_senses.get(&id).copied().unwrap_or(0);
        let mut senses = Vec::with_capacity(n.into());
        for s in 0..n {
            if let Some(sense) = self.senses.get((id, s))? {
                senses.push(sense);
            }
        }
        Ok(ItemEmbedding {
            ety: self.ety.get(id)?,
            glosses: self.glosses.get(id)?,
            senses,
            discount,
        })
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        ab += x * y;
        aa += x * x;
        bb += y * y;
    }
    if aa == 0.0 || bb == 0.0 {
        return 0.0;
    }
    ab / (aa * bb).sqrt()
}

fn best_match<'a>(a: impl Iterator<Item = &'a Embedding> + Clone, b: &[Embedding]) -> f32 {
    a.flat_map(|x| b.iter().map(move |y| cosine(x, y)))
        .fold(0.0, f32::max)
}

/// How well a candidate fits a citation, by meaning: the best match between a
/// sense of the cited term as the citation glosses it (t=) or else of the
/// citing item, and a sense of the candidate (0.7); and how near the child's
/// etymology text is to the parent's glosses (0.3), since an etymology
/// describes the source it names. The weights were chosen on ambiguous
/// citations whose answer an id or a Descendants list gives.
pub(crate) fn citation_score(
    citer: &ItemEmbedding,
    candidate: &ItemEmbedding,
    citer_is_child: bool,
    gloss: Option<&Embedding>,
) -> f32 {
    let senses = match gloss {
        Some(gloss) => best_match(std::iter::once(gloss), &candidate.senses),
        None => best_match(citer.senses.iter(), &candidate.senses),
    };
    let (child, parent) = if citer_is_child {
        (citer, candidate)
    } else {
        (candidate, citer)
    };
    let etymology = match (&child.ety, &parent.glosses) {
        (Some(ety), Some(glosses)) => cosine(ety, glosses),
        _ => 0.0,
    };
    citer.discount.min(candidate.discount) * (0.7 * senses + 0.3 * etymology)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Whether, among two candidates, the right one scores higher for a
    // citation, with the default model.
    fn assert_right_disambiguation(
        name: &str,
        citer: (&str, &str, &str, &[&str]),
        citer_is_child: bool,
        candidates: (&str, &str),
        right: (&str, &[&str]),
        wrong: (&str, &[&str]),
    ) {
        let cache = std::env::temp_dir().join(format!("wety-embeddings-tests-{name}"));
        let config = Config {
            model_name: DEFAULT_MODEL.to_string(),
            model_revision: DEFAULT_MODEL_REVISION.to_string(),
            batch_size: 1,
            cache_path: cache.clone(),
        };
        let mut embeddings = Embeddings::new(&config).unwrap();
        let owned = |glosses: &[&str]| glosses.iter().map(|g| (*g).to_owned()).collect::<Vec<_>>();
        let ids = [0, 1, 2].map(ItemId::from);
        embeddings
            .add(ids[0], citer.0, citer.1, Some(citer.2), &owned(citer.3))
            .unwrap();
        for (id, (ety, glosses)) in ids[1..].iter().zip([right, wrong]) {
            embeddings
                .add(*id, candidates.0, candidates.1, Some(ety), &owned(glosses))
                .unwrap();
        }
        embeddings.flush().unwrap();
        let get = |id| {
            let item = Item::Imputed(wety_core::items::ImputedItem {
                ety_num: 1,
                lang: "en".parse().unwrap(),
                term: wety_core::langterm::Term::new(&mut wety_core::string_pool::StringPool::new(), "x"),
                romanization: None,
                from: id,
            });
            let mut embedding = embeddings.get(&item, id).unwrap();
            embedding.discount = 1.0;
            embedding
        };
        let (base, right, wrong) = (get(ids[0]), get(ids[1]), get(ids[2]));
        let right_score = citation_score(&base, &right, citer_is_child, None);
        let wrong_score = citation_score(&base, &wrong, citer_is_child, None);
        println!("{name}: {right_score} vs {wrong_score}");
        assert!(right_score > wrong_score);
        drop(embeddings);
        std::fs::remove_dir_all(cache).unwrap();
    }

    #[test]
    fn disambiguates_minþiją() {
        // Proto-Germanic *minþiją lists Old Norse minni among its descendants
        assert_right_disambiguation(
            "minþiją",
            ("Proto-Germanic", "minþiją", "From Proto-Indo-European *men- (“to think”).", &["memory, remembrance"]),
            false,
            ("Old Norse", "minni"),
            ("From Proto-Germanic *(ga)minþiją.", &["memory"]),
            ("From Proto-Germanic *minnizô, comparative of *lītilaz.", &["less, smaller: comparative degree of lítill"]),
        );
    }

    #[test]
    fn disambiguates_mone() {
        // English moon cites Middle English mone
        assert_right_disambiguation(
            "mone",
            (
                "English",
                "moon",
                "From Middle English mone, from Old English mōna (“moon”), from Proto-West Germanic *mānō, from Proto-Germanic *mēnô (“moon”), from Proto-Indo-European *mḗh₁n̥s (“moon, month”), probably from *meh₁- (“to measure”).",
                &["The Earth's only permanent natural satellite.", "A satellite of a planet."],
            ),
            true,
            ("Middle English", "mone"),
            (
                "From Old English mōna. The sense of the word as silver is the result of its astrological association with the planet.",
                &["The celestial body closest to the Earth, considered to be a planet in the Ptolemic system as well as the boundary between the Earth and the heavens; the Moon.", "A white, precious metal; silver."],
            ),
            ("From Old English mān, from Proto-West Germanic *mainu, from Proto-Germanic *mainō.", &["A lamentation", "A moan, complaint"]),
        );
    }

    #[test]
    fn scores() {
        let e = |v: &[f32]| v.to_vec();
        let item = |ety: Option<&[f32]>, glosses: &[f32], senses: &[&[f32]]| ItemEmbedding {
            ety: ety.map(e),
            glosses: Some(e(glosses)),
            senses: senses.iter().map(|s| e(s)).collect(),
            discount: 1.0,
        };
        // the child's etymology describes the parent's sense [0, 1]
        let child = item(Some(&[0.0, 1.0]), &[1.0, 0.0], &[&[1.0, 0.0]]);
        let right = item(None, &[0.0, 1.0], &[&[0.0, 1.0], &[1.0, 0.0]]);
        let wrong = item(None, &[1.0, 1.0], &[&[0.6, 0.8]]);
        assert!(citation_score(&child, &right, true, None) > citation_score(&child, &wrong, true, None));
        // a citation's gloss replaces the citing item's senses
        let gloss = e(&[0.6, 0.8]);
        assert!(citation_score(&child, &wrong, true, Some(&gloss)) > citation_score(&child, &right, true, Some(&gloss)) - 0.3);
        // seen from the parent, the roles of etymology and glosses swap
        assert!((citation_score(&right, &child, false, None) - citation_score(&child, &right, true, None)).abs() < 1e-6);
    }
}

#[cfg(test)]
mod parity {
    use super::*;

    // Writes embeddings of a few texts for comparison with sentence-transformers:
    // WETY_PARITY_OUT=path cargo test -p processor parity -- --ignored
    #[test]
    #[ignore = "downloads a model; for manual comparison"]
    fn write_embeddings() {
        let out = std::env::var("WETY_PARITY_OUT").unwrap();
        let model = std::env::var("WETY_PARITY_MODEL").unwrap();
        let revision = std::env::var("WETY_PARITY_REVISION").unwrap();
        let model = Model::new(model, revision).unwrap();
        let texts = [
            "A child or baby.",
            "barn, farm building, granary",
            "English bairn. From Middle English bern, barn, from Old English bearn (“child”).",
            "to bear, carry",
        ];
        let embeddings = model
            .encode(texts.iter().map(|t| (*t).to_owned()).collect())
            .unwrap()
            .to_vec2::<f32>()
            .unwrap();
        std::fs::write(out, serde_json::to_string(&embeddings).unwrap()).unwrap();
    }

    // Texts per second: WETY_PARITY_MODEL=... WETY_PARITY_REVISION=... cargo test
    // --release -p processor parity::throughput -- --ignored --nocapture
    #[test]
    #[ignore = "downloads a model; for manual measurement"]
    fn throughput() {
        let model = std::env::var("WETY_PARITY_MODEL").unwrap();
        let revision = std::env::var("WETY_PARITY_REVISION").unwrap();
        let model = Model::new(model, revision).unwrap();
        let texts: Vec<String> = (0..4000)
            .map(|i| format!("a word for the act or result of doing thing number {i}, as of a kind of work or play"))
            .collect();
        let start = std::time::Instant::now();
        model.encode(texts.clone()).unwrap();
        println!("{:.0} texts/s", texts.len() as f64 / start.elapsed().as_secs_f64());
    }
}
