use crate::{items::ItemId, wiktextract_json::WiktextractJson, HashMap};

use std::{mem, path::PathBuf, rc::Rc};

use anyhow::Result;
use clap::ValueEnum;
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsConfig, SentenceEmbeddingsModel,
    SentenceEmbeddingsModelType,
};
use simd_json::ValueAccess;
use sled::{Batch, Db, IVec};
use xxhash_rust::xxh3::xxh3_64;

// rust_bert::pipelines::sentence_embeddings::Embedding is also Vec<f32>, but we
// use our own type so that if we update the package and for some reason it
// changes to say Vec<f16/f64>, then
//      embeddings: Vec<Embedding> = model.encode(...)
// will be a compilation error instead of quiet unexpected behavior, as we need
// to be sure of the elements being f32 for the caching (see below).
type Embedding = Vec<f32>;

pub(crate) struct ItemEmbedding {
    ety: Option<Embedding>,
    glosses: Option<Embedding>,
}

impl ItemEmbedding {
    pub(crate) fn is_empty(&self) -> bool {
        self.ety.is_none() && self.glosses.is_none()
    }
}

type TextHash = u64;

trait ToByteSlice {
    fn to_bytes(&self) -> [u8; 8];
}

impl ToByteSlice for TextHash {
    fn to_bytes(&self) -> [u8; 8] {
        self.to_be_bytes()
    }
}

trait ToByteVec {
    fn to_bytes(&self) -> Vec<u8>;
}

impl ToByteVec for Embedding {
    fn to_bytes(&self) -> Vec<u8> {
        self.iter().flat_map(|e| e.to_be_bytes()).collect()
    }
}

trait ToEmbedding {
    fn to_embedding(&self) -> Embedding;
}

impl ToEmbedding for &[u8] {
    fn to_embedding(&self) -> Embedding {
        // the 4 here assumes Embedding elements are f32
        self.array_chunks::<4>()
            .map(|&bytes| f32::from_be_bytes(bytes))
            .collect()
    }
}

impl ToEmbedding for IVec {
    fn to_embedding(&self) -> Embedding {
        self.as_ref().to_embedding()
    }
}

struct EmbeddingBatch {
    max_size: usize,
    model: Rc<SentenceEmbeddingsModel>,
    cache: Rc<Db>,
    items: Vec<ItemId>,
    texts: Vec<String>,
    text_hashes: Vec<TextHash>,
}

