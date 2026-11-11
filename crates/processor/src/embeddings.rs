//! Text embeddings of items, for choosing among homographs (see resolve.rs).
//!
//! Each item gets embeddings of each of its senses' glosses, of its glosses
//! joined, and of its etymology text; a citation may also give a gloss of the
//! cited term (t=). Embeddings are cached by the hash of their text, in a `Store`
//! under a namespace that is the fingerprint of what the model computes
//! (`namespace`).

#[allow(clippy::large_enum_variant)]
mod model;

use crate::{HashMap, HashSet};
use model::Model;
use wety_core::items::{Item, ItemId};

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    mem,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use memmap2::Mmap;
use xxhash_rust::xxh3::xxh3_64;

/// For an `imputed_item` embedding, we use the embedding for
/// `imputed_item.from`, weighted by this discount factor
pub(crate) const IMPUTATION_DISCOUNT: f32 = 0.5;

// senses per item to embed, and characters of etymology text
const MAX_SENSES: usize = 12;
const MAX_ETY_CHARS: usize = 600;

/// An item's embeddings, borrowed from the store.
pub(crate) struct ItemEmbedding {
    ety: Option<Vector>,
    glosses: Option<Vector>,
    senses: Vec<Vector>,
    discount: f32,
}

/// An embedding as scores read it, copied out of the store, with its squared norm
/// computed once for however many vectors it is compared with.
pub(crate) struct Vector {
    values: Box<[i8]>,
    norm2: i32,
}

impl Vector {
    fn new(stored: &[i8]) -> Self {
        Self {
            values: stored.into(),
            norm2: dot(stored, stored),
        }
    }
}

/// An embedding as the store keeps it: each value as a multiple of the vector's
/// largest magnitude / 127, rounded. Cosine similarity does not depend on a vector's
/// scale, so the scale is not kept; the sums it is computed from are exact integers.
fn quantize(embedding: &[f32]) -> Vec<i8> {
    let max = embedding.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    if max == 0.0 {
        return vec![0; embedding.len()];
    }
    #[allow(clippy::cast_possible_truncation)]
    embedding
        .iter()
        .map(|x| (x * 127.0 / max).round().clamp(-127.0, 127.0) as i8)
        .collect()
}

fn dot(a: &[i8], b: &[i8]) -> i32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| i32::from(x) * i32::from(y))
        .sum()
}

pub type TextHash = u64;

fn text_hash(text: &str) -> TextHash {
    xxh3_64(text.as_bytes())
}

/// Embeddings by the hash of their text, on disk in a directory: `keys` (a u64 per
/// entry), `vectors` (`dim` i8 per entry, see `quantize`; read in place through a
/// memory map) and
/// `dim`, all little-endian; entries are only appended. Which keys it holds is kept
/// in memory, so a warm cache is checked without reading the disk. One process at
/// a time may open it (`lock`).
pub struct Store {
    dir: PathBuf,
    dim: usize,
    index: HashMap<TextHash, u32>,
    keys: File,
    vectors: File,
    len: usize,
    map: Option<Mmap>,
    _lock: File,
}

impl Store {
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read or created, or if another
    /// process has the store open.
    pub fn open(dir: &Path) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let lock = File::create(dir.join("lock"))?;
        if lock.try_lock().is_err() {
            bail!(
                "another process is using the embedding cache {}",
                dir.display()
            );
        }
        let file = |name: &str| {
            OpenOptions::new()
                .read(true)
                .append(true)
                .create(true)
                .open(dir.join(name))
                .with_context(|| dir.join(name).display().to_string())
        };
        let (mut keys, vectors) = (file("keys")?, file("vectors")?);
        let dim = match fs::read_to_string(dir.join("dim")) {
            Ok(dim) => dim.trim().parse()?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
            Err(e) => return Err(e.into()),
        };
        // An interrupted append leaves at most a partial entry: drop it.
        let entry_bytes = dim.max(1) as u64;
        let len = if dim == 0 {
            0
        } else {
            (keys.metadata()?.len() / 8).min(vectors.metadata()?.len() / entry_bytes)
        };
        keys.set_len(len * 8)?;
        vectors.set_len(len * entry_bytes * u64::from(dim > 0))?;
        let mut bytes = vec![];
        keys.read_to_end(&mut bytes)?;
        let mut index = HashMap::default();
        index.reserve(bytes.len() / 8);
        for (i, key) in (0u32..).zip(bytes.as_chunks::<8>().0) {
            index.entry(u64::from_le_bytes(*key)).or_insert(i);
        }
        let mut store = Self {
            dir: dir.to_owned(),
            dim,
            index,
            keys,
            vectors,
            len: usize::try_from(len)?,
            map: None,
            _lock: lock,
        };
        store.remap()?;
        Ok(store)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn contains(&self, key: TextHash) -> bool {
        self.index.contains_key(&key)
    }

