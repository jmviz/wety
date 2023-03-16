use std::{mem::take, rc::Rc, time::Instant};

use anyhow::Result;
use hashbrown::HashMap;
use indicatif::HumanDuration;
use rust_bert::pipelines::sentence_embeddings::{
    Embedding, SentenceEmbeddingsBuilder, SentenceEmbeddingsConfig, SentenceEmbeddingsModel,
    SentenceEmbeddingsModelType,
};
use simd_json::{value::borrowed::Value, ValueAccess};

const ETY_BATCH_SIZE: usize = 800;
const GLOSSES_BATCH_SIZE: usize = 800;

struct EmbeddingBatch {
    items: Vec<usize>,
    texts: Vec<String>,
    max_size: usize,
    model: Rc<SentenceEmbeddingsModel>,
    name: String,
    n_batches: usize,
}

impl EmbeddingBatch {
    fn new(model: &Rc<SentenceEmbeddingsModel>, size: usize, name: &str) -> Self {
        Self {
            items: Vec::with_capacity(size),
            texts: Vec::with_capacity(size),
            max_size: size,
            model: Rc::clone(model),
            name: name.into(),
            n_batches: 0,
        }
    }
    fn len(&self) -> usize {
        assert!(self.items.len() == self.texts.len());
        self.items.len()
    }
    fn add(&mut self, item: usize, text: String) {
        self.items.push(item);
        self.texts.push(text);
    }
    fn clear(&mut self) {
        self.items.clear();
        self.texts.clear();
    }
    fn update(
        &mut self,
        item: usize,
        text: String,
    ) -> Result<Option<(Vec<usize>, Vec<Embedding>)>> {
        self.add(item, text);
        if self.len() >= self.max_size {
            self.n_batches += 1;
            let items = take(&mut self.items);
            let t = Instant::now();
            let embeddings = self.model.encode(&self.texts)?;
            println!(
                "Generated {} {} embeddings in total. Last batch of {} took {}.",
                self.n_batches * self.max_size,
                &self.name,
                self.max_size,
                HumanDuration(t.elapsed())
            );
            self.clear();
            return Ok(Some((items, embeddings)));
        }
        Ok(None)
    }
}

struct EmbeddingMap {
    batch: EmbeddingBatch,
    map: HashMap<usize, Embedding>,
}

impl EmbeddingMap {
    fn new(model: &Rc<SentenceEmbeddingsModel>, batch_size: usize, name: &str) -> Self {
        Self {
            batch: EmbeddingBatch::new(model, batch_size, name),
            map: HashMap::new(),
        }
    }
    fn update(&mut self, item: usize, text: String) -> Result<()> {
        if let Some((items, embeddings)) = self.batch.update(item, text)? {
            for (&item, embedding) in items.iter().zip(embeddings) {
                self.map.insert(item, embedding);
            }
        }
        Ok(())
    }
}

pub(crate) struct Embeddings {
    ety: EmbeddingMap,
    glosses: EmbeddingMap,
}

impl Embeddings {
    pub(crate) fn new() -> Result<Self> {
        // https://www.sbert.net/docs/pretrained_models.html
        // https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2
        let model = Rc::from(
            SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL6V2)
                .create_model()?,
        );
        let config = SentenceEmbeddingsConfig::from(SentenceEmbeddingsModelType::AllMiniLmL6V2);
        let maybe_cuda = if config.device.is_cuda() { "" } else { "non-" };
        println!("...Using {maybe_cuda}CUDA backend...");
        Ok(Self {
            ety: EmbeddingMap::new(&model, ETY_BATCH_SIZE, "ety"),
            glosses: EmbeddingMap::new(&model, GLOSSES_BATCH_SIZE, "glosses"),
        })
    }
    pub(crate) fn add(&mut self, json_item: &Value, i: usize) -> Result<()> {
        if let Some(ety_text) = json_item.get_str("etymology_text")
            && !ety_text.is_empty() {
                self.ety.update(i, ety_text.to_string())?;
            }
        let mut glosses_text = String::new();
        if let Some(senses) = json_item.get_array("senses") {
            for sense in senses {
                if let Some(gloss) = sense
                    .get_array("glosses")
                    .and_then(|glosses| glosses.get(0))
                    .and_then(|gloss| gloss.as_str())
                {
                    glosses_text.push_str(gloss);
                }
            }
        }
        if !glosses_text.is_empty() {
            self.glosses.update(i, glosses_text.to_string())?;
        }
        Ok(())
    }
}

trait EmbeddingExt {
    fn cosine_similarity(&self, other: &Embedding) -> f32 {}
}
