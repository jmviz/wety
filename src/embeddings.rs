use crate::{raw_items::RawItem, wiktextract_json::WiktextractJson};

use std::{mem::take, rc::Rc};

use anyhow::Result;
use clap::ValueEnum;
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

#[allow(clippy::module_name_repetitions)]
#[derive(ValueEnum, Clone)]
#[clap(rename_all = "PascalCase")]
pub enum EmbeddingsModel {
    AllMiniLmL6V2,
    DistiluseBaseMultilingualCased,
    BertBaseNliMeanTokens,
    AllMiniLmL12V2,
    AllDistilrobertaV1,
    ParaphraseAlbertSmallV2,
    SentenceT5Base,
}

pub const DEFAULT_MODEL: EmbeddingsModel = EmbeddingsModel::AllMiniLmL6V2;
pub const DEFAULT_BATCH_SIZE: usize = 800;
pub const DEFAULT_PROGRESS_UPDATE_INTERVAL: usize = DEFAULT_BATCH_SIZE * 10;

impl EmbeddingsModel {
    fn kind(&self) -> SentenceEmbeddingsModelType {
        match self {
            EmbeddingsModel::AllMiniLmL6V2 => SentenceEmbeddingsModelType::AllMiniLmL6V2,
            EmbeddingsModel::DistiluseBaseMultilingualCased => {
                SentenceEmbeddingsModelType::DistiluseBaseMultilingualCased
            }
            EmbeddingsModel::BertBaseNliMeanTokens => {
                SentenceEmbeddingsModelType::BertBaseNliMeanTokens
            }
            EmbeddingsModel::AllMiniLmL12V2 => SentenceEmbeddingsModelType::AllMiniLmL12V2,
            EmbeddingsModel::AllDistilrobertaV1 => SentenceEmbeddingsModelType::AllDistilrobertaV1,
            EmbeddingsModel::ParaphraseAlbertSmallV2 => {
                SentenceEmbeddingsModelType::ParaphraseAlbertSmallV2
            }
            EmbeddingsModel::SentenceT5Base => SentenceEmbeddingsModelType::SentenceT5Base,
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct EmbeddingsConfig {
    pub model: EmbeddingsModel,
    pub batch_size: usize,
    pub progress_update_interval: usize,
}

impl Embeddings {
    pub(crate) fn new(config: &EmbeddingsConfig) -> Result<Self> {
        // https://www.sbert.net/docs/pretrained_models.html
        // https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2
        let model =
            Rc::from(SentenceEmbeddingsBuilder::remote(config.model.kind()).create_model()?);
        let se_config = SentenceEmbeddingsConfig::from(config.model.kind());
        let maybe_cuda = if se_config.device.is_cuda() {
            ""
        } else {
            "non-"
        };
        println!("Using {maybe_cuda}CUDA backend for embeddings...");
        Ok(Self {
            ety: EmbeddingMap::new(&model, config.batch_size),
            glosses: EmbeddingMap::new(&model, config.batch_size),
        })
    }
    pub(crate) fn add(
        &mut self,
        json_item: &WiktextractJson,
        item_lang: &str,
        item_term: &str,
        item_i: usize,
    ) -> Result<()> {
        if !self.ety.map.contains_key(&item_i)
            && let Some(ety_text) = json_item.get_str("etymology_text")
            && !ety_text.is_empty()
        {
            // We prepend the lang name and term to the ety text. Consider a
            // veridical ancestor chain of a>b>c0, where c0 has a within-lang
            // homograph c1. Suppose that the ety texts are as follows: a: "",
            // b: "From a.", c0: "From b.", c1: "From z." If we just compared
            // ety texts, then c0 and c1 would have comparable similarities to
            // b, because neither c0 nor c1's ety text share's anything from
            // b's. Now consider the prepended versions: a: "a", b: "b. From
            // a.", c0: "c0. From b.", c1: "c1. From z." Now c0 shares "b" with
            // b's ety text, while c1 still shares nothing with b's ety text. So
            // c0's similarity to b will be higher than c1's, as desired.
            let ety_text = format!("{item_lang} {item_term}. {ety_text}");
            println!("{ety_text}");
            self.ety.update(item_i, ety_text)?;
        }
        if !self.glosses.map.contains_key(&item_i) {
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
                self.glosses.update(item_i, glosses_text.to_string())?;
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

const ETY_WEIGHT: f32 = 0.4;
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

// The farther you get down a chain of ancestry, the more an item's meaning (and
// hence glosses) is likely to diverge from the remoter ancestors'. This
// discount factor thus assigns ancestors progressively lesser weights the
// farther you get up the chain from the item in question.
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::string_pool::Symbol;
    use simd_json::json;
    use string_interner::Symbol as SymbolTrait;

    fn json<'a>(ety: &str, gloss: &str) -> WiktextractJson<'a> {
        json!({
            "etymology_text": ety,
            "senses": [
                {
                    "glosses": [
                        gloss
                    ]
                }
            ]
        })
        .into()
    }

    fn item(i: usize) -> Rc<RawItem> {
        Rc::from(RawItem {
            line: None,
            is_imputed: false,
            is_reconstructed: false,
            i,
            lang: 0,
            term: Symbol::try_from_usize(0).unwrap(),
            page_title: None,
            ety_num: None,
            pos: None,
            gloss: None,
            gloss_num: 0,
            raw_etymology: None,
            raw_root: None,
            raw_descendants: None,
        })
    }

    fn embeddings() -> Embeddings {
        let config = EmbeddingsConfig {
            model: DEFAULT_MODEL,
            batch_size: 1,
            progress_update_interval: 1,
        };
        Embeddings::new(&config).unwrap()
    }

    fn feq(f0: f32, f1: f32) -> bool {
        (f0 - f1).abs() <= f32::EPSILON
    }

    #[test]
    fn cosine_similarity_identical() {
        let mut embeddings = embeddings();
        let json = json("test", "test test");
        let lang = "test_lang";
        let term = "test_term";
        let item0 = item(0);
        let item1 = item(1);
        embeddings.add(&json, lang, term, item0.i).unwrap();
        embeddings.add(&json, lang, term, item1.i).unwrap();
        let item_embedding0 = embeddings.get(&item0);
        assert!(item_embedding0.ety.is_some());
        assert!(item_embedding0.glosses.is_some());
        let item_embedding1 = embeddings.get(&item1);
        assert!(item_embedding1.ety.is_some());
        assert!(item_embedding1.glosses.is_some());
        assert_eq!(item_embedding0.ety.unwrap(), item_embedding1.ety.unwrap());
        assert_eq!(
            item_embedding0.glosses.unwrap(),
            item_embedding1.glosses.unwrap()
        );
        let similarity0 = item_embedding0.cosine_similarity(item_embedding1);
        println!("{similarity0}");
        assert!(feq(similarity0, 1.0));
        let similarity1 = item_embedding1.cosine_similarity(item_embedding0);
        assert!(feq(similarity0, similarity1));
    }

    fn assert_right_disambiguation(
        base_lang: &str,
        base_term: &str,
        base_json: &WiktextractJson,
        candidates_lang: &str,
        candidates_term: &str,
        right_json: &WiktextractJson,
        wrong_json: &WiktextractJson,
    ) {
        let mut embeddings = embeddings();
        let parent = item(0);
        let right = item(1);
        let wrong = item(2);
        embeddings
            .add(base_json, base_lang, base_term, parent.i)
            .unwrap();
        embeddings
            .add(right_json, candidates_lang, candidates_term, right.i)
            .unwrap();
        embeddings
            .add(wrong_json, candidates_lang, candidates_term, wrong.i)
            .unwrap();
        let base_embedding = embeddings.get(&parent);
        let right_embedding = embeddings.get(&right);
        let wrong_embedding = embeddings.get(&wrong);
        let ety_right_similarity = base_embedding.ety.cosine_similarity(right_embedding.ety);
        let ety_wrong_similarity = base_embedding.ety.cosine_similarity(wrong_embedding.ety);
        println!("ety similarities: {ety_right_similarity}, {ety_wrong_similarity}");
        // assert!(ety_right_similarity > ety_wrong_similarity);
        let glosses_right_similarity = base_embedding
            .glosses
            .cosine_similarity(right_embedding.glosses);
        let glosses_wrong_similarity = base_embedding
            .glosses
            .cosine_similarity(wrong_embedding.glosses);
        println!("glosses similarities: {glosses_right_similarity}, {glosses_wrong_similarity}");
        // assert!(glosses_right_similarity > glosses_wrong_similarity);
        let right_similarity = base_embedding.cosine_similarity(right_embedding);
        let wrong_similarity = base_embedding.cosine_similarity(wrong_embedding);
        println!("similarities: {right_similarity}, {wrong_similarity}");
        assert!(right_similarity > wrong_similarity);
    }

    #[test]
    fn cosine_similarity_minþiją() {
        let base_lang = "Proto-Germanic";
        let base_term = "minþiją";
        let base_json = json(
            "From Proto-Indo-European *men- (“to think”).",
            "memory, remembrance",
        );
        let candidates_lang = "Old Norse";
        let candidates_term = "minni";
        let right_json = json("From Proto-Germanic *(ga)minþiją.", "memory");
        let wrong_json = json(
            "From Proto-Germanic *minnizô, comparative of *lītilaz.",
            "less, smaller: comparative degree of lítill",
        );
        assert_right_disambiguation(
            base_lang,
            base_term,
            &base_json,
            candidates_lang,
            candidates_term,
            &right_json,
            &wrong_json,
        );
    }

    #[test]
    fn cosine_similarity_mone() {
        let base_lang = "English";
        let base_term = "moon";
        let base_json = json(
            "From Middle English mone, from Old English mōna (“moon”), from Proto-West Germanic *mānō, from Proto-Germanic *mēnô (“moon”), from Proto-Indo-European *mḗh₁n̥s (“moon, month”), probably from *meh₁- (“to measure”).\ncognates and doublets\nCognate with Scots mone, mune, muin (“moon”), North Frisian muun (“moon”), West Frisian moanne (“moon”), Dutch maan (“moon”), German Mond (“moon”), Danish måne (“moon”), Norwegian Bokmål måne (“moon”), Norwegian Nynorsk måne (“moon”), Swedish måne (“moon”), Icelandic máni (“moon”), Latin mēnsis (“month”). See also month, a related term within Indo-European.",
            "Alternative letter-case form of Moon (“the Earth's only permanent natural satellite”).",
        );
        let candidates_lang = "Middle English";
        let candidates_term = "mone";
        let right_json = json(
            "From Old English mōna. The sense of the word as silver is the result of its astrological association with the planet.",
            "The celestial body closest to the Earth, considered to be a planet in the Ptolemic system as well as the boundary between the Earth and the heavens; the Moon. A white, precious metal; silver."
        );
        let wrong_json = json(
            "From Old English mān, from Proto-West Germanic *mainu, from Proto-Germanic *mainō.",
            "A lamentation A moan, complaint",
        );
        assert_right_disambiguation(
            base_lang,
            base_term,
            &base_json,
            candidates_lang,
            candidates_term,
            &right_json,
            &wrong_json,
        );
    }
}