    /// Add embeddings not yet held, quantized (`quantize`): a quarter of the memory
    /// and reading of f32. They can be read after `remap`.
    ///
    /// # Errors
    ///
    /// Returns an error if an embedding's length is not the store's, or on a
    /// failed write.
    pub fn append<'a>(
        &mut self,
        entries: impl IntoIterator<Item = (TextHash, &'a [f32])>,
    ) -> Result<()> {
        let (mut keys, mut vectors) = (vec![], vec![]);
        let mut added = vec![];
        for (key, embedding) in entries {
            if self.dim == 0 {
                ensure!(!embedding.is_empty(), "an empty embedding");
                self.dim = embedding.len();
                fs::write(self.dir.join("dim"), self.dim.to_string())?;
            }
            ensure!(
                embedding.len() == self.dim,
                "an embedding of {} values in a store of {}",
                embedding.len(),
                self.dim
            );
            if self.index.contains_key(&key) || added.iter().any(|&(k, _)| k == key) {
                continue;
            }
            keys.extend_from_slice(&key.to_le_bytes());
            vectors.extend_from_slice(bytemuck::cast_slice(&quantize(embedding)));
            added.push((key, u32::try_from(self.len + added.len())?));
        }
        // vectors first: keys name only entries whose vectors are written
        self.vectors.write_all(&vectors)?;
        self.keys.write_all(&keys)?;
        self.len += added.len();
        self.index.extend(added);
        Ok(())
    }

    /// Make what was appended durable and readable.
    ///
    /// # Errors
    ///
    /// Returns an error if the files cannot be synced or mapped.
    pub fn remap(&mut self) -> Result<()> {
        self.vectors.sync_data()?;
        self.keys.sync_data()?;
        // SAFETY: the vectors file is only appended to, by this process (the lock),
        // and the map covers only what was written before it was made.
        self.map = (self.len > 0)
            .then(|| unsafe { Mmap::map(&self.vectors) })
            .transpose()?;
        // Resolution reads the vectors at random: read the file in now, in order,
        // so that the system reads ahead in large reads, rather than a page at a
        // time on demand. (madvise(WILLNEED) alone did not, on macOS.)
        if let Some(map) = &self.map {
            map.advise(memmap2::Advice::Sequential)?;
            let mut sum = 0u8;
            for page in map.chunks(4096) {
                sum = sum.wrapping_add(page[0]);
            }
            std::hint::black_box(sum);
            map.advise(memmap2::Advice::Random)?;
        }
        Ok(())
    }

