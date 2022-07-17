//! WIP attempt to digest etymologies from wiktextract data

mod etymology_templates;

use crate::etymology_templates::*;

use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};
use bytelines::ByteLines;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use hashbrown::{hash_map::OccupiedError, HashMap, HashSet};
use indicatif::{ProgressBar, ProgressStyle};
use simd_json::{to_borrowed_value, BorrowedValue, ValueAccess};
use string_interner::{backend::StringBackend, symbol::SymbolU32, StringInterner};

const WIKTEXTRACT_URL: &str = "https://kaikki.org/dictionary/raw-wiktextract-data.json.gz";
const WIKTEXTRACT_PATH: &str = "data/raw-wiktextract-data.json.gz";
// const WIKTEXTRACT_URL: &str = "http://0.0.0.0:8000/data/test/bank.json.gz";
// const WIKTEXTRACT_PATH: &str = "data/test/bank.json.gz";

// For etymological relationships given by DERIVED_TYPE_TEMPLATES
// and ABBREV_TYPE_TEMPLATES in etymology_templates.rs.
// Akin to Wikidata's derived from lexeme https://www.wikidata.org/wiki/Property:P5191
// and mode of derivation https://www.wikidata.org/wiki/Property:P5886
#[derive(Hash, Eq, PartialEq, Debug)]
struct DerivedFrom {
    item: Rc<Item>,
    mode: SymbolU32,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
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

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
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

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
enum RawEtyNode {
    RawDerivedFrom(RawDerivedFrom),
    RawCombines(RawCombines),
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
struct Item {
    term: SymbolU32,             // e.g. "bank"
    lang: SymbolU32,             // e.g "en", i.e. the wiktextract lang_code
    language: Option<SymbolU32>, // e.g. "English" i.e. the wiktextract lang
    ety_text: Option<SymbolU32>, // e.g. "From Middle English banke, from Middle French banque...
    pos: SymbolU32,              // e.g. "noun"
    gloss: Option<SymbolU32>,    // e.g. "An institution where one can place and borrow money...
    raw_ety_nodes: Option<Box<[RawEtyNode]>>,
}

type GlossMap = HashMap<Option<SymbolU32>, Rc<Item>>;
type PosMap = HashMap<SymbolU32, GlossMap>;
type EtyMap = HashMap<Option<SymbolU32>, PosMap>;
type LangMap = HashMap<SymbolU32, EtyMap>;

#[derive(Default)]
struct Items {
    term_map: HashMap<SymbolU32, LangMap>,
}

impl Items {
    fn add(&mut self, item: &Rc<Item>) -> Result<()> {
        // check if the item's term has been seen before
        if !self.term_map.contains_key(&item.term) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let mut lang_map = LangMap::new();
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(item.pos, gloss_map);
            ety_map.insert(item.ety_text, pos_map);
            lang_map.insert(item.lang, ety_map);
            self.term_map.insert(item.term, lang_map);
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
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(item.pos, gloss_map);
            ety_map.insert(item.ety_text, pos_map);
            lang_map.insert(item.lang, ety_map);
            return Ok(());
        }
        // since lang has been seen before, there must be at least one ety (possibly "")
        // check if this ety has been seen in this lang before
        let ety_map: &mut EtyMap = lang_map
            .get_mut(&item.lang)
            .ok_or_else(|| anyhow!("no EtyMap for lang when adding:\n{:#?}", item))?;
        if !ety_map.contains_key(&item.ety_text) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(item.pos, gloss_map);
            ety_map.insert(item.ety_text, pos_map);
            return Ok(());
        }
        // since ety has been seen before, there must be at least one pos
        // check if this pos has been seen for this ety before
        let pos_map: &mut PosMap = ety_map
            .get_mut(&item.ety_text)
            .ok_or_else(|| anyhow!("no PosMap for ety when adding:\n{:#?}", item))?;
        if !pos_map.contains_key(&item.pos) {
            let mut gloss_map = GlossMap::new();
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(item.pos, gloss_map);
            return Ok(());
        }
        // since pos has been seen before, there must be at least one gloss (possibly "")
        let gloss_map: &mut GlossMap = pos_map
            .get_mut(&item.pos)
            .ok_or_else(|| anyhow!("no GlossMap for pos when adding:\n{:#?}", item))?;
        if !gloss_map.contains_key(&item.gloss) {
            gloss_map.insert(item.gloss, Rc::clone(&item));
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
        match self.item_map.try_insert(Rc::clone(item), ety_node_opt) {
            Ok(_) => {}
            Err(OccupiedError { .. }) => {
                Err(anyhow!("Tried inserting duplicate item:\n{:#?}", item))?
            }
        }
        Ok(())
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
        let sense = Sense::new(string_pool, &item);
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
                            .into_iter()
                            .flat_map(|(_, pos_map)| {
                                pos_map.into_iter().flat_map(|(_, gloss_map)| {
                                    gloss_map.into_iter().map(|(_, other_item)| other_item)
                                })
                            })
                            .max_by_key(|other_item| {
                                let other_item_sense = Sense::new(string_pool, &other_item);
                                sense.lesk_score(&other_item_sense)
                            })
                        {
                            let node = EtyNode::DerivedFrom(DerivedFrom {
                                item: Rc::clone(&source_item),
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
            RawEtyNode::RawCombines(raw_combines) => {}
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
        Sense { gloss: gloss }
    }
    // https://en.wikipedia.org/wiki/Lesk_algorithm
    fn lesk_score(&self, other: &Sense) -> u32 {
        self.gloss.intersection(&other.gloss).count() as u32
    }
}

#[derive(Default)]
pub struct Processor {
    string_pool: StringPool,
    items: Items,
    sources: Sources,
}

impl Processor {
    // just for debugging
    pub fn print_all_items(&self) {
        for (term, lang_map) in self.items.term_map.iter() {
            println!("{}", self.string_pool.resolve(*term));
            for (lang, ety_map) in lang_map.iter() {
                println!("  {}", self.string_pool.resolve(*lang));
                for (ety_text, pos_map) in ety_map.iter() {
                    let et = ety_text
                        .and_then(|et| Some(self.string_pool.resolve(et)))
                        .unwrap_or_else(|| "");
                    println!("    {}", et);
                    for (pos, gloss_map) in pos_map.iter() {
                        println!("      {}", self.string_pool.resolve(*pos));
                        for (gloss, _) in gloss_map.iter() {
                            let g = gloss
                                .and_then(|g| Some(self.string_pool.resolve(g)))
                                .unwrap_or_else(|| "");
                            println!("        {}", g);
                        }
                    }
                }
            }
        }
    }

    fn process_derived_type_json_template(
        &mut self,
        template: &BorrowedValue,
        mode: &str,
        lang: SymbolU32,
    ) -> Option<RawEtyNode> {
        let args = template
            .get_object("args")
            .expect("get json ety template args");
        let term_lang = args
            .get("1")
            .expect("get derived-type json ety template term lang")
            .as_str()
            .expect("parse json ety template term lang as str");
        if term_lang != self.string_pool.resolve(lang) {
            return None;
        }
        let source_lang = args
            .get("2")
            .expect("get derived-type json ety template source lang")
            .as_str()
            .expect("parse json ety template source lang as str");
        let source_term_opt = args.get("3");
        match source_term_opt {
            Some(source_term) => {
                let source_term = source_term
                    .as_str()
                    .expect("parse json ety template source term as str");
                if source_term == "" || source_term == "-" {
                    return None;
                } else {
                    return Some(RawEtyNode::RawDerivedFrom(RawDerivedFrom {
                        source_term: self.string_pool.get_or_intern(clean_json_term(source_term)),
                        source_lang: self.string_pool.get_or_intern(source_lang),
                        mode: self.string_pool.get_or_intern(mode),
                    }));
                }
            }
            None => {
                return None;
            }
        }
    }

    fn process_abbrev_type_json_template(
        &mut self,
        template: &BorrowedValue,
        mode: &str,
        lang: SymbolU32,
    ) -> Option<RawEtyNode> {
        let args = template
            .get_object("args")
            .expect("get json ety template args");
        let term_lang = args
            .get("1")
            .expect("get abbrev-type json ety template term lang")
            .as_str()
            .expect("parse json ety template term lang as str");
        if term_lang != self.string_pool.resolve(lang) {
            return None;
        }
        let source_term_opt = args.get("2");
        match source_term_opt {
            Some(source_term) => {
                let source_term = source_term
                    .as_str()
                    .expect("parse json ety template source term as str");
                if source_term == "" || source_term == "-" {
                    return None;
                } else {
                    return Some(RawEtyNode::RawDerivedFrom(RawDerivedFrom {
                        source_term: self.string_pool.get_or_intern(clean_json_term(source_term)),
                        source_lang: lang.clone(),
                        mode: self.string_pool.get_or_intern(mode),
                    }));
                }
            }
            None => {
                return None;
            }
        }
    }

    fn process_compound_type_json_template(
        &mut self,
        template: &BorrowedValue,
        mode: &str,
        lang: SymbolU32,
    ) -> Option<RawEtyNode> {
        let args = template
            .get_object("args")
            .expect("get json ety template args");
        let term_lang = args
            .get("1")
            .expect("get compound-type json ety template term lang")
            .as_str()
            .expect("parse json ety template term lang as str");
        if term_lang != self.string_pool.resolve(lang) {
            return None;
        }

        let mut n = 2;
        let mut source_terms = Vec::new();
        let mut source_langs = Vec::new();
        let mut has_source_langs = false;
        while let Some(source_term_opt) = args.get(n.to_string().as_str()) {
            let source_term = source_term_opt
                .as_str()
                .expect("parse json ety template source term as str");
            if source_term == "" || source_term == "-" {
                break;
            }
            source_terms.push(self.string_pool.get_or_intern(clean_json_term(source_term)));
            if let Some(source_lang_opt) = args.get(format!("lang{n}").as_str()) {
                let source_lang = source_lang_opt
                    .as_str()
                    .expect("parse json ety template source lang as str");
                if source_lang == "" || source_lang == "-" {
                    break;
                }
                has_source_langs = true;
                source_langs.push(self.string_pool.get_or_intern(source_lang));
            } else {
                source_langs.push(lang.clone());
            }
            n += 1;
        }
        return (!source_terms.is_empty()).then(|| {
            RawEtyNode::RawCombines(RawCombines {
                source_terms: source_terms.into_boxed_slice(),
                source_langs: has_source_langs.then(|| source_langs.into_boxed_slice()),
                mode: self.string_pool.get_or_intern(mode),
            })
        });
    }

    fn process_json_ety_template(
        &mut self,
        template: &BorrowedValue,
        lang: SymbolU32,
    ) -> Option<RawEtyNode> {
        template.get_str("name").and_then(|name| {
            if DERIVED_TYPE_TEMPLATES.contains_key(name) {
                let mode = *DERIVED_TYPE_TEMPLATES.get(name).unwrap();
                self.process_derived_type_json_template(template, mode, lang)
            } else if ABBREV_TYPE_TEMPLATES.contains_key(name) {
                let mode = *ABBREV_TYPE_TEMPLATES.get(name).unwrap();
                self.process_abbrev_type_json_template(template, mode, lang)
            } else if COMPOUND_TYPE_TEMPLATES.contains_key(name) {
                let mode = *COMPOUND_TYPE_TEMPLATES.get(name).unwrap();
                self.process_compound_type_json_template(template, mode, lang)
            } else {
                None
            }
        })
    }

    fn process_json_ety_templates(
        &mut self,
        json_item: BorrowedValue,
        lang: SymbolU32,
    ) -> Option<Box<[RawEtyNode]>> {
        let raw_ety_nodes = json_item
            .get_array("etymology_templates")
            .and_then(|templates| {
                Some(
                    templates
                        .iter()
                        .map(|template| self.process_json_ety_template(template, lang))
                        .flatten() // only take the Some elements from the map
                        .collect::<Vec<RawEtyNode>>()
                        .into_boxed_slice(),
                )
            });

        // if no ety section or no templates, as a fallback we see if term
        // is listed as a "form_of" (item.senses[0].form_of[0].word)
        // or "alt_of" (item.senses[0].alt_of[0].word) another term.
        // e.g. "happenin'" is listed as an alt_of of "happening".
        if raw_ety_nodes.is_none() || raw_ety_nodes.as_ref().unwrap().is_empty() {
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
                        source_lang: lang.clone(),
                        mode: self.string_pool.get_or_intern("form of"),
                    });
                    let raw_ety_nodes = vec![raw_ety_node];
                    return Some(raw_ety_nodes.into_boxed_slice());
                }
                None => {
                    return None;
                }
            }
        }
        raw_ety_nodes
    }

