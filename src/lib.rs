//! WIP attempt to digest etymologies from wiktextract data

mod etymology_templates;

use crate::etymology_templates::{
    ABBREV_TYPE_TEMPLATES, COMPOUND_TYPE_TEMPLATES, DERIVED_TYPE_TEMPLATES,
};

use std::cmp::min;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};
use bytelines::ByteLines;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use hashbrown::{HashMap, HashSet};
use indicatif::{ProgressBar, ProgressStyle};
use phf::{phf_set, Set};
use simd_json::{to_borrowed_value, value::borrowed::Value, ValueAccess};
use string_interner::{backend::StringBackend, symbol::SymbolU32, StringInterner};

const WIKTEXTRACT_URL: &str = "https://kaikki.org/dictionary/raw-wiktextract-data.json.gz";
const WIKTEXTRACT_PATH: &str = "data/raw-wiktextract-data.json.gz";
// const WIKTEXTRACT_URL: &str = "http://0.0.0.0:8000/data/test/bank.json.gz";
// const WIKTEXTRACT_PATH: &str = "data/test/bank.json.gz";
const LANG_PATH: &str = "data/lang.txt";
const POS_PATH: &str = "data/pos.txt";
const ID_PATH: &str = "data/id.txt";
const SOURCE_PATH: &str = "data/source.txt";
const DB_PATH: &str = "data/wety.db";
const RDF_PATH: &str = "data/wety.rdf";

// https://github.com/tatuylonen/wiktextract/blob/master/wiktwords
static IGNORED_REDIRECTS: Set<&'static str> = phf_set! {
    "Index:", "Help:", "MediaWiki:", "Citations:", "Concordance:", "Rhymes:",
    "Thread:", "Summary:", "File:", "Transwiki:", "Category:", "Appendix:",
    "Wiktionary:", "Thesaurus:", "Module:", "Template:"
};

