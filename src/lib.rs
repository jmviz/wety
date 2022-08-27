//! WIP attempt to digest etymologies from wiktextract data

mod etymology_templates;
mod lang;
mod pos;
mod turtle;

use crate::{
    etymology_templates::{
        ABBREV_TYPE_TEMPLATES, COMPOUND_TYPE_TEMPLATES, DERIVED_TYPE_TEMPLATES, MODE,
    },
    lang::{LANG_CODE2NAME, LANG_ETYCODE2CODE, LANG_NAME2CODE},
    pos::POS,
    turtle::write_turtle_file,
};

use std::{
    convert::TryFrom,
    fs::{remove_dir_all, File},
    hash::Hasher,
    io::BufReader,
    path::Path,
    rc::Rc,
    time::Instant,
};

use ahash::AHasher;
use anyhow::{anyhow, Ok, Result};
use bytelines::ByteLines;
use flate2::read::GzDecoder;
use hashbrown::{HashMap, HashSet};
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use oxigraph::{io::GraphFormat::Turtle, model::GraphNameRef::DefaultGraph, store::Store};
use phf::{phf_set, OrderedMap, OrderedSet, Set};
use simd_json::{to_borrowed_value, value::borrowed::Value, ValueAccess};
use string_interner::{backend::StringBackend, symbol::SymbolU32, StringInterner};

// cf. https://github.com/tatuylonen/wiktextract/blob/master/wiktwords
static IGNORED_REDIRECTS: Set<&'static str> = phf_set! {
    "Index:", "Help:", "MediaWiki:", "Citations:", "Concordance:", "Rhymes:",
    "Thread:", "Summary:", "File:", "Transwiki:", "Category:", "Appendix:",
    "Wiktionary:", "Thesaurus:", "Module:", "Template:"
};