    fn process_json_item(&mut self, json_item: BorrowedValue) -> Result<()> {
        // some wiktionary pages are redirects, which we don't want
        // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
        if json_item.contains_key("redirect") {
            return Ok(());
        }
        // 'word' field must be present
        let term =
            self.string_pool
                .get_or_intern(json_item.get_str("word").ok_or_else(|| {
                    anyhow!("failed parsing 'word' field in json item:\n{json_item}")
                })?);
        // 'lang_code' field must be present
        let lang = self
            .string_pool
            .get_or_intern(json_item.get_str("lang_code").ok_or_else(|| {
                anyhow!("failed parsing 'lang_code' field in json item:\n{json_item}")
            })?);
        // 'lang' field may be missing, in which case we return None
        let language = json_item
            .get_str("lang")
            .and_then(|s| Some(self.string_pool.get_or_intern(s)));
        // 'etymology_text' field may be missing, in which case we return None
        let ety_text = json_item
            .get_str("etymology_text")
            .and_then(|s| Some(self.string_pool.get_or_intern(s)));
        // 'pos' field must be present
        let pos = self.string_pool.get_or_intern(
            json_item
                .get_str("pos")
                .ok_or_else(|| anyhow!("failed parsing 'pos' field in json item:\n{json_item}"))?,
        );
        // 'senses' field should always be present and non-empty, but glosses
        // may be missing or empty. We just return None if gloss not found
        // for any reason.
        // $$ For reconstructed terms, what should really be different entries
        // $$ are muddled together as different senses in the same entry.
        // $$ Need to implement adding different items for each sense for reconstructed terms.
        let gloss = json_item
            .get_array("senses")
            .and_then(|senses| senses.get(0))
            .and_then(|sense| sense.get_array("glosses"))
            .and_then(|glosses| glosses.get(0))
            .and_then(|gloss| gloss.as_str())
            .and_then(|s| Some(self.string_pool.get_or_intern(s)));

        let raw_ety_nodes = self.process_json_ety_templates(json_item, lang);

        let item = Rc::from(Item {
            term: term,
            lang: lang,
            language: language,
            ety_text: ety_text,
            pos: pos,
            gloss: gloss,
            raw_ety_nodes: raw_ety_nodes,
        });
        self.items.add(&item)?;
        Ok(())
    }