// For etymological relationships given by DERIVED_TYPE_TEMPLATES
// and ABBREV_TYPE_TEMPLATES in etymology_templates.rs.
// Akin to Wikidata's derived from lexeme https://www.wikidata.org/wiki/Property:P5191
// and mode of derivation https://www.wikidata.org/wiki/Property:P5886
#[derive(Hash, Eq, PartialEq, Debug)]
struct DerivedFrom {
    item: Rc<Item>,
    mode: SymbolU32,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct RawDerivedFrom {
    source_term: SymbolU32,
    source_lang: SymbolU32,
    mode: SymbolU32,
}

// For etymological relationships given by COMPOUND_TYPE_TEMPLATES
// in etymology_templates.rs.
// Akin to Wikidata's combines lexeme https://www.wikidata.org/wiki/Property:P5238
#[derive(Hash, Eq, PartialEq, Debug)]
struct Combines {
    items: Box<[Rc<Item>]>,
    mode: SymbolU32,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct RawCombines {
    source_terms: Box<[SymbolU32]>,
    source_langs: Option<Box<[SymbolU32]>>,
    mode: SymbolU32,
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum EtyNode {
    DerivedFrom(DerivedFrom),
    Combines(Combines),
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum RawEtyNode {
    RawDerivedFrom(RawDerivedFrom),
    RawCombines(RawCombines),
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct Item {
    term: SymbolU32,             // e.g. "bank"
    lang: SymbolU32,             // e.g "en", i.e. the wiktextract lang_code
    language: SymbolU32,         // e.g. "English" i.e. the wiktextract lang
    ety_text: Option<SymbolU32>, // e.g. "From Middle English banke, from Middle French banque...
    ety_num: u8, // the nth ety encountered for this term-lang combo, 0 means ety is missing or ""
    pos: SymbolU32, // e.g. "noun"
    gloss: Option<SymbolU32>, // e.g. "An institution where one can place and borrow money...
    gloss_num: u8, // the nth gloss encountered for this term-lang-ety-pos combo, 0 means gloss is missing or ""
    raw_ety_nodes: Option<Box<[RawEtyNode]>>,
}

impl Item {
    fn id(&self, string_pool: &StringPool) -> String {
        // term__lang__eN__pos__sM
        format!(
            "{}__{}__e{}__{}__s{}",
            string_pool.resolve(self.term),
            string_pool.resolve(self.lang),
            self.ety_num,
            string_pool.resolve(self.pos),
            self.gloss_num
        )
    }
}

type GlossMap = HashMap<Option<SymbolU32>, (u8, Rc<Item>)>;
type PosMap = HashMap<SymbolU32, GlossMap>;
type EtyMap = HashMap<Option<SymbolU32>, (u8, PosMap)>;
type LangMap = HashMap<SymbolU32, EtyMap>;
type TermMap = HashMap<SymbolU32, LangMap>;

#[derive(Default)]
struct Items {
    term_map: TermMap,
}

impl Items {
    fn add(&mut self, mut item: Item) -> Result<()> {
        // check if the item's term has been seen before
        if !self.term_map.contains_key(&item.term) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let mut lang_map = LangMap::new();
            let (gloss, pos, ety_text, lang, term) =
                (item.gloss, item.pos, item.ety_text, item.lang, item.term);
            let gloss_num = gloss.map_or(0u8, |_| 1u8);
            item.gloss_num = gloss_num;
            let ety_num = ety_text.map_or(0u8, |_| 1u8);
            item.ety_num = ety_num;
            gloss_map.insert(gloss, (gloss_num, Rc::from(item)));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text, (ety_num, pos_map));
            lang_map.insert(lang, ety_map);
            self.term_map.insert(term, lang_map);
            return Ok(());
        }
        // since term has been seen before, there must be at least one lang for it
        // check if item's lang has been seen before
        let lang_map: &mut LangMap = self
            .term_map
            .get_mut(&item.term)
            .ok_or_else(|| anyhow!("no LangMap for term when adding:\n{:#?}", item))?;
        if !lang_map.contains_key(&item.lang) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let (gloss, pos, ety_text, lang) = (item.gloss, item.pos, item.ety_text, item.lang);
            let gloss_num = gloss.map_or(0u8, |_| 1u8);
            item.gloss_num = gloss_num;
            let ety_num = ety_text.map_or(0u8, |_| 1u8);
            item.ety_num = ety_num;
            gloss_map.insert(gloss, (gloss_num, Rc::from(item)));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text, (ety_num, pos_map));
            lang_map.insert(lang, ety_map);
            return Ok(());
        }
        // since lang has been seen before, there must be at least one ety (possibly None)
        // check if this ety has been seen in this lang before
        let ety_map: &mut EtyMap = lang_map
            .get_mut(&item.lang)
            .ok_or_else(|| anyhow!("no EtyMap for lang when adding:\n{:#?}", item))?;
        if !ety_map.contains_key(&item.ety_text) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let (gloss, pos, ety_text) = (item.gloss, item.pos, item.ety_text);
            let gloss_num = gloss.map_or(0u8, |_| 1u8);
            item.gloss_num = gloss_num;
            let ety_map_len = u8::try_from(ety_map.len())?;
            let ety_num = ety_text.map_or(0u8, |_| 1u8 + ety_map_len);
            item.ety_num = ety_num;
            gloss_map.insert(gloss, (gloss_num, Rc::from(item)));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text, (ety_num, pos_map));
            return Ok(());
        }
        // since ety has been seen before, there must be at least one pos
        // check if this pos has been seen for this ety before
        let (ety_num, pos_map): &mut (u8, PosMap) = ety_map
            .get_mut(&item.ety_text)
            .ok_or_else(|| anyhow!("no PosMap for ety when adding:\n{:#?}", item))?;
        if !pos_map.contains_key(&item.pos) {
            let mut gloss_map = GlossMap::new();
            let (gloss, pos) = (item.gloss, item.pos);
            let gloss_num = gloss.map_or(0u8, |_| 1u8);
            item.gloss_num = gloss_num;
            item.ety_num = *ety_num;
            gloss_map.insert(gloss, (gloss_num, Rc::from(item)));
            pos_map.insert(pos, gloss_map);
            return Ok(());
        }
        // since pos has been seen before, there must be at least one gloss (possibly None)
        let gloss_map: &mut GlossMap = pos_map
            .get_mut(&item.pos)
            .ok_or_else(|| anyhow!("no GlossMap for pos when adding:\n{:#?}", item))?;
        if !gloss_map.contains_key(&item.gloss) {
            let gloss = item.gloss;
            let gloss_map_len = u8::try_from(gloss_map.len())?;
            let gloss_num = gloss.map_or(0u8, |_| 1u8 + gloss_map_len);
            item.gloss_num = gloss_num;
            item.ety_num = *ety_num;
            gloss_map.insert(gloss, (gloss_num, Rc::from(item)));
            return Ok(());
        }
        Ok(())
    }
}