impl EmbeddingBatch {
    fn new(model: &Rc<SentenceEmbeddingsModel>, size: usize, cache: &Rc<Db>) -> Self {
        Self {
            items: Vec::with_capacity(size),
            texts: Vec::with_capacity(size),
            text_hashes: Vec::with_capacity(size),
            max_size: size,
            model: Rc::clone(model),
            cache: Rc::clone(cache),
        }
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn add(&mut self, item: ItemId, text: String, text_hash: TextHash) {
        self.items.push(item);
        self.texts.push(text);
        self.text_hashes.push(text_hash);
    }

    fn clear(&mut self) {
        self.items.clear();
        self.texts.clear();
        self.text_hashes.clear();
    }

    fn update(
        &mut self,
        item: ItemId,
        text: String,
        text_hash: TextHash,
    ) -> Result<Option<(Vec<ItemId>, Vec<TextHash>)>> {
        self.add(item, text, text_hash);
        if self.len() >= self.max_size {
            return Ok(Some(self.encode_and_cache()?));
        }
        Ok(None)
    }

    fn flush(&mut self) -> Result<Option<(Vec<ItemId>, Vec<TextHash>)>> {
        if self.len() > 0 {
            return Ok(Some(self.encode_and_cache()?));
        }
        Ok(None)
    }

    fn encode_and_cache(&mut self) -> Result<(Vec<ItemId>, Vec<TextHash>)> {
        let items = mem::take(&mut self.items);
        let text_hashes = mem::take(&mut self.text_hashes);
        let embeddings: Vec<Embedding> = self.model.encode(&self.texts)?;
        self.cache(&text_hashes, &embeddings)?;
        self.clear();
        Ok((items, text_hashes))
    }

    fn cache(&self, text_hashes: &[TextHash], embeddings: &[Embedding]) -> Result<()> {
        let mut batch = Batch::default();
        for (text_hash, embedding) in text_hashes.iter().zip(embeddings.iter()) {
            batch.insert(&text_hash.to_bytes(), embedding.to_bytes());
        }
        self.cache.apply_batch(batch)?;
        Ok(())
    }
}

struct EmbeddingsMap {
    batch: EmbeddingBatch,
    map: HashMap<ItemId, TextHash>,
    cache: Rc<Db>,
}

impl EmbeddingsMap {
    fn new(model: &Rc<SentenceEmbeddingsModel>, batch_size: usize, cache: &Rc<Db>) -> Self {
        Self {
            batch: EmbeddingBatch::new(model, batch_size, cache),
            map: HashMap::default(),
            cache: Rc::clone(cache),
        }
    }
    fn update(&mut self, item: ItemId, text: String) -> Result<()> {
        let text_hash = xxh3_64(text.as_bytes());
        if self.cache.contains_key(text_hash.to_bytes())? {
            self.map.insert(item, text_hash);
            return Ok(());
        }
        if let Some((items, text_hashes)) = self.batch.update(item, text, text_hash)? {
            for (&item, text_hash) in items.iter().zip(text_hashes) {
                self.map.insert(item, text_hash);
            }
        }
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        if let Some((items, text_hashes)) = self.batch.flush()? {
            for (&item, text_hash) in items.iter().zip(text_hashes) {
                self.map.insert(item, text_hash);
            }
        }
        Ok(())
    }
    fn get(&self, item: ItemId) -> Result<Option<Embedding>> {
        if let Some(text_hash) = self.map.get(&item)
            && let Some(embedding_bytes) = self.cache.get(text_hash.to_bytes())?
        {
            return Ok(Some(embedding_bytes.to_embedding()));
        }
        Ok(None)
    }
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
    pub cache_path: PathBuf,
}

pub(crate) struct Embeddings {
    ety: EmbeddingsMap,
    glosses: EmbeddingsMap,
    cache: Rc<Db>,
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
        let cache = Rc::from(sled::open(&config.cache_path)?);
        Ok(Self {
            ety: EmbeddingsMap::new(&model, config.batch_size, &cache),
            glosses: EmbeddingsMap::new(&model, config.batch_size, &cache),
            cache,
        })
    }
    pub(crate) fn add(
        &mut self,
        json_item: &WiktextractJson,
        item_lang: &str,
        item_term: &str,
        item_id: ItemId,
    ) -> Result<()> {
        if !self.ety.map.contains_key(&item_id)
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
            self.ety.update(item_id, ety_text)?;
        }
        if !self.glosses.map.contains_key(&item_id) {
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
                self.glosses.update(item_id, glosses_text.to_string())?;
            }
        }
        Ok(())
    }
    pub(crate) fn flush(&mut self) -> Result<()> {
        self.ety.flush()?;
        self.glosses.flush()?;
        self.cache.flush()?;
        Ok(())
    }
    pub(crate) fn get(&self, item: ItemId) -> Result<ItemEmbedding> {
        Ok(ItemEmbedding {
            ety: self.ety.get(item)?,
            glosses: self.glosses.get(item)?,
        })
    }
}

pub(crate) trait EmbeddingComparand<T> {
    fn cosine_similarity(&self, other: &T) -> f32;
}

impl EmbeddingComparand<Embedding> for Embedding {
    fn cosine_similarity(&self, other: &Embedding) -> f32 {
        let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
        for (a, b) in self.iter().zip(other) {
            ab += a * b;
            aa += a * a;
            bb += b * b;
        }
        ab / (aa.sqrt() * bb.sqrt())
    }
}