    fn process_json_items<T: BufRead>(&mut self, lines: ByteLines<T>) -> Result<()> {
        lines
            .into_iter()
            .filter_map(Result::ok)
            .for_each(|mut line| {
                let json_item: BorrowedValue =
                    to_borrowed_value(&mut line).expect("parse json line from file");
                self.process_json_item(json_item)
                    .expect("process json item");
            });
        // self.print_all_items();
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
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
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

    pub async fn process_wiktextract_data(&mut self) -> Result<()> {
        let file = match File::open(WIKTEXTRACT_PATH) {
            Ok(file) => {
                println!("Processing data from local file {WIKTEXTRACT_PATH}");
                file
            }
            Err(_) => {
                // file doesn't exist or error opening it; download it
                println!("No local file found, downloading from {WIKTEXTRACT_URL}");
                download_file(WIKTEXTRACT_URL, WIKTEXTRACT_PATH).await?;
                let file = File::open(WIKTEXTRACT_PATH)
                    .or_else(|_| Err(anyhow!("Failed to open file '{WIKTEXTRACT_PATH}'")))?;
                println!("Processing data from downloaded file {WIKTEXTRACT_PATH}");
                file
            }
        };
        self.process_file(file)?;
        println!("Finished processing {WIKTEXTRACT_PATH}");
        self.process_items()?;

        Ok(())
    }
}

fn clean_json_term(term: &str) -> &str {
    // In wiktextract json, reconstructed terms (e.g. PIE) start with "*",
    // except for when they are values of a json item's 'word' field, where they
    // seem to be cleaned already. Since we will be trying to match to terms
    // taken from 'word' fields, we need to clean the terms when they do start
    // with "*".
    if term.starts_with("*") {
        &term[1..]
    } else {
        term
    }
}

fn remove_punctuation(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_ascii_punctuation())
        .collect::<String>()
}

// https://gist.github.com/giuliano-oliveira/4d11d6b3bb003dba3a1b53f43d81b30d
pub async fn download_file(url: &str, path: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .or_else(|_| Err(anyhow!("Failed to GET from '{url}'")))?;
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
        let mut file =
            File::create(path).or_else(|_| Err(anyhow!("Failed to create file '{path}'")))?;

        while let Some(item) = stream.next().await {
            let chunk = item.or_else(|_| Err(anyhow!("Error while downloading file")))?;
            file.write_all(&chunk)
                .or_else(|_| Err(anyhow!("Error while writing to file")))?;
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new);
        }
        pb.finish_with_message("Finished download.");
    }
    Ok(())
}