// wrapper around a Hashmap linking items with their immediate etymological source,
// as parsed from the first raw ety node.
#[derive(Default)]
struct Sources {
    item_map: HashMap<Rc<Item>, Option<EtyNode>>,
}

impl Sources {
    fn add(&mut self, item: &Rc<Item>, ety_node_opt: Option<EtyNode>) -> Result<()> {
        self.item_map
            .try_insert(Rc::clone(item), ety_node_opt)
            .and(Ok(()))
            .map_err(|_| anyhow!("Tried inserting duplicate item:\n{:#?}", item))
    }
    // For now we'll just take the first node. But cf. notes.md.
    /// Only to be called once all json items have been processed into items.
    fn process_item_raw_ety_nodes(
        &mut self,
        string_pool: &StringPool,
        items: &Items,
        item: &Rc<Item>,
    ) -> Result<()> {
        if item.raw_ety_nodes.is_none() {
            return Ok(()); // don't add anything to sources if no valid raw ety nodes
        }
        let sense = Sense::new(string_pool, item);
        // The boxed array should never be empty, based on the logic in
        // process_json_ety_templates().
        let raw_ety_node = &item.raw_ety_nodes.as_ref().unwrap()[0];
        match raw_ety_node {
            RawEtyNode::RawDerivedFrom(raw_derived_from) => {
                let ety_map = items
                    .term_map
                    .get(&raw_derived_from.source_term)
                    .and_then(|lang_map| lang_map.get(&raw_derived_from.source_lang));
                match ety_map {
                    // if we found an ety_map, we're guaranteed to find at least
                    // one item at the end of the following nested iteration. If
                    // there are multiple items, we have to do a word sense disambiguation.
                    Some(ety_map) => {
                        if let Some(source_item) = ety_map
                            .values()
                            .flat_map(|(_, pos_map)| {
                                pos_map.values().flat_map(|gloss_map| {
                                    gloss_map.values().map(|(_, other_item)| other_item)
                                })
                            })
                            .max_by_key(|other_item| {
                                let other_item_sense = Sense::new(string_pool, other_item);
                                sense.lesk_score(&other_item_sense)
                            })
                        {
                            let node = EtyNode::DerivedFrom(DerivedFrom {
                                item: Rc::clone(source_item),
                                mode: raw_derived_from.mode,
                            });
                            self.add(item, Some(node))?;
                        } else {
                            // should never be reached
                            bail!(
                                "ety_map was found but no ultimate item for:\n{}, {}",
                                string_pool.resolve(raw_derived_from.source_lang),
                                string_pool.resolve(raw_derived_from.source_term),
                            );
                        }
                    }
                    None => {
                        self.add(item, None)?;
                    }
                }
            }
            RawEtyNode::RawCombines(raw_combines) => {
                let source_terms = &raw_combines.source_terms;
                let source_langs = raw_combines
                    .source_langs
                    .as_ref()
                    .map_or_else(|| [item.lang].repeat(source_terms.len()), |s| s.to_vec());
                let mut source_items = Vec::with_capacity(source_terms.len());
                for (source_term, source_lang) in source_terms.iter().zip(source_langs.iter()) {
                    let ety_map = items
                        .term_map
                        .get(source_term)
                        .and_then(|lang_map| lang_map.get(source_lang));
                    match ety_map {
                        // if we found an ety_map, we're guaranteed to find at least
                        // one item at the end of the following nested iteration. If
                        // there are multiple items, we have to do a word sense disambiguation.
                        Some(ety_map) => {
                            if let Some(source_item) = ety_map
                                .values()
                                .flat_map(|(_, pos_map)| {
                                    pos_map.values().flat_map(|gloss_map| {
                                        gloss_map.values().map(|(_, other_item)| other_item)
                                    })
                                })
                                .max_by_key(|other_item| {
                                    let other_item_sense = Sense::new(string_pool, other_item);
                                    sense.lesk_score(&other_item_sense)
                                })
                            {
                                source_items.push(Rc::clone(source_item));
                            } else {
                                // should never be reached
                                bail!(
                                    "ety_map was found but no ultimate item for:\n{}, {}",
                                    string_pool.resolve(*source_lang),
                                    string_pool.resolve(*source_term),
                                );
                            }
                        }
                        None => {}
                    }
                }
                if source_items.len() == source_terms.len() {
                    let node = EtyNode::Combines(Combines {
                        items: source_items.into_boxed_slice(),
                        mode: raw_combines.mode,
                    });
                    self.add(item, Some(node))?;
                } else {
                    self.add(item, None)?;
                }
            }
        }
        Ok(())
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
// $$ This should be a more structured representation using all fields of Item
// $$ but we'll start with just a bag of words from the gloss and Lesk score
// $$ comparison for now. In a better implementation, shared pos e.g. could be weighted more.
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

#[derive(Hash, Eq, PartialEq, Debug)]
struct ReconstructionTitle {
    language: SymbolU32,
    term: SymbolU32,
}

#[derive(Default)]
pub struct Processor {
    string_pool: StringPool,
    items: Items,
    sources: Sources,
    reconstruction_redirects: HashMap<ReconstructionTitle, ReconstructionTitle>,
    regular_redirects: HashMap<SymbolU32, SymbolU32>,
    langs: HashMap<SymbolU32, SymbolU32>,
    poss: HashSet<SymbolU32>,
}

impl Processor {
    fn write_sources(&self) -> Result<()> {
        let mut file = File::create(SOURCE_PATH)?;
        for (item, ety) in self.sources.item_map.iter() {
            file.write_all(format!("{}, ", item.id(&self.string_pool)).as_bytes())?;
            match ety {
                Some(ety) => match ety {
                    EtyNode::DerivedFrom(d) => file.write_all(
                        format!(
                            "{}, {}",
                            self.string_pool.resolve(d.mode),
                            d.item.id(&self.string_pool)
                        )
                        .as_bytes(),
                    )?,
                    EtyNode::Combines(c) => {
                        file.write_all(
                            format!("{}, ", self.string_pool.resolve(c.mode)).as_bytes(),
                        )?;
                        for i in c.items.iter() {
                            file.write_all(format!("{}, ", i.id(&self.string_pool)).as_bytes())?;
                        }
                    }
                },
                None => file.write_all(b"NONE")?,
            }
            file.write_all(b"\n")?;
        }
        Ok(())
    }

    fn write_ids(&self) -> Result<()> {
        let mut file = File::create(ID_PATH)?;
        for lang_map in self.items.term_map.values() {
            for ety_map in lang_map.values() {
                for (_, pos_map) in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for (_, item) in gloss_map.values() {
                            file.write_all(format!("{}\n", item.id(&self.string_pool)).as_bytes())?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn write_poss(&self) -> Result<()> {
        let mut file = File::create(POS_PATH)?;
        for pos in self.poss.iter() {
            file.write_all(format!("{}\n", self.string_pool.resolve(*pos)).as_bytes())?;
        }
        Ok(())
    }

    fn write_langs(&self) -> Result<()> {
        let mut file = File::create(LANG_PATH)?;
        for (lang, language) in self.langs.iter() {
            file.write_all(
                format!(
                    "{}, {}\n",
                    self.string_pool.resolve(*lang),
                    self.string_pool.resolve(*language),
                )
                .as_bytes(),
            )?;
        }
        Ok(())
    }

    fn process_derived_type_json_template(
        &mut self,
        args: &Value,
        mode: &str,
        lang: SymbolU32,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != self.string_pool.resolve(lang) {
            return Ok(None);
        }
        let source_lang = args.get_expected_str("2")?;
        return if let Some(source_term) = args.get_optional_str("3") {
            let source_term = clean_ety_term(self.string_pool.resolve(lang), source_term);
            match source_term {
                "" | "-" => Ok(None),
                _ => Ok(Some(RawEtyNode::RawDerivedFrom(RawDerivedFrom {
                    source_term: self.string_pool.get_or_intern(source_term),
                    source_lang: self.string_pool.get_or_intern(source_lang),
                    mode: self.string_pool.get_or_intern(mode),
                }))),
            }
        } else {
            Ok(None)
        };
    }

    fn process_abbrev_type_json_template(
        &mut self,
        args: &Value,
        mode: &str,
        lang: SymbolU32,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != self.string_pool.resolve(lang) {
            return Ok(None);
        }
        return if let Some(source_term) = args.get_optional_str("2") {
            let source_term = clean_ety_term(self.string_pool.resolve(lang), source_term);
            match source_term {
                "" | "-" => Ok(None),
                _ => Ok(Some(RawEtyNode::RawDerivedFrom(RawDerivedFrom {
                    source_term: self.string_pool.get_or_intern(source_term),
                    source_lang: lang,
                    mode: self.string_pool.get_or_intern(mode),
                }))),
            }
        } else {
            Ok(None)
        };
    }

    fn process_compound_type_json_template(
        &mut self,
        args: &Value,
        mode: &str,
        lang: SymbolU32,
    ) -> Result<Option<RawEtyNode>> {
        let term_lang = args.get_expected_str("1")?;
        if term_lang != self.string_pool.resolve(lang) {
            return Ok(None);
        }

        let mut n = 2;
        let mut source_terms = vec![];
        let mut source_langs = vec![];
        let mut has_source_langs = false;
        while let Some(source_term) = args.get_optional_str(n.to_string().as_str()) {
            if source_term.is_empty() || source_term == "-" {
                break;
            }
            if let Some(source_lang) = args.get_optional_str(format!("lang{n}").as_str()) {
                if source_lang.is_empty() || source_lang == "-" {
                    break;
                }
                has_source_langs = true;
                let source_term = clean_ety_term(source_lang, source_term);
                source_terms.push(self.string_pool.get_or_intern(source_term));
                source_langs.push(self.string_pool.get_or_intern(source_lang));
            } else {
                let source_term = clean_ety_term(self.string_pool.resolve(lang), source_term);
                source_terms.push(self.string_pool.get_or_intern(source_term));
                source_langs.push(lang);
            }
            n += 1;
        }
        Ok((!source_terms.is_empty()).then(|| {
            RawEtyNode::RawCombines(RawCombines {
                source_terms: source_terms.into_boxed_slice(),
                source_langs: has_source_langs.then(|| source_langs.into_boxed_slice()),
                mode: self.string_pool.get_or_intern(mode),
            })
        }))
    }

    fn process_json_ety_template(
        &mut self,
        template: &Value,
        lang: SymbolU32,
    ) -> Result<Option<RawEtyNode>> {
        if let Some(name) = template.get_str("name") {
            let args = template.get_expected_object("args")?;
            if DERIVED_TYPE_TEMPLATES.contains_key(name) {
                let mode = *DERIVED_TYPE_TEMPLATES.get(name).unwrap();
                self.process_derived_type_json_template(args, mode, lang)
            } else if ABBREV_TYPE_TEMPLATES.contains_key(name) {
                let mode = *ABBREV_TYPE_TEMPLATES.get(name).unwrap();
                self.process_abbrev_type_json_template(args, mode, lang)
            } else if COMPOUND_TYPE_TEMPLATES.contains_key(name) {
                let mode = *COMPOUND_TYPE_TEMPLATES.get(name).unwrap();
                self.process_compound_type_json_template(args, mode, lang)
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn process_json_ety_templates(
        &mut self,
        json_item: &Value,
        lang: SymbolU32,
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
                    let raw_ety_node = RawEtyNode::RawDerivedFrom(RawDerivedFrom {
                        source_term: self.string_pool.get_or_intern(alt),
                        source_lang: lang,
                        mode: self.string_pool.get_or_intern("form of"),
                    });
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

    fn process_redirect(&mut self, json_item: &Value) -> Result<()> {
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
            if let Some(from) = self.process_reconstruction_title(title) {
                // e.g. "Reconstruction:Proto-West Germanic/pīpā"
                if let Some(to) = self.process_reconstruction_title(redirect) {
                    self.reconstruction_redirects.insert(from, to);
                }
                return Ok(());
            }

            // otherwise, this is a simple term-to-term redirect
            let from = self.string_pool.get_or_intern(title);
            let to = self.string_pool.get_or_intern(redirect);
            self.regular_redirects.insert(from, to);
        }
        Ok(())
    }

    fn process_reconstruction_title(&mut self, title: &str) -> Option<ReconstructionTitle> {
        // e.g. Reconstruction:Proto-Germanic/pīpǭ
        if let Some(title) = title.strip_prefix("Reconstruction:") {
            if let Some(slash) = title.find('/') {
                let language = &title[..slash];
                if let Some(term) = title.get(slash + 1..) {
                    return Some(ReconstructionTitle {
                        language: self.string_pool.get_or_intern(language),
                        term: self.string_pool.get_or_intern(term),
                    });
                }
            }
        }
        None
    }

    // We look for a canonical form, otherwise we take the "word" field.
    // See notes.md for motivation.
    fn get_term(&mut self, json_item: &Value) -> Result<SymbolU32> {
        if let Some(forms) = json_item.get_array("forms") {
            let mut f = 0;
            while let Some(form) = forms.get(f) {
                if let Some(tags) = form.get_array("tags") {
                    let mut t = 0;
                    while let Some(tag) = tags.get(t).as_str() {
                        if tag == "canonical" {
                            let canonical_form = form.get_expected_str("form")?;
                            return Ok(self.string_pool.get_or_intern(canonical_form));
                        }
                        t += 1;
                    }
                }
                f += 1;
            }
        }
        return Ok(self
            .string_pool
            .get_or_intern(json_item.get_expected_str("word")?));
    }

    fn process_json_item(&mut self, json_item: &Value) -> Result<()> {
        // Some wiktionary pages are redirects. These are actually used somewhat
        // heavily, so we need to take them into account
        // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
        if json_item.contains_key("redirect") {
            return self.process_redirect(json_item);
        }
        let term = self.get_term(json_item)?;
        // 'lang_code' key must be present
        let lang = self
            .string_pool
            .get_or_intern(json_item.get_expected_str("lang_code")?);
        // 'lang' key must be present
        let language = self
            .string_pool
            .get_or_intern(json_item.get_expected_str("lang")?);
        // 'etymology_text' key may be missing or empty
        let ety_text = json_item
            .get_str("etymology_text")
            .map(|s| self.string_pool.get_or_intern(s));
        // 'pos' key must be present
        let pos = self
            .string_pool
            .get_or_intern(json_item.get_expected_str("pos")?);
        // 'senses' key should always be present with non-empty value, but glosses
        // may be missing or empty.
        let gloss = json_item
            .get_array("senses")
            .and_then(|senses| senses.get(0))
            .and_then(|sense| sense.get_array("glosses"))
            .and_then(|glosses| glosses.get(0))
            .and_then(simd_json::ValueAccess::as_str)
            .and_then(|s| (!s.is_empty()).then(|| self.string_pool.get_or_intern(s)));

        let raw_ety_nodes = self.process_json_ety_templates(json_item, lang)?;

        self.langs.insert(lang, language);
        self.poss.insert(pos);

        let item = Item {
            term,
            lang,
            language,
            ety_text,
            ety_num: 0, // temp value to be changed if need be in add()
            pos,
            gloss,
            gloss_num: 0, // temp value to be changed if need be in add()
            raw_ety_nodes,
        };
        self.items.add(item)?;
        Ok(())
    }

    fn process_json_items<T: BufRead>(&mut self, lines: ByteLines<T>) -> Result<()> {
        for mut line in lines.into_iter().filter_map(Result::ok) {
            let json_item = to_borrowed_value(&mut line)?;
            self.process_json_item(&json_item)?;
        }
        Ok(())
    }

    fn process_file(&mut self, file: File) -> Result<()> {
        let reader = BufReader::new(file);
        let gz = GzDecoder::new(reader);
        let gz_reader = BufReader::new(gz);
        let lines = ByteLines::new(gz_reader);
        self.process_json_items(lines)?;
        Ok(())
    }

    fn process_items(&mut self) -> Result<()> {
        for lang_map in self.items.term_map.values() {
            for ety_map in lang_map.values() {
                for (_, pos_map) in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for (_, item) in gloss_map.values() {
                            self.sources.process_item_raw_ety_nodes(
                                &self.string_pool,
                                &self.items,
                                item,
                            )?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Will return `Err` if any unexpected problem arises in processing.
    pub async fn process_wiktextract_data(&mut self) -> Result<()> {
        let file = if let Ok(file) = File::open(WIKTEXTRACT_PATH) {
            println!("Processing data from local file {WIKTEXTRACT_PATH}");
            file
        } else {
            // file doesn't exist or error opening it; download it
            println!("No local file found, downloading from {WIKTEXTRACT_URL}");
            download_file(WIKTEXTRACT_URL, WIKTEXTRACT_PATH).await?;
            let file = File::open(WIKTEXTRACT_PATH)
                .map_err(|_| anyhow!("Failed to open file '{WIKTEXTRACT_PATH}'"))?;
            println!("Processing data from downloaded file {WIKTEXTRACT_PATH}");
            file
        };

        self.process_file(file)?;
        println!("Finished");
        println!("Writing all encountered PoSs to {}", POS_PATH);
        self.write_poss()?;
        println!("Finished");
        println!("Writing all encountered langs to {}", LANG_PATH);
        self.write_langs()?;
        println!("Finished");
        println!("Writing all generated term ids to {}", ID_PATH);
        self.write_ids()?;
        println!("Finished");
        println!("Processing etymologies");
        self.process_items()?;
        println!("Finished");
        println!(
            "Writing all found immediate etymology relationships to {}",
            SOURCE_PATH
        );
        self.write_sources()?;
        println!("Finished");
        println!("Writing to database {DB_PATH}");
        // write to oxigraph store
        println!("Finished");
        println!("Writing to RDF {RDF_PATH}");
        // dump oxigraph store
        println!("Finished");
        println!("All done! Exiting");
        Ok(())
    }
}

fn clean_ety_term<'a>(lang: &str, term: &'a str) -> &'a str {
    // Reconstructed terms (e.g. PIE) are supposed to start with "*" when cited in
    // etymologies but their entry titles (and hence wiktextract "word" field) do not.
    // This is done by https://en.wiktionary.org/wiki/Module:links.

    // lang code ends with "-pro" iff term is a Reconstruction
    if lang.ends_with("-pro") {
        // it's common enough for proto terms to be missing *, so
        // we can't just err if we don't find it
        term.strip_prefix('*').unwrap_or(term)
    } else {
        term
    }

    // else do regular lang entryName creation stuff here a la
    // https://en.wiktionary.org/wiki/Module:languages#Language:makeEntryName
}

fn remove_punctuation(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_ascii_punctuation())
        .collect::<String>()
}

// https://gist.github.com/giuliano-oliveira/4d11d6b3bb003dba3a1b53f43d81b30d
async fn download_file(url: &str, path: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| anyhow!("Failed to GET from '{url}'"))?;
    let total_size = response
        .content_length()
        .ok_or_else(|| anyhow!("Failed to get content length from '{url}'"))?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));
    pb.set_message("Downloading...");

    if response.status() == reqwest::StatusCode::OK {
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut file = File::create(path).map_err(|_| anyhow!("Failed to create file '{path}'"))?;

        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|_| anyhow!("Error while downloading file"))?;
            file.write_all(&chunk)
                .map_err(|_| anyhow!("Error while writing to file"))?;
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new);
        }
        pb.finish_with_message("Finished download.");
    }
    Ok(())
}
