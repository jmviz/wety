use crate::{raw_items::RawItem, wiktextract_json::WiktextractJson};

use std::{mem::take, rc::Rc};

use anyhow::Result;
use hashbrown::HashMap;
use rust_bert::pipelines::sentence_embeddings::{
    Embedding, SentenceEmbeddingsBuilder, SentenceEmbeddingsConfig, SentenceEmbeddingsModel,
    SentenceEmbeddingsModelType,
};
use simd_json::ValueAccess;

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
    pub(crate) fn add(&mut self, json_item: &WiktextractJson, item: &Rc<RawItem>) -> Result<()> {
        if json_item.get_str("word").is_some_and(|w| w == "min") {
            println!("hey");
        }
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
    pub(crate) fn get(&self, item: &Rc<RawItem>) -> ItemEmbedding {
        ItemEmbedding {
            ety: self.ety.map.get(&item.i),
            glosses: self.glosses.map.get(&item.i),
        }
    }
}

pub(crate) trait EmbeddingComparand<T> {
    fn cosine_similarity(self, other: T) -> f32;
}

impl EmbeddingComparand<&Embedding> for &Embedding {
    fn cosine_similarity(self, other: &Embedding) -> f32 {
        let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
        for (a, b) in self.iter().zip(other) {
            ab += a * b;
            aa += a * a;
            bb += b * b;
        }
        ab / (aa.sqrt() * bb.sqrt())
    }
}

impl EmbeddingComparand<Option<&Embedding>> for Option<&Embedding> {
    fn cosine_similarity(self, other: Option<&Embedding>) -> f32 {
        if let Some(this) = self
            && let Some(other) = other
        {
            return this.cosine_similarity(other);
        }
        0.0
    }
}

const ETY_WEIGHT: f32 = 0.5;
const GLOSSES_WEIGHT: f32 = 1.0 - ETY_WEIGHT;

impl EmbeddingComparand<ItemEmbedding<'_>> for ItemEmbedding<'_> {
    fn cosine_similarity(self, other: ItemEmbedding<'_>) -> f32 {
        let glosses_similarity = self.glosses.cosine_similarity(other.glosses);
        if let Some(self_ety) = self.ety
            && let Some(other_ety) = other.ety
            {
                let ety_similarity = self_ety.cosine_similarity(other_ety);
                return ETY_WEIGHT * ety_similarity + GLOSSES_WEIGHT * glosses_similarity
            }
        glosses_similarity
    }
}

const DISCOUNT: f32 = 0.95;
const ETY_QUALITY: f32 = 1.0;
const NO_ETY_QUALITY: f32 = 0.5;
const EMPTY_QUALITY: f32 = 0.0;

// for comparing an item with all its ancestors
impl EmbeddingComparand<ItemEmbedding<'_>> for &Vec<ItemEmbedding<'_>> {
    fn cosine_similarity(self, other: ItemEmbedding<'_>) -> f32 {
        if other.is_empty() {
            return 0.0;
        }
        let mut total_similarity = 0.0;
        let mut discount = 1.0;
        let mut total_weight = 0.0;
        for &ancestor in self.iter().rev() {
            let similarity = other.cosine_similarity(ancestor);
            let quality = if other.ety.is_some() && ancestor.ety.is_some() {
                ETY_QUALITY
            } else if !ancestor.is_empty() {
                NO_ETY_QUALITY
            } else {
                EMPTY_QUALITY
            };
            let weight = discount * quality;
            total_similarity += weight * similarity;
            total_weight += weight;
            discount *= DISCOUNT;
        }
        if total_weight > 0.0 {
            return total_similarity / total_weight;
        }
        0.0
    }
}