// models the basic info from a wiktionary etymology template
#[derive(Hash, Eq, PartialEq, Debug)]
struct RawEtyNode {
    source_terms: Box<[SymbolU32]>, // e.g. "re-", "do"
    source_langs: Box<[usize]>,     // e.g. "en", "en"
    mode: usize,                    // e.g. "prefix"
    head: u8,                       // e.g. 1 (the index of "do")
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct RawRoot {
    term: SymbolU32,
    lang: usize,
    sense_id: Option<SymbolU32>,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct Item {
    is_imputed: bool,
    i: usize,                 // the i-th item seem, used as id for RDF
    term: SymbolU32,          // e.g. "bank"
    lang: usize,              // e.g "en", i.e. the wiktextract lang_code
    ety_num: u8,              // the nth ety encountered for this term-lang combo
    pos: usize,               // e.g. "noun"
    gloss: Option<SymbolU32>, // e.g. "An institution where one can place and borrow money...
    gloss_num: u8,            // the nth gloss encountered for this term-lang-ety-pos combo
    raw_ety_nodes: Option<Box<[RawEtyNode]>>,
    raw_root: Option<RawRoot>,
}

impl Item {
    fn new_imputed(i: usize, pos: usize, lang: usize, term: SymbolU32) -> Self {
        Self {
            is_imputed: true,
            i,
            term,
            lang,
            pos,
            ety_num: 0,
            gloss_num: 0,
            gloss: None,
            raw_ety_nodes: None,
            raw_root: None,
        }
    }
}

type GlossMap = HashMap<Option<SymbolU32>, Rc<Item>>;
type PosMap = HashMap<usize, GlossMap>;
type EtyMap = HashMap<Option<u64>, (u8, PosMap)>;
type LangMap = HashMap<usize, EtyMap>;
type TermMap = HashMap<SymbolU32, LangMap>;

#[derive(Default)]
struct Items {
    term_map: TermMap,
    n: usize,
    redirects: Redirects,
}

impl Items {
    fn add(&mut self, ety_text_hash: Option<u64>, mut item: Item) -> Result<()> {
        // check if the item's term has been seen before
        if !self.term_map.contains_key(&item.term) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let mut lang_map = LangMap::new();
            let (pos, lang, term) = (item.pos, item.lang, item.term);
            gloss_map.insert(item.gloss, Rc::from(item));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text_hash, (0, pos_map));
            lang_map.insert(lang, ety_map);
            self.term_map.insert(term, lang_map);
            self.n += 1;
            return Ok(());
        }
        // since term has been seen before, there must be at least one lang for it
        // check if item's lang has been seen before
        let lang_map: &mut LangMap = self.term_map.get_mut(&item.term).unwrap();
        if !lang_map.contains_key(&item.lang) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let (pos, lang) = (item.pos, item.lang);
            gloss_map.insert(item.gloss, Rc::from(item));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text_hash, (0, pos_map));
            lang_map.insert(lang, ety_map);
            self.n += 1;
            return Ok(());
        }
        // since lang has been seen before, there must be at least one ety (possibly None)
        // check if this ety has been seen in this lang before
        let ety_map: &mut EtyMap = lang_map.get_mut(&item.lang).unwrap();
        if !ety_map.contains_key(&ety_text_hash) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let pos = item.pos;
            let ety_num = u8::try_from(ety_map.len())?;
            item.ety_num = ety_num;
            gloss_map.insert(item.gloss, Rc::from(item));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text_hash, (ety_num, pos_map));
            self.n += 1;
            return Ok(());
        }
        // since ety has been seen before, there must be at least one pos
        // check if this pos has been seen for this ety before
        let (ety_num, pos_map): &mut (u8, PosMap) = ety_map.get_mut(&ety_text_hash).unwrap();
        if !pos_map.contains_key(&item.pos) {
            let mut gloss_map = GlossMap::new();
            let pos = item.pos;
            item.ety_num = *ety_num;
            gloss_map.insert(item.gloss, Rc::from(item));
            pos_map.insert(pos, gloss_map);
            self.n += 1;
            return Ok(());
        }
        // since pos has been seen before, there must be at least one gloss (possibly None)
        let gloss_map: &mut GlossMap = pos_map.get_mut(&item.pos).unwrap();
        if !gloss_map.contains_key(&item.gloss) {
            item.gloss_num = u8::try_from(gloss_map.len())?;
            item.ety_num = *ety_num;
            gloss_map.insert(item.gloss, Rc::from(item));
            self.n += 1;
            return Ok(());
        }
        Ok(())
    }

    // Get all items that have this lang and term
    fn get(&self, lang: usize, term: SymbolU32) -> Result<Option<Vec<&Rc<Item>>>> {
        // If lang is an etymology-only language, we will not find any entries
        // for it in Items lang map, since such a language definitionally does
        // not have any entries itself. So we look for the actual lang that the
        // ety lang is associated with.
        let lang = LANG_ETYCODE2CODE
            .get(LANG_CODE2NAME.get_expected_index_key(lang)?)
            .and_then(|code| LANG_CODE2NAME.get_index(code))
            .unwrap_or(lang);
        // Then we also check if there is a redirect for this lang term combo.
        let (lang, term) = self.redirects.get(lang, term)?;
        Ok(self
            .term_map
            .get(&term)
            .and_then(|lang_map| lang_map.get(&lang))
            // If an ety_map is found, that means there is at least one item to
            // collect after this nested iteration. See logic in Items::add()
            // for why. Therefore, this function will always return either a
            // non-empty Vec or None.
            .map(|ety_map| {
                ety_map
                    .values()
                    .flat_map(|(_, pos_map)| pos_map.values().flat_map(hashbrown::HashMap::values))
                    .collect()
            }))
    }

    // For now we'll just take the first node. But cf. notes.md.
    /// Only to be called once all json items have been processed into items.
    fn process_item_raw_ety_nodes(
        &self,
        string_pool: &StringPool,
        sources: &mut Sources,
        item: &Rc<Item>,
    ) -> Result<()> {
        if item.raw_ety_nodes.is_none() {
            return Ok(()); // don't add anything to sources if no valid raw ety nodes
        }
        let mut current_item = Rc::clone(item); // for tracking possibly imputed items
        let mut next_item = Rc::clone(item); // for tracking possibly imputed items
        for node in item.raw_ety_nodes.as_ref().unwrap().iter() {
            let sense = Sense::new(string_pool, &current_item);
            let mut source_items = Vec::with_capacity(node.source_terms.len());
            let mut has_new_imputation = false;
            for (&source_lang, &source_term) in
                node.source_langs.iter().zip(node.source_terms.iter())
            {
                if let Some(candidate_source_items) = self.get(source_lang, source_term)? {
                    // There exists at least one item for this lang term combo.
                    // We have to do a word sense disambiguation in case there
                    // are multiple items.
                    let source_item = candidate_source_items
                        .iter()
                        .max_by_key(|candidate| {
                            let candidate_sense = Sense::new(string_pool, candidate);
                            sense.lesk_score(&candidate_sense)
                        })
                        .unwrap(); // Item::get() never returns an empty Vec

                    source_items.push(Rc::clone(source_item));
                } else if let Some(imputed_source_item) = sources
                    .imputed_items
                    .term_map
                    .get(&source_term)
                    .and_then(|lang_map| lang_map.get(&source_lang))
                {
                    // We have already imputed an item that corresponds to this term.
                    source_items.push(Rc::clone(imputed_source_item));
                } else if node.source_terms.len() == 1 {
                    // This is an unseen term, and it is in a non-compound-type template.
                    // We will impute an item for this term, and use this new imputed
                    // item as the item for the next template in the outer loop.
                    has_new_imputation = true;
                    let i = self.n + sources.imputed_items.n;
                    // We assume the imputed item has the same pos as the current_item.
                    // (How often is this not the case?)
                    let imputed_source_item = Rc::from(Item::new_imputed(
                        i,
                        current_item.pos,
                        source_lang,
                        source_term,
                    ));
                    sources.imputed_items.add(&imputed_source_item);
                    source_items.push(Rc::clone(&imputed_source_item));
                    next_item = Rc::clone(&imputed_source_item);
                } else {
                    // This is a term of a compound-type template without a
                    // link, and for which a corresponding imputed item has not
                    // yet been created. We won't bother trying to do convoluted
                    // imputations for such cases at the moment. So we stop
                    // processing templates here.
                    return Ok(());
                }
            }
            let source = Source {
                items: source_items.into_boxed_slice(),
                mode: node.mode,
                head: node.head,
            };
            sources.add(&current_item, source)?;
            // We keep processing templates until we hit the first one with no
            // imputation required.
            if !has_new_imputation {
                return Ok(());
            }
            current_item = Rc::clone(&next_item);
        }
        Ok(())
    }

    fn generate_sources(&self, string_pool: &StringPool) -> Result<Sources> {
        let pb = ProgressBar::new(u64::try_from(self.n)?);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({per_sec}, {eta})")?
            .progress_chars("#>-"));
        let mut sources = Sources::default();
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for (_, pos_map) in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            self.process_item_raw_ety_nodes(string_pool, &mut sources, item)?;
                            pb.inc(1);
                        }
                    }
                }
            }
        }
        pb.finish();
        Ok(sources)
    }
}