    pub fn get(&self, key: TextHash) -> Option<&[i8]> {
        let i = *self.index.get(&key)? as usize;
        let map = self.map.as_deref().expect("remap after appending");
        let bytes = map
            .get(i * self.dim..(i + 1) * self.dim)
            .expect("remap after appending");
        Some(bytemuck::cast_slice(bytes))
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

pub struct Config {
    pub model_name: String,
    pub model_revision: String,
    pub batch_size: usize,
    pub cache_path: PathBuf,
}

// Texts of several lengths whose embeddings name the cache.
const FINGERPRINT_TEXTS: [&str; 4] = [
    "A child or baby.",
    "barn, farm building, granary",
    "English bairn. From Middle English bern, barn, from Old English bearn (“child”).",
    "to bear, carry",
];

/// The cache namespace of a model: the model and its revision, and the exact values
/// it gives a few texts. Whatever changes what the model code computes (candle,
/// tokenizers, the device, the encoding or pooling in model.rs) starts an empty
/// cache; anything that does not keeps it.
fn namespace(model: &Model, config: &Config) -> Result<u64> {
    let texts = FINGERPRINT_TEXTS.map(str::to_owned).to_vec();
    let embeddings = model.encode(texts)?.flatten_all()?.to_vec1::<f32>()?;
    // the store's format (i8) is part of it
    let mut bytes = format!("{}\0{}\0i8\0", config.model_name, config.model_revision).into_bytes();
    bytes.extend_from_slice(bytemuck::cast_slice(&embeddings));
    Ok(xxh3_64(&bytes))
}

pub(crate) struct Embeddings {
    batch_size: usize,
    model: Model,
    store: Store,
    pending: Vec<(TextHash, String)>,
    pending_keys: HashSet<TextHash>,
    ety: HashMap<ItemId, TextHash>,
    glosses: HashMap<ItemId, TextHash>,
    senses: HashMap<(ItemId, u8), TextHash>,
    n_senses: HashMap<ItemId, u8>,
    texts: HashSet<TextHash>,
}

impl Embeddings {
    pub(crate) fn new(config: &Config) -> Result<Self> {
        ensure!(
            config.model_revision.len() == 40
                && config.model_revision.bytes().all(|b| b.is_ascii_hexdigit()),
            "Embedding model revision must be an immutable 40-character commit"
        );
        let model = Model::new(config.model_name.clone(), config.model_revision.clone())?;
        let namespace = namespace(&model, config)?;
        let store = Store::open(&config.cache_path.join(format!("{namespace:016x}")))?;
        Ok(Self {
            batch_size: config.batch_size,
            model,
            store,
            pending: vec![],
            pending_keys: HashSet::default(),
            ety: HashMap::default(),
            glosses: HashMap::default(),
            senses: HashMap::default(),
            n_senses: HashMap::default(),
            texts: HashSet::default(),
        })
    }

    // The key of a text, queued to be embedded unless the store has it.
    fn queue(&mut self, text: String) -> Result<TextHash> {
        let key = text_hash(&text);
        if !self.store.contains(key) && self.pending_keys.insert(key) {
            self.pending.push((key, text));
            if self.pending.len() >= self.batch_size {
                self.embed_pending()?;
            }
        }
        Ok(key)
    }

    fn embed_pending(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let (keys, texts): (Vec<_>, Vec<_>) = mem::take(&mut self.pending).into_iter().unzip();
        let embeddings = self.model.encode(texts)?.to_vec2::<f32>()?;
        self.store
            .append(keys.into_iter().zip(embeddings.iter().map(Vec::as_slice)))?;
        self.pending_keys.clear();
        Ok(())
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
            let key = self.queue(format!("{item_lang} {item_term}. {ety_text}"))?;
            self.ety.insert(item_id, key);
        }
        let glosses = &glosses[..glosses.len().min(MAX_SENSES)];
        if !glosses.is_empty() {
            let key = self.queue(glosses.join(" "))?;
            self.glosses.insert(item_id, key);
            for (n, gloss) in (0u8..).zip(glosses) {
                let key = self.queue(gloss.clone())?;
                self.senses.insert((item_id, n), key);
            }
            #[allow(clippy::cast_possible_truncation)]
            self.n_senses.insert(item_id, glosses.len() as u8);
        }
        Ok(())
    }

    /// Embed a text that is not an item's, e.g. a gloss a citation gives.
    pub(crate) fn add_text(&mut self, text: &str) -> Result<()> {
        if !self.texts.contains(&text_hash(text)) {
            let key = self.queue(text.to_owned())?;
            self.texts.insert(key);
        }
        Ok(())
    }

    /// Embed what is queued; after this, embeddings can be read.
    pub(crate) fn flush(&mut self) -> Result<()> {
        self.embed_pending()?;
        self.store.remap()
    }

    pub(crate) fn text(&self, text: &str) -> Option<Vector> {
        let key = text_hash(text);
        self.texts
            .contains(&key)
            .then(|| self.store.get(key))
            .flatten()
            .map(Vector::new)
    }

    pub(crate) fn get(&self, item: &Item, item_id: ItemId) -> ItemEmbedding {
        let (id, discount) = match item {
            Item::Real(_) => (item_id, 1.0),
            Item::Imputed(imputed) => (imputed.from, IMPUTATION_DISCOUNT),
        };
        let get =
            |key: Option<&TextHash>| key.and_then(|&key| self.store.get(key)).map(Vector::new);
        let n = self.n_senses.get(&id).copied().unwrap_or(0);
        ItemEmbedding {
            ety: get(self.ety.get(&id)),
            glosses: get(self.glosses.get(&id)),
            senses: (0..n)
                .filter_map(|s| get(self.senses.get(&(id, s))))
                .collect(),
            discount,
        }
    }
}

fn cosine(a: &Vector, b: &Vector) -> f32 {
    if a.norm2 == 0 || b.norm2 == 0 {
        return 0.0;
    }
    let ab = f64::from(dot(&a.values, &b.values));
    #[allow(clippy::cast_possible_truncation)]
    let cosine = (ab / (f64::from(a.norm2) * f64::from(b.norm2)).sqrt()) as f32;
    cosine
}

fn best_match<'a>(a: impl Iterator<Item = &'a Vector> + Clone, b: &[Vector]) -> f32 {
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
    gloss: Option<&Vector>,
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
                term: wety_core::langterm::Term::new(
                    &mut wety_core::string_pool::StringPool::new(),
                    "x",
                ),
                romanization: None,
                from: id,
            });
            let mut embedding = embeddings.get(&item, id);
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

    fn values(v: Option<&[i8]>) -> Option<Vec<i8>> {
        v.map(<[i8]>::to_vec)
    }

    #[test]
    fn quantizes_to_the_largest_magnitude() {
        assert_eq!(quantize(&[1.0, -2.0, 0.5]), vec![64, -127, 32]);
        assert_eq!(quantize(&[0.0, 0.0]), vec![0, 0]);
    }

    #[test]
    fn store_appends_and_recovers() {
        let dir = std::env::temp_dir().join("wety-embeddings-tests-store");
        let _ = std::fs::remove_dir_all(&dir);
        {
            let mut store = Store::open(&dir).unwrap();
            assert!(Store::open(&dir).is_err(), "locked");
            store
                .append([
                    (1, &[1.0, 2.0][..]),
                    (2, &[3.0, 4.0][..]),
                    (1, &[9.0, 9.0][..]),
                ])
                .unwrap();
            store.remap().unwrap();
            assert_eq!(values(store.get(1)), Some(vec![64, 127]));
            assert_eq!(values(store.get(2)), Some(vec![95, 127]));
            assert_eq!(store.get(3), None);
            assert!(store.append([(3, &[1.0][..])]).is_err());
        }
        // a partial entry, as an interrupted append leaves it, is dropped
        OpenOptions::new()
            .append(true)
            .open(dir.join("vectors"))
            .unwrap()
            .write_all(&[0; 1])
            .unwrap();
        let mut store = Store::open(&dir).unwrap();
        assert_eq!(store.len(), 2);
        store.append([(3, &[5.0, 6.0][..])]).unwrap();
        store.remap().unwrap();
        assert_eq!(values(store.get(3)), Some(vec![106, 127]));
        assert_eq!(values(store.get(1)), Some(vec![64, 127]));
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn disambiguates_minþiją() {
        // Proto-Germanic *minþiją lists Old Norse minni among its descendants
        assert_right_disambiguation(
            "minþiją",
            (
                "Proto-Germanic",
                "minþiją",
                "From Proto-Indo-European *men- (“to think”).",
                &["memory, remembrance"],
            ),
            false,
            ("Old Norse", "minni"),
            ("From Proto-Germanic *(ga)minþiją.", &["memory"]),
            (
                "From Proto-Germanic *minnizô, comparative of *lītilaz.",
                &["less, smaller: comparative degree of lítill"],
            ),
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
                &[
                    "The Earth's only permanent natural satellite.",
                    "A satellite of a planet.",
                ],
            ),
            true,
            ("Middle English", "mone"),
            (
                "From Old English mōna. The sense of the word as silver is the result of its astrological association with the planet.",
                &[
                    "The celestial body closest to the Earth, considered to be a planet in the Ptolemic system as well as the boundary between the Earth and the heavens; the Moon.",
                    "A white, precious metal; silver.",
                ],
            ),
            (
                "From Old English mān, from Proto-West Germanic *mainu, from Proto-Germanic *mainō.",
                &["A lamentation", "A moan, complaint"],
            ),
        );
    }

    #[test]
    fn scores() {
        let h = |v: &[f32]| Vector::new(&quantize(v));
        let item = |ety: Option<&[f32]>, glosses: &[f32], senses: &[&[f32]]| ItemEmbedding {
            ety: ety.map(h),
            glosses: Some(h(glosses)),
            senses: senses.iter().map(|s| h(s)).collect(),
            discount: 1.0,
        };
        // the child's etymology describes the parent's sense [0, 1]
        let child = item(Some(&[0.0, 1.0]), &[1.0, 0.0], &[&[1.0, 0.0]]);
        let right = item(None, &[0.0, 1.0], &[&[0.0, 1.0], &[1.0, 0.0]]);
        let wrong = item(None, &[1.0, 1.0], &[&[0.6, 0.8]]);
        assert!(
            citation_score(&child, &right, true, None) > citation_score(&child, &wrong, true, None)
        );
        // a citation's gloss replaces the citing item's senses
        let gloss = h(&[0.6, 0.8]);
        assert!(
            citation_score(&child, &wrong, true, Some(&gloss))
                > citation_score(&child, &right, true, Some(&gloss)) - 0.3
        );
        // seen from the parent, the roles of etymology and glosses swap
        assert!(
            (citation_score(&right, &child, false, None)
                - citation_score(&child, &right, true, None))
            .abs()
                < 1e-6
        );
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

    // The cache namespace of the default model with this build:
    // cargo test --release -p processor parity::namespace -- --ignored --nocapture
    #[test]
    #[ignore = "loads the model; for manual comparison"]
    fn namespace() {
        let config = Config {
            model_name: DEFAULT_MODEL.to_string(),
            model_revision: DEFAULT_MODEL_REVISION.to_string(),
            batch_size: DEFAULT_BATCH_SIZE,
            cache_path: PathBuf::new(),
        };
        let model = Model::new(config.model_name.clone(), config.model_revision.clone()).unwrap();
        println!(
            "namespace {:016x}",
            super::namespace(&model, &config).unwrap()
        );
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
        println!(
            "{:.0} texts/s",
            texts.len() as f64 / start.elapsed().as_secs_f64()
        );
    }
}