impl EmbeddingComparand<Option<Embedding>> for Option<Embedding> {
    fn cosine_similarity(&self, other: &Option<Embedding>) -> f32 {
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

impl EmbeddingComparand<ItemEmbedding> for ItemEmbedding {
    fn cosine_similarity(&self, other: &ItemEmbedding) -> f32 {
        let glosses_similarity = self.glosses.cosine_similarity(&other.glosses);
        if let Some(self_ety) = &self.ety
            && let Some(other_ety) = &other.ety
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
impl EmbeddingComparand<ItemEmbedding> for Vec<ItemEmbedding> {
    fn cosine_similarity(&self, other: &ItemEmbedding) -> f32 {
        if other.is_empty() {
            return 0.0;
        }
        let mut total_similarity = 0.0;
        let mut discount = 1.0;
        let mut total_weight = 0.0;
        for ancestor in self.iter().rev() {
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
    use simd_json::json;
    use std::path::Path;

    fn delete_cache(path: &Path) {
        std::fs::remove_dir_all(path).unwrap();
    }

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

    fn embeddings(cache_path: &Path) -> Embeddings {
        let config = EmbeddingsConfig {
            model: DEFAULT_MODEL,
            batch_size: 1,
            progress_update_interval: 1,
            cache_path: cache_path.to_path_buf(),
        };
        Embeddings::new(&config).unwrap()
    }

    fn feq(f0: f32, f1: f32) -> bool {
        (f0 - f1).abs() <= f32::EPSILON
    }

    #[test]
    fn cosine_similarity_identical() {
        let cache = PathBuf::from("tmp-embeddings-tests-identical");
        let mut embeddings = embeddings(&cache);
        let json = json("test", "test test");
        let lang = "test_lang";
        let term = "test_term";
        embeddings.add(&json, lang, term, 0).unwrap();
        embeddings.add(&json, lang, term, 1).unwrap();
        let item_embedding0 = embeddings.get(0).unwrap();
        assert!(item_embedding0.ety.is_some());
        assert!(item_embedding0.glosses.is_some());
        let item_embedding1 = embeddings.get(1).unwrap();
        assert!(item_embedding1.ety.is_some());
        assert!(item_embedding1.glosses.is_some());
        assert_eq!(item_embedding0.ety, item_embedding1.ety);
        assert_eq!(item_embedding0.glosses, item_embedding1.glosses);
        let similarity0 = item_embedding0.cosine_similarity(&item_embedding1);
        println!("{similarity0}");
        assert!(feq(similarity0, 1.0));
        let similarity1 = item_embedding1.cosine_similarity(&item_embedding0);
        assert!(feq(similarity0, similarity1));
        delete_cache(&cache);
    }

    #[allow(clippy::too_many_arguments)]
    fn assert_right_disambiguation(
        embeddings: &mut Embeddings,
        base_lang: &str,
        base_term: &str,
        base_json: &WiktextractJson,
        candidates_lang: &str,
        candidates_term: &str,
        right_json: &WiktextractJson,
        wrong_json: &WiktextractJson,
    ) {
        let parent = 0;
        let right = 1;
        let wrong = 2;
        embeddings
            .add(base_json, base_lang, base_term, parent)
            .unwrap();
        embeddings
            .add(right_json, candidates_lang, candidates_term, right)
            .unwrap();
        embeddings
            .add(wrong_json, candidates_lang, candidates_term, wrong)
            .unwrap();
        let base_embedding = embeddings.get(parent).unwrap();
        let right_embedding = embeddings.get(right).unwrap();
        let wrong_embedding = embeddings.get(wrong).unwrap();
        let ety_right_similarity = base_embedding.ety.cosine_similarity(&right_embedding.ety);
        let ety_wrong_similarity = base_embedding.ety.cosine_similarity(&wrong_embedding.ety);
        println!("ety similarities: {ety_right_similarity}, {ety_wrong_similarity}");
        // assert!(ety_right_similarity > ety_wrong_similarity);
        let glosses_right_similarity = base_embedding
            .glosses
            .cosine_similarity(&right_embedding.glosses);
        let glosses_wrong_similarity = base_embedding
            .glosses
            .cosine_similarity(&wrong_embedding.glosses);
        println!("glosses similarities: {glosses_right_similarity}, {glosses_wrong_similarity}");
        // assert!(glosses_right_similarity > glosses_wrong_similarity);
        let right_similarity = base_embedding.cosine_similarity(&right_embedding);
        let wrong_similarity = base_embedding.cosine_similarity(&wrong_embedding);
        println!("similarities: {right_similarity}, {wrong_similarity}");
        assert!(right_similarity > wrong_similarity);
    }

    #[test]
    fn cosine_similarity_minþiją() {
        let cache = PathBuf::from("tmp-embeddings-tests-minþiją");
        let mut embeddings = embeddings(&cache);
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
            &mut embeddings,
            base_lang,
            base_term,
            &base_json,
            candidates_lang,
            candidates_term,
            &right_json,
            &wrong_json,
        );
        delete_cache(&cache);
    }

    #[test]
    fn cosine_similarity_mone() {
        let cache = PathBuf::from("tmp-embeddings-tests-mone");
        let mut embeddings = embeddings(&cache);
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
            &mut embeddings,
            base_lang,
            base_term,
            &base_json,
            candidates_lang,
            candidates_term,
            &right_json,
            &wrong_json,
        );
        delete_cache(&cache);
    }

    #[test]
    fn xxhash_equality() {
        let a = xxh3_64("test".as_bytes());
        let b = xxh3_64("test".as_bytes());
        assert_eq!(a, b);
    }
}