type ImputedLangMap = HashMap<usize, Rc<Item>>;
type ImputedTermMap = HashMap<SymbolU32, ImputedLangMap>;

// Quite often an etymology section on wiktionary will have multiple valid
// templates that don't actually link to anything (because the term has no page,
// or doesn't have the relevant page section), before an eventual valid template
// that does. See e.g. https://en.wiktionary.org/wiki/arsenic#English. The first
// two templates linking to Middle English and Middle French terms are both
// valid for our purposes, and the pages exist, but the language sections they
// link to do not exist. Therefore, both of these terms will not correspond to a
// findable item, and so the current procedure will give an ety of None. Instead
// we can go through the templates until we find the template linking Latin
// https://en.wiktionary.org/wiki/arsenicum#Latin, where the page and section
// both exist.
#[derive(Default)]
struct ImputedItems {
    term_map: ImputedTermMap,
    n: usize,
}

impl ImputedItems {
    fn add(&mut self, item: &Rc<Item>) {
        // check if the item's term has been seen before
        if !self.term_map.contains_key(&item.term) {
            let mut lang_map = ImputedLangMap::new();
            let term = item.term;
            lang_map.insert(item.lang, Rc::clone(item));
            self.term_map.insert(term, lang_map);
            self.n += 1;
            return;
        }
        // since term has been seen before, there must be at least one lang for it
        // check if item's lang has been seen before
        let lang_map: &mut ImputedLangMap = self.term_map.get_mut(&item.term).unwrap();
        if !lang_map.contains_key(&item.lang) {
            lang_map.insert(item.lang, Rc::clone(item));
            self.n += 1;
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct Source {
    items: Box<[Rc<Item>]>,
    mode: usize,
    head: u8,
}

// wrapper around a Hashmap linking items with their immediate etymological source
#[derive(Default)]
struct Sources {
    item_map: HashMap<Rc<Item>, Source>,
    imputed_items: ImputedItems,
}

impl Sources {
    fn add(&mut self, item: &Rc<Item>, source: Source) -> Result<()> {
        self.item_map
            .try_insert(Rc::clone(item), source)
            .and(std::result::Result::Ok(()))
            .map_err(|_| anyhow!("Tried inserting duplicate item:\n{:#?}", item))
    }

    fn get(&self, item: &Item) -> Option<&Source> {
        self.item_map.get(item)
    }
}

#[derive(Default)]
struct StringPool {
    pool: StringInterner<StringBackend<SymbolU32>>,
}

impl StringPool {
    // SymbolU32 is Copy so we don't need to do &SymbolU32
    fn resolve(&self, symbol: SymbolU32) -> &str {
        self.pool
            .resolve(symbol)
            .expect("Could not resolve string pool symbol")
    }
    fn get_or_intern(&mut self, s: &str) -> SymbolU32 {
        self.pool.get_or_intern(s)
    }
}

// Always short-lived struct used for sense disambiguation.
struct Sense {
    gloss: HashSet<String>,
}

impl Sense {
    fn new(string_pool: &StringPool, item: &Rc<Item>) -> Sense {
        let mut gloss = HashSet::new();
        let gloss_str = item.gloss.map_or("", |g| string_pool.resolve(g));
        for word in remove_punctuation(gloss_str).split_whitespace() {
            gloss.insert(word.to_string());
        }
        Sense { gloss }
    }
    // https://en.wikipedia.org/wiki/Lesk_algorithm
    fn lesk_score(&self, other: &Sense) -> usize {
        self.gloss.intersection(&other.gloss).count()
    }
}

// convenience extension trait methods for reading from json
trait ValueExt {
    fn get_expected_str(&self, key: &str) -> Result<&str>;
    fn get_optional_str(&self, key: &str) -> Option<&str>;
    fn get_expected_object(&self, key: &str) -> Result<&Value>;
}

impl ValueExt for Value<'_> {
    fn get_expected_str(&self, key: &str) -> Result<&str> {
        self.get_str(key)
            .ok_or_else(|| anyhow!("failed parsing '{key}' key in json:\n{self}"))
            .and_then(|s| {
                (!s.is_empty())
                    .then(|| s)
                    .ok_or_else(|| anyhow!("empty str value for '{key}' key in json:\n{self}"))
            })
    }
    fn get_optional_str(&self, key: &str) -> Option<&str> {
        self.get_str(key).and_then(|s| (!s.is_empty()).then(|| s))
    }
    fn get_expected_object(&self, key: &str) -> Result<&Value> {
        self.get(key)
            .ok_or_else(|| anyhow!("failed parsing '{key}' key in json:\n{self}"))
    }
}

// convenience extension trait methods for dealing with ordered maps and sets
trait OrderedMapExt {
    fn get_index_key(&self, index: usize) -> Option<&str>;
    fn get_expected_index_key(&self, index: usize) -> Result<&str>;
    fn get_index_value(&self, index: usize) -> Option<&str>;
    fn get_expected_index_value(&self, index: usize) -> Result<&str>;
    fn get_expected_index(&self, key: &str) -> Result<usize>;
}

impl OrderedMapExt for OrderedMap<&str, &str> {
    fn get_index_key(&self, index: usize) -> Option<&str> {
        self.index(index).map(|(&key, _)| key)
    }
    fn get_expected_index_key(&self, index: usize) -> Result<&str> {
        self.get_index_key(index)
            .ok_or_else(|| anyhow!("The index {index} does not exist."))
    }
    fn get_index_value(&self, index: usize) -> Option<&str> {
        self.index(index).map(|(_, &value)| value)
    }
    fn get_expected_index_value(&self, index: usize) -> Result<&str> {
        self.get_index_value(index)
            .ok_or_else(|| anyhow!("The index {index} does not exist."))
    }
    fn get_expected_index(&self, key: &str) -> Result<usize> {
        self.get_index(key)
            .ok_or_else(|| anyhow!("The key '{key}' does not exist."))
    }
}

trait OrderedSetExt {
    fn get_expected_index_key(&self, index: usize) -> Result<&str>;
    fn get_expected_index(&self, key: &str) -> Result<usize>;
}

impl OrderedSetExt for OrderedSet<&str> {
    fn get_expected_index_key(&self, index: usize) -> Result<&str> {
        self.index(index)
            .copied()
            .ok_or_else(|| anyhow!("The index {index} does not exist."))
    }
    fn get_expected_index(&self, key: &str) -> Result<usize> {
        self.get_index(key)
            .ok_or_else(|| anyhow!("The key '{key}' does not exist."))
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct ReconstructionTitle {
    language: usize,
    term: SymbolU32,
}

#[derive(Default)]
struct Redirects {
    reconstruction: HashMap<ReconstructionTitle, ReconstructionTitle>,
    regular: HashMap<SymbolU32, SymbolU32>,
}

impl Redirects {
    // If a redirect page exists for given lang + term combo, get the redirect.
    // If not, just return back the original lang + term.
    fn get(&self, lang: usize, term: SymbolU32) -> Result<(usize, SymbolU32)> {
        if let Some(language) = LANG_CODE2NAME.get_index_value(lang) {
            let language_index = LANG_NAME2CODE.get_expected_index(language)?;
            if let Some(redirect) = self.reconstruction.get(&ReconstructionTitle {
                language: language_index,
                term,
            }) {
                if let Some(redirect_lang) = LANG_NAME2CODE.get_index_value(redirect.language) {
                    let redirect_lang_index = LANG_CODE2NAME.get_expected_index(redirect_lang)?;
                    return Ok((redirect_lang_index, redirect.term));
                }
            } else if let Some(&redirect_term) = self.regular.get(&term) {
                return Ok((lang, redirect_term));
            }
        }
        Ok((lang, term))
    }
}

#[derive(Default)]
struct RawDataProcessor {
    string_pool: StringPool,
}

impl RawDataProcessor {
    fn process_derived_type_json_template(
        &mut self,
        args: &Value,
        mode: &str,
        lang: &str,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != lang {
            return Ok(None);
        }
        if let Some(source_lang) = LANG_CODE2NAME.get_index(args.get_expected_str("2")?) {
            if let Some(source_term) = args.get_optional_str("3") {
                if source_term.is_empty() || source_term == "-" {
                    return Ok(None);
                }
                let source_term = clean_ety_term(source_term);
                return Ok(Some(RawEtyNode {
                    source_terms: Box::new([self.string_pool.get_or_intern(source_term)]),
                    source_langs: Box::new([source_lang]),
                    mode: MODE.get_expected_index(mode)?,
                    head: 0,
                }));
            }
        }
        Ok(None)
    }

    fn process_abbrev_type_json_template(
        &mut self,
        args: &Value,
        mode: &str,
        lang: &str,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != lang {
            return Ok(None);
        }
        if let Some(source_term) = args.get_optional_str("2") {
            if source_term.is_empty() || source_term == "-" {
                return Ok(None);
            }
            let source_term = clean_ety_term(source_term);
            return Ok(Some(RawEtyNode {
                source_terms: Box::new([self.string_pool.get_or_intern(source_term)]),
                source_langs: Box::new([LANG_CODE2NAME.get_expected_index(lang)?]),
                mode: MODE.get_expected_index(mode)?,
                head: 0,
            }));
        }
        Ok(None)
    }

    fn process_prefix_json_template(
        &mut self,
        args: &Value,
        lang: &str,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != lang {
            return Ok(None);
        }
        if let Some(source_prefix) = args.get_optional_str("2") {
            if source_prefix.is_empty() || source_prefix == "-" {
                return Ok(None);
            }
            if let Some(source_term) = args.get_optional_str("3") {
                if source_term.is_empty() || source_term == "-" {
                    return Ok(None);
                }
                let source_prefix = clean_ety_term(source_prefix).to_string();
                let source_prefix = format!("{}-", source_prefix);
                let source_prefix = self.string_pool.get_or_intern(source_prefix.as_str());
                let source_term = clean_ety_term(source_term);
                let source_term = self.string_pool.get_or_intern(source_term);
                return Ok(Some(RawEtyNode {
                    source_terms: Box::new([source_prefix, source_term]),
                    source_langs: Box::new([LANG_CODE2NAME.get_expected_index(lang)?; 2]),
                    mode: MODE.get_expected_index("prefix")?,
                    head: 1,
                }));
            }
        }
        Ok(None)
    }

    fn process_suffix_json_template(
        &mut self,
        args: &Value,
        lang: &str,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != lang {
            return Ok(None);
        }
        if let Some(source_term) = args.get_optional_str("2") {
            if source_term.is_empty() || source_term == "-" {
                return Ok(None);
            }
            if let Some(source_suffix) = args.get_optional_str("3") {
                if source_suffix.is_empty() || source_suffix == "-" {
                    return Ok(None);
                }
                let source_term = clean_ety_term(source_term);
                let source_term = self.string_pool.get_or_intern(source_term);
                let source_suffix = clean_ety_term(source_suffix).to_string();
                let source_suffix = format!("-{}", source_suffix);
                let source_suffix = self.string_pool.get_or_intern(source_suffix.as_str());
                return Ok(Some(RawEtyNode {
                    source_terms: Box::new([source_term, source_suffix]),
                    source_langs: Box::new([LANG_CODE2NAME.get_expected_index(lang)?; 2]),
                    mode: MODE.get_expected_index("suffix")?,
                    head: 0,
                }));
            }
        }
        Ok(None)
    }

    fn process_circumfix_json_template(
        &mut self,
        args: &Value,
        lang: &str,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != lang {
            return Ok(None);
        }
        if let Some(source_prefix) = args.get_optional_str("2") {
            if source_prefix.is_empty() || source_prefix == "-" {
                return Ok(None);
            }
            if let Some(source_term) = args.get_optional_str("3") {
                if source_term.is_empty() || source_term == "-" {
                    return Ok(None);
                }
                if let Some(source_suffix) = args.get_optional_str("4") {
                    if source_suffix.is_empty() || source_suffix == "-" {
                        return Ok(None);
                    }
                    let source_term = clean_ety_term(source_term);
                    let source_term = self.string_pool.get_or_intern(source_term);
                    let source_prefix = clean_ety_term(source_prefix).to_string();
                    let source_suffix = clean_ety_term(source_suffix).to_string();
                    let source_circumfix = format!("{}- -{}", source_prefix, source_suffix);
                    let source_circumfix =
                        self.string_pool.get_or_intern(source_circumfix.as_str());

                    return Ok(Some(RawEtyNode {
                        source_terms: Box::new([source_term, source_circumfix]),
                        source_langs: Box::new([LANG_CODE2NAME.get_expected_index(lang)?; 2]),
                        mode: MODE.get_expected_index("circumfix")?,
                        head: 0,
                    }));
                }
            }
        }
        Ok(None)
    }

    fn process_infix_json_template(
        &mut self,
        args: &Value,
        lang: &str,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != lang {
            return Ok(None);
        }
        if let Some(source_term) = args.get_optional_str("2") {
            if source_term.is_empty() || source_term == "-" {
                return Ok(None);
            }
            if let Some(source_infix) = args.get_optional_str("3") {
                if source_infix.is_empty() || source_infix == "-" {
                    return Ok(None);
                }
                let source_term = clean_ety_term(source_term);
                let source_term = self.string_pool.get_or_intern(source_term);
                let source_infix = clean_ety_term(source_infix).to_string();
                let source_infix = format!("-{}-", source_infix);
                let source_infix = self.string_pool.get_or_intern(source_infix.as_str());
                return Ok(Some(RawEtyNode {
                    source_terms: Box::new([source_term, source_infix]),
                    source_langs: Box::new([LANG_CODE2NAME.get_expected_index(lang)?; 2]),
                    mode: MODE.get_expected_index("infix")?,
                    head: 0,
                }));
            }
        }
        Ok(None)
    }

    fn process_confix_json_template(
        &mut self,
        args: &Value,
        lang: &str,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != lang {
            return Ok(None);
        }
        if let Some(source_prefix) = args.get_optional_str("2") {
            if source_prefix.is_empty() || source_prefix == "-" {
                return Ok(None);
            }
            if let Some(source2) = args.get_optional_str("3") {
                if source2.is_empty() || source2 == "-" {
                    return Ok(None);
                }
                let source_prefix = clean_ety_term(source_prefix).to_string();
                let source_prefix = format!("{}-", source_prefix);
                let source_prefix = self.string_pool.get_or_intern(source_prefix.as_str());
                if let Some(source3) = args.get_optional_str("4") {
                    if source3.is_empty() || source3 == "-" {
                        return Ok(None);
                    }
                    let source_term = clean_ety_term(source2);
                    let source_term = self.string_pool.get_or_intern(source_term);
                    let source_suffix = clean_ety_term(source3).to_string();
                    let source_suffix = format!("-{}", source_suffix);
                    let source_suffix = self.string_pool.get_or_intern(source_suffix.as_str());
                    return Ok(Some(RawEtyNode {
                        source_terms: Box::new([source_prefix, source_term, source_suffix]),
                        source_langs: Box::new([LANG_CODE2NAME.get_expected_index(lang)?; 3]),
                        mode: MODE.get_expected_index("confix")?,
                        head: 1,
                    }));
                }
                let source_suffix = clean_ety_term(source2).to_string();
                let source_suffix = format!("-{}", source_suffix);
                let source_suffix = self.string_pool.get_or_intern(source_suffix.as_str());
                return Ok(Some(RawEtyNode {
                    source_terms: Box::new([source_prefix, source_suffix]),
                    source_langs: Box::new([LANG_CODE2NAME.get_expected_index(lang)?; 2]),
                    mode: MODE.get_expected_index("confix")?,
                    head: 0, // no true head here, arbitrarily take first
                }));
            }
        }
        Ok(None)
    }

    fn process_compound_type_json_template(
        &mut self,
        args: &Value,
        mode: &str,
        lang: &str,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != lang {
            return Ok(None);
        }

        let mut n = 2;
        let mut source_terms = vec![];
        let mut source_langs = vec![];
        while let Some(source_term) = args.get_optional_str(n.to_string().as_str()) {
            if source_term.is_empty() || source_term == "-" {
                return Ok(None);
            }
            if let Some(source_lang) = args.get_optional_str(format!("lang{n}").as_str()) {
                let source_lang_index = LANG_CODE2NAME.get_index(source_lang);
                if source_lang.is_empty() || source_lang == "-" || source_lang_index.is_none() {
                    return Ok(None);
                }
                let source_term = clean_ety_term(source_term);
                source_terms.push(self.string_pool.get_or_intern(source_term));
                source_langs.push(source_lang_index.unwrap());
            } else {
                let source_term = clean_ety_term(source_term);
                source_terms.push(self.string_pool.get_or_intern(source_term));
                source_langs.push(LANG_CODE2NAME.get_expected_index(lang)?);
            }
            n += 1;
        }
        if !source_terms.is_empty() {
            return Ok(Some(RawEtyNode {
                source_terms: source_terms.into_boxed_slice(),
                source_langs: source_langs.into_boxed_slice(),
                mode: MODE.get_expected_index(mode)?,
                head: 0, // no true head here, arbitrarily take first
            }));
        }
        Ok(None)
    }

    fn process_json_ety_template(
        &mut self,
        template: &Value,
        lang: &str,
    ) -> Result<Option<RawEtyNode>> {
        let name = template.get_expected_str("name")?;
        let args = template.get_expected_object("args")?;
        if let Some(&mode) = DERIVED_TYPE_TEMPLATES.get(name) {
            self.process_derived_type_json_template(args, mode, lang)
        } else if let Some(&mode) = ABBREV_TYPE_TEMPLATES.get(name) {
            self.process_abbrev_type_json_template(args, mode, lang)
        } else if let Some(&mode) = COMPOUND_TYPE_TEMPLATES.get(name) {
            match mode {
                "prefix" => self.process_prefix_json_template(args, lang),
                "suffix" => self.process_suffix_json_template(args, lang),
                "circumfix" => self.process_circumfix_json_template(args, lang),
                "infix" => self.process_infix_json_template(args, lang),
                "confix" => self.process_confix_json_template(args, lang),
                _ => self.process_compound_type_json_template(args, mode, lang),
            }
        } else {
            Ok(None)
        }
    }

    fn process_json_ety(
        &mut self,
        json_item: &Value,
        lang: &str,
    ) -> Result<Option<Box<[RawEtyNode]>>> {
        let mut raw_ety_nodes = vec![];
        if let Some(templates) = json_item.get_array("etymology_templates") {
            raw_ety_nodes.reserve(templates.len());
            for template in templates {
                if let Some(raw_ety_node) = self.process_json_ety_template(template, lang)? {
                    raw_ety_nodes.push(raw_ety_node);
                }
            }
        }

        // if no ety section or no templates, as a fallback we see if term
        // is listed as a "form_of" (item.senses[0].form_of[0].word)
        // or "alt_of" (item.senses[0].alt_of[0].word) another term.
        // e.g. "happenin'" is listed as an alt_of of "happening".
        if raw_ety_nodes.is_empty() {
            let alt_term = json_item
                .get_array("senses")
                .and_then(|senses| senses.get(0))
                .and_then(|sense| {
                    sense
                        .get_array("alt_of")
                        .or_else(|| sense.get_array("form_of"))
                })
                .and_then(|alt_list| alt_list.get(0))
                .and_then(|alt_obj| alt_obj.get_str("word"));
            match alt_term {
                Some(alt) => {
                    let raw_ety_node = RawEtyNode {
                        source_terms: Box::new([self.string_pool.get_or_intern(alt)]),
                        source_langs: Box::new([LANG_CODE2NAME.get_expected_index(lang)?]),
                        mode: MODE.get_expected_index("form")?,
                        head: 0,
                    };
                    raw_ety_nodes.push(raw_ety_node);
                    return Ok(Some(raw_ety_nodes.into_boxed_slice()));
                }
                None => {
                    return Ok(None);
                }
            }
        }
        Ok(Some(raw_ety_nodes.into_boxed_slice()))
    }

    // cf. https://en.wiktionary.org/wiki/Template:root. For now we skip
    // attempting to deal with multiple roots listed in a root template or
    // multiple root templates being listed. In both cases we just take the
    // first root term seen. If we discover it is common, we will handle it. $$
    // NEED TO HANDLE CHECKING CATEGORIES IF NO ROOT TEMPLATE FOUND, $$ ONCE WE
    // MAKE PHF MAPS FOR LANG AND POS.
    fn process_json_root(&mut self, json_item: &Value, lang: &str) -> Result<Option<RawRoot>> {
        if let Some(templates) = json_item.get_array("etymology_templates") {
            for template in templates {
                if template.get_expected_str("name")? == "root" {
                    let args = template.get_expected_object("args")?;
                    let term_lang = args.get_expected_str("1")?;
                    if term_lang != lang {
                        return Ok(None);
                    }
                    let root_lang = args.get_expected_str("2")?;
                    let root_lang_index = LANG_CODE2NAME.get_index(root_lang);
                    let root_term = args.get_optional_str("3");
                    let further_root_term = args.get_optional_str("4");
                    if root_lang_index.is_none()
                        || root_term.is_none()
                        || further_root_term.is_some()
                    {
                        return Ok(None);
                    }
                    let root_lang_index = root_lang_index.unwrap();
                    let mut root_term = root_term.unwrap();
                    let mut root_sense_id = "";
                    // Sometimes a root's senseid is given in parentheses after the term in
                    // the 3 arg slot, see e.g. https://en.wiktionary.org/wiki/blaze.
                    if let Some(right_paren_idx) = root_term.rfind(')') {
                        if let Some(left_paren_idx) = root_term.rfind(" (") {
                            root_sense_id = &root_term[left_paren_idx + 2..right_paren_idx];
                            root_term = &root_term[..left_paren_idx];
                        }
                    } else if let Some(sense_id) = args.get_optional_str("id") {
                        root_sense_id = sense_id;
                    }
                    let root_sense_id = if root_sense_id.is_empty() {
                        None
                    } else {
                        Some(self.string_pool.get_or_intern(root_sense_id))
                    };
                    return Ok(Some(RawRoot {
                        term: self.string_pool.get_or_intern(root_term),
                        lang: root_lang_index,
                        sense_id: root_sense_id,
                    }));
                }
            }
        }
        // if { $$ CHECK CATEGORIES FOR ROOT $$}
        Ok(None)
    }

    fn process_redirect(&mut self, items: &mut Items, json_item: &Value) -> Result<()> {
        // there is one tricky redirect that makes it so title is
        // "optional" (i.e. could be empty string):
        // {"title": "", "redirect": "Appendix:Control characters"}
        if let Some(title) = json_item.get_optional_str("title") {
            for ignored in IGNORED_REDIRECTS.iter() {
                if title.strip_prefix(ignored).is_some() {
                    return Ok(());
                }
            }
            let redirect = json_item.get_expected_str("redirect")?;
            // e.g. Reconstruction:Proto-Germanic/pīpǭ
            if let Some(from) = self.process_reconstruction_title(title)? {
                // e.g. "Reconstruction:Proto-West Germanic/pīpā"
                if let Some(to) = self.process_reconstruction_title(redirect)? {
                    items.redirects.reconstruction.insert(from, to);
                }
                return Ok(());
            }

            // otherwise, this is a simple term-to-term redirect
            let from = self.string_pool.get_or_intern(title);
            let to = self.string_pool.get_or_intern(redirect);
            items.redirects.regular.insert(from, to);
        }
        Ok(())
    }

    fn process_reconstruction_title(&mut self, title: &str) -> Result<Option<ReconstructionTitle>> {
        // e.g. Reconstruction:Proto-Germanic/pīpǭ
        if let Some(title) = title.strip_prefix("Reconstruction:") {
            if let Some(slash) = title.find('/') {
                let language = &title[..slash];
                if let Some(term) = title.get(slash + 1..) {
                    if let Some(language_index) = LANG_NAME2CODE.get_index(language) {
                        return Ok(Some(ReconstructionTitle {
                            language: language_index,
                            term: self.string_pool.get_or_intern(term),
                        }));
                    }
                }
            }
        }
        Ok(None)
    }

    fn process_json_item(&mut self, items: &mut Items, json_item: &Value) -> Result<()> {
        // Some wiktionary pages are redirects. These are actually used somewhat
        // heavily, so we need to take them into account
        // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
        if json_item.contains_key("redirect") {
            return self.process_redirect(items, json_item);
        }
        let term = get_term(json_item)?;
        // 'pos' key must be present
        let pos = json_item.get_expected_str("pos")?;
        if should_ignore_term(term, pos) {
            return Ok(());
        }
        // 'lang_code' key must be present
        let lang = json_item.get_expected_str("lang_code")?;
        let lang_index = LANG_CODE2NAME.get_expected_index(lang)?;
        // 'etymology_text' key may be missing or empty
        let ety_text_hash = json_item.get_str("etymology_text").map(|s| {
            let mut hasher = AHasher::default();
            hasher.write(s.as_bytes());
            hasher.finish()
        });
        // 'senses' key should always be present with non-empty value, but glosses
        // may be missing or empty.
        let gloss = json_item
            .get_array("senses")
            .and_then(|senses| senses.get(0))
            .and_then(|sense| sense.get_array("glosses"))
            .and_then(|glosses| glosses.get(0))
            .and_then(simd_json::ValueAccess::as_str)
            .and_then(|s| (!s.is_empty()).then(|| self.string_pool.get_or_intern(s)));

        let raw_root = self.process_json_root(json_item, lang)?;
        let raw_ety_nodes = self.process_json_ety(json_item, lang)?;

        let item = Item {
            is_imputed: false,
            i: items.n,
            term: self.string_pool.get_or_intern(term),
            lang: lang_index,
            ety_num: 0, // temp value to be changed if need be in add()
            pos: POS.get_expected_index(pos)?,
            gloss,
            gloss_num: 0, // temp value to be changed if need be in add()
            raw_ety_nodes,
            raw_root,
        };
        items.add(ety_text_hash, item)?;
        Ok(())
    }

    fn process_file(&mut self, file: File) -> Result<Items> {
        let mut items = Items::default();
        let reader = BufReader::new(file);
        let gz = GzDecoder::new(reader);
        let gz_reader = BufReader::new(gz);
        let lines = ByteLines::new(gz_reader);
        for mut line in lines.into_iter().filter_map(Result::ok) {
            let json_item = to_borrowed_value(&mut line)?;
            self.process_json_item(&mut items, &json_item)?;
        }
        Ok(items)
    }
}

fn clean_ety_term(term: &str) -> &str {
    // Reconstructed terms (e.g. PIE) are supposed to start with "*" when cited
    // in etymologies but their entry titles (and hence wiktextract "word"
    // field) do not. This is done by
    // https://en.wiktionary.org/wiki/Module:links. Sometimes reconstructed
    // terms are missing this *, and sometimes non-reconstructed terms start
    // with * incorrectly. So we strip the * in every case. This will break
    // terms that actually start with *, but there are almost none of these, and
    // none of them are particularly relevant for our purposes AFAIK.
    term.strip_prefix('*').unwrap_or(term)
}

fn remove_punctuation(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_ascii_punctuation())
        .collect::<String>()
}

fn should_ignore_term(term: &str, pos: &str) -> bool {
    // This function needs revisiting depending on results.

    // We would generally like to ignore phrases, and potentially other things.
    //  Barring all phrases may be both too strict and not strict enough. Too
    // strict because certain phrases may be relevant for etymologies (i.e. a
    // phrase became one word in a daughter language). Not strict enough because
    // many phrases are categorized as other pos. See e.g.
    // https://en.wiktionary.org/wiki/this,_that,_or_the_other. Ignoring terms
    // that contain any ascii punctuation is too strict, as this would ingore
    // e.g. affixes with -. Ignoring terms with any ascii whitespace is too
    // strict as well, as this would ignore e.g. circumfixes (e.g. "ver- -en").
    if pos.contains("phrase") || term.contains(|c: char| c == ',') {
        return true;
    }
    false
}

// We look for a canonical form, otherwise we take the "word" field.
// See notes.md for motivation.
fn get_term<'a>(json_item: &'a Value) -> Result<&'a str> {
    if let Some(forms) = json_item.get_array("forms") {
        let mut f = 0;
        while let Some(form) = forms.get(f) {
            if let Some(tags) = form.get_array("tags") {
                let mut t = 0;
                while let Some(tag) = tags.get(t).as_str() {
                    if tag == "canonical" {
                        return Ok(form.get_expected_str("form")?);
                    }
                    t += 1;
                }
            }
            f += 1;
        }
    }
    Ok(json_item.get_expected_str("word")?)
}

pub(crate) struct ProcessedData {
    string_pool: StringPool,
    items: Items,
    sources: Sources,
}

/// # Errors
///
/// Will return `Err` if any unexpected issue arises parsing the wiktextract
/// data or writing to Turtle file.
pub fn wiktextract_to_turtle(wiktextract_path: &str, turtle_path: &str) -> Result<Instant> {
    let mut t = Instant::now();
    let file = File::open(wiktextract_path)?;
    println!("Processing raw wiktextract data from {wiktextract_path}...");
    let mut processor = RawDataProcessor::default();
    let items = processor.process_file(file)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    println!("Processing etymologies...");
    t = Instant::now();
    let string_pool = processor.string_pool;
    let sources = items.generate_sources(&string_pool)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    println!("Writing RDF to Turtle file {turtle_path}...");
    t = Instant::now();
    let data = ProcessedData {
        string_pool,
        items,
        sources,
    };
    write_turtle_file(&data, turtle_path)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    t = Instant::now();
    println!("Dropping all processed data...");
    Ok(t)
}

/// # Errors
///
/// Will return `Err` if any unexpected issue arises building the Oxigraph store.
pub fn build_store(turtle_path: &str, store_path: &str, skip_optimizing: bool) -> Result<()> {
    let mut t = Instant::now();
    println!("Building oxigraph store {store_path}...");
    let turtle = BufReader::new(File::open(turtle_path)?);
    // delete any previous oxigraph db
    if Path::new(store_path).is_dir() {
        remove_dir_all(store_path)?;
    }
    let store = Store::open(store_path)?;
    store
        .bulk_loader()
        .load_graph(turtle, Turtle, DefaultGraph, None)?;
    store.flush()?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    if !skip_optimizing {
        t = Instant::now();
        println!("Optimizing oxigraph store {store_path}...");
        store.optimize()?;
        store.flush()?;
        println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    }
    Ok(())
}
