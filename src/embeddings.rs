use std::{mem::take, rc::Rc};

use anyhow::Result;
use hashbrown::HashMap;
use rust_bert::pipelines::sentence_embeddings::{
    Embedding, SentenceEmbeddingsBuilder, SentenceEmbeddingsConfig, SentenceEmbeddingsModel,
    SentenceEmbeddingsModelType,
};
use simd_json::{value::borrowed::Value, ValueAccess};

use crate::Item;

#[derive(Clone, Copy)]
pub(crate) struct ItemEmbedding<'a> {
    ety: Option<&'a Embedding>,
    glosses: Option<&'a Embedding>,
}

impl ItemEmbedding<'_> {
    pub(crate) fn is_empty(&self) -> bool {
        self.ety.is_none() && self.glosses.is_none()
    }
}

const ETY_BATCH_SIZE: usize = 800;
const GLOSSES_BATCH_SIZE: usize = 800;

struct EmbeddingBatch {
    items: Vec<usize>,
    texts: Vec<String>,
    max_size: usize,
    model: Rc<SentenceEmbeddingsModel>,
}

impl EmbeddingBatch {
    fn new(model: &Rc<SentenceEmbeddingsModel>, size: usize) -> Self {
        Self {
            items: Vec::with_capacity(size),
            texts: Vec::with_capacity(size),
            max_size: size,
            model: Rc::clone(model),
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
            let items = take(&mut self.items);
            let embeddings = self.model.encode(&self.texts)?;
            self.clear();
            return Ok(Some((items, embeddings)));
        }
        Ok(None)
    }
    fn flush(&mut self) -> Result<Option<(Vec<usize>, Vec<Embedding>)>> {
        if self.len() > 0 {
            let items = take(&mut self.items);
            let embeddings = self.model.encode(&self.texts)?;
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
    fn new(model: &Rc<SentenceEmbeddingsModel>, batch_size: usize) -> Self {
        Self {
            batch: EmbeddingBatch::new(model, batch_size),
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
    fn flush(&mut self) -> Result<()> {
        if let Some((items, embeddings)) = self.batch.flush()? {
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
        println!("Using {maybe_cuda}CUDA backend for embeddings...");
        Ok(Self {
            ety: EmbeddingMap::new(&model, ETY_BATCH_SIZE),
            glosses: EmbeddingMap::new(&model, GLOSSES_BATCH_SIZE),
        })
    }
    pub(crate) fn add(&mut self, json_item: &Value, item: &Rc<Item>) -> Result<()> {
        if !self.ety.map.contains_key(&item.i)
            && let Some(ety_text) = json_item.get_str("etymology_text")
            && !ety_text.is_empty()
            {
                self.ety.update(item.i, ety_text.to_string())?;
            }
        if !self.glosses.map.contains_key(&item.i) {
            let mut glosses_text = String::new();
            if let Some(senses) = json_item.get_array("senses") {
                for sense in senses {
                    if let Some(gloss) = sense
                        .get_array("glosses")
                        .and_then(|glosses| glosses.get(0))
                        .and_then(|gloss| gloss.as_str())
                    {
                        glosses_text.push_str(gloss);
                        glosses_text.push(' ');
                    }
                }
            }
            if !glosses_text.is_empty() {
                self.glosses.update(item.i, glosses_text.to_string())?;
            }
        }
        Ok(())
    }
    pub(crate) fn flush(&mut self) -> Result<()> {
        self.ety.flush()?;
        self.glosses.flush()?;
        Ok(())
    }
    pub(crate) fn get(&self, item: &Rc<Item>) -> ItemEmbedding {
        ItemEmbedding {
            ety: self.ety.map.get(&item.i),
            glosses: self.glosses.map.get(&item.i),
        }
    }
}

pub(crate) trait EmbeddingComparand {
    fn cosine_similarity(self, other: Self) -> f32;
}

impl EmbeddingComparand for &Embedding {
    fn cosine_similarity(self, other: Self) -> f32 {
        let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
        for (a, b) in self.iter().zip(other) {
            ab += a * b;
            aa += a * a;
            bb += b * b;
        }
        ab / (aa.sqrt() * bb.sqrt())
    }
}

impl EmbeddingComparand for Option<&Embedding> {
    fn cosine_similarity(self, other: Self) -> f32 {
        if let Some(this) = self
            && let Some(other) = other
        {
            return this.cosine_similarity(other);
        }
        0.0
    }
}

impl EmbeddingComparand for &ItemEmbedding<'_> {
    fn cosine_similarity(self, other: Self) -> f32 {
        let glosses_similarity = self.glosses.cosine_similarity(other.glosses);
        if let Some(self_ety) = self.ety
            && let Some(other_ety) = other.ety
            {
                let ety_similarity = self_ety.cosine_similarity(other_ety);
                return (ety_similarity + glosses_similarity) / 2.0
            }
        glosses_similarity
    }
}

impl EmbeddingComparand for Option<&ItemEmbedding<'_>> {
    fn cosine_similarity(self, other: Self) -> f32 {
        if let Some(self_ie) = self
            && let Some(other_ie) = other
        {
            return self_ie.cosine_similarity(other_ie);
        }
        0.0
    }
}
