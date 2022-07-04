//! WIP attempt to digest etymologies from wiktextract data

mod etymology_templates;

use crate::etymology_templates::*;

use std::cmp::min;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::rc::Rc;

use bytelines::ByteLines;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use simd_json::{to_borrowed_value, BorrowedValue, ValueAccess};
use string_interner::{symbol::SymbolU32, StringInterner};

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
pub struct Item {
    term: SymbolU32,                // e.g. "bank"
    lang: SymbolU32,                // e.g "English"
    ety_text: Option<SymbolU32>,            // e.g. "From Middle English banke, from Middle French banque...
    pos: SymbolU32,                 // e.g. "noun"
    gloss: Option<SymbolU32>,               // e.g. "An institution where one can place and borrow money...
    raw_ety_nodes: Option<Box<[RawEtyNode]>>,
    source: Option<EtyNode>,         // e.g. a Some(DerivedFrom) with Rc<Item> for M.E. "banke" and mode = "inherited" 
    source_of: Option<Vec<EtyNode>>,
}    

type GlossMap = HashMap<Option<SymbolU32>, Rc<Item>>;
type PosMap = HashMap<SymbolU32, GlossMap>;
type EtyMap = HashMap<Option<SymbolU32>, PosMap>;
type LangMap = HashMap<SymbolU32, EtyMap>;
type TermMap = HashMap<SymbolU32, LangMap>;

#[derive(Default)]
pub struct Processor {
    term_map: TermMap,
    string_pool: StringInterner,
}

impl Processor {
    fn add_item(&mut self, item: Rc<Item>) -> Result<(), Box<dyn Error>> {
        let term = item.term.clone();
        let lang = item.lang.clone();
        let ety_text = item.ety_text.clone();
        let pos = item.pos.clone();
        let gloss = item.gloss.clone();
        // check if the item's term has been seen before
        if !self.term_map.contains_key(&term) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let mut lang_map = LangMap::new();
            gloss_map.insert(gloss, item);
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text, pos_map);
            lang_map.insert(lang, ety_map);
            self.term_map.insert(term, lang_map);
            return Ok(());
        }
        // since term has been seen before, there must be at least one lang for it
        // check if item's lang has been seen before
        let lang_map: &mut LangMap = self
            .term_map
            .get_mut(&term)
            .ok_or_else(|| format!("no LangMap for term when adding:\n{:#?}", item))?;
        if !lang_map.contains_key(&lang) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            gloss_map.insert(gloss, item);
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text, pos_map);
            lang_map.insert(lang, ety_map);
            return Ok(());
        }
        // since lang has been seen before, there must be at least one ety (possibly "")
        // check if this ety has been seen in this lang before
        let ety_map: &mut EtyMap = lang_map
            .get_mut(&lang)
            .ok_or_else(|| format!("no EtyMap for lang when adding:\n{:#?}", item))?;
        if !ety_map.contains_key(&ety_text) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            gloss_map.insert(gloss, item);
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text, pos_map);
            return Ok(());
        }
        // since ety has been seen before, there must be at least one pos
        // check if this pos has been seen for this ety before
        let pos_map: &mut PosMap = ety_map
            .get_mut(&ety_text)
            .ok_or_else(|| format!("no PosMap for ety when adding:\n{:#?}", item))?;
        if !pos_map.contains_key(&pos) {
            let mut gloss_map = GlossMap::new();
            gloss_map.insert(gloss, item);
            pos_map.insert(pos, gloss_map);
            return Ok(());
        }
        // since pos has been seen before, there must be at least one gloss (possibly "")
        let gloss_map: &mut GlossMap = pos_map
            .get_mut(&pos)
            .ok_or_else(|| format!("no GlossMap for pos when adding:\n{:#?}", item))?;
        if !gloss_map.contains_key(&gloss) {
            gloss_map.insert(gloss, item);
            return Ok(());
        }
        Ok(())
    }

    // just for debugging
    pub fn print_all_items(&self) {
        for (term, lang_map) in self.term_map.iter() {
            println!("{}", self.string_pool.resolve(*term).unwrap());
            for (lang, ety_map) in lang_map.iter() {
                println!("  {}", self.string_pool.resolve(*lang).unwrap());
                for (ety_text, pos_map) in ety_map.iter() {
                    let et = ety_text
                        .and_then(|et| Some(self.string_pool.resolve(et).unwrap()))
                        .unwrap_or("");
                    println!("    {}", et);
                    for (pos, gloss_map) in pos_map.iter() {
                        println!("      {}", self.string_pool.resolve(*pos).unwrap());
                        for (gloss, _) in gloss_map.iter() {
                            let g = gloss
                                .and_then(|g| Some(self.string_pool.resolve(g).unwrap()))
                                .unwrap_or("");
                            println!("        {}", g);
                        }
                    }
                }
            }
        }
    }

    // deal with case where there is a valid chain of derivs but there is a term amid
    // it that doesn't have an item entry, while a subsequent term in the chain does.
    fn process_item_ety_list() {}

    fn process_derived_type_json_template(
        &mut self, 
        template: &BorrowedValue,
        mode: &str,
        lang: &SymbolU32
    ) -> Option<RawEtyNode> {
        let args = template.get_object("args").expect("get json ety template args");
        let term_lang = 
            args
                .get("1").expect("get derived-type json ety template term lang")
                .as_str().expect("parse json ety template term lang as str");
        if term_lang != self.string_pool.resolve(*lang).unwrap() {
            return None;
        }
        let source_lang = 
            args
                .get("2").expect("get derived-type json ety template source lang")
                .as_str().expect("parse json ety template source lang as str");
        let source_term_opt = args.get("3");
        match source_term_opt {
            Some(source_term) => {
                let source_term = source_term.as_str().expect("parse json ety template source term as str");
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
        lang: &SymbolU32,
    ) -> Option<RawEtyNode> {
        let args = template.get_object("args").expect("get json ety template args");
        let term_lang = 
            args
                .get("1").expect("get abbrev-type json ety template term lang")
                .as_str().expect("parse json ety template term lang as str");
        if term_lang != self.string_pool.resolve(*lang).unwrap() {
            return None;
        }
        let source_term_opt = args.get("2");
        match source_term_opt {
            Some(source_term) => {
                let source_term = source_term.as_str().expect("parse json ety template source term as str");
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
        lang: &SymbolU32
    ) -> Option<RawEtyNode> {
        let args = template.get_object("args").expect("get json ety template args");
        let term_lang = 
            args
                .get("1").expect("get compound-type json ety template term lang")
                .as_str().expect("parse json ety template term lang as str");
        if term_lang != self.string_pool.resolve(*lang).unwrap() {
            return None;
        }
        
        let mut n = 2;
        let mut source_terms = Vec::new();
        let mut source_langs = Vec::new();
        let mut has_source_langs = false;
        while let Some(source_term_opt) = args.get(n.to_string().as_str()) {
            let source_term = source_term_opt.as_str().expect("parse json ety template source term as str");
            if source_term == "" || source_term == "-" {
                break;
            }
            source_terms.push(self.string_pool.get_or_intern(clean_json_term(source_term)));
            if let Some(source_lang_opt) = args.get(format!("lang{n}").as_str()) {
                let source_lang = source_lang_opt.as_str().expect("parse json ety template source lang as str");
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
        if source_terms.is_empty() {
            return None;
        }
        let source_langs_opt;
        if has_source_langs {
            source_langs_opt = Some(source_langs.into_boxed_slice());
        } else {
            source_langs_opt = None;
        }
        return Some(RawEtyNode::RawCombines(RawCombines {
            source_terms: source_terms.into_boxed_slice(),
            source_langs: source_langs_opt,
            mode: self.string_pool.get_or_intern(mode),
        }));
    }

    fn process_json_ety_template(
        &mut self, 
        template: &BorrowedValue,
        lang: &SymbolU32,
    ) -> Option<RawEtyNode> {
        match template.get_str("name") {
            Some(name) => {
                if DERIVED_TYPE_TEMPLATES.contains_key(name) {
                    let mode = *DERIVED_TYPE_TEMPLATES.get(name).unwrap();
                    return self.process_derived_type_json_template(template, mode, lang);
                } else if ABBREV_TYPE_TEMPLATES.contains_key(name) {
                    let mode = *ABBREV_TYPE_TEMPLATES.get(name).unwrap();
                    return self.process_abbrev_type_json_template(template, mode, lang);
                } else if COMPOUND_TYPE_TEMPLATES.contains_key(name) {
                    let mode = *COMPOUND_TYPE_TEMPLATES.get(name).unwrap();
                    return self.process_compound_type_json_template(template, mode, lang);
                } else {
                    return None;
                }
            }
            None => {
                return None;
            },
        }
    }

    fn process_json_ety_templates(
        &mut self, 
        json_item: BorrowedValue,
        lang: &SymbolU32
    ) -> Option<Box<[RawEtyNode]>> { 
        let raw_ety_nodes = 
            json_item
                .get_array("etymology_templates")
                .and_then(|templates|
                    Some(templates
                        .iter()
                        .map(|template| self.process_json_ety_template(template, lang))
                        .flatten() // only take the Some elements from the map
                        .collect::<Vec<RawEtyNode>>().into_boxed_slice()));
        
        // if no ety section or no templates, as a fallback we see if term
        // is listed as a "form_of" (item.senses[0].form_of[0].word)
        // or "alt_of" (item.senses[0].alt_of[0].word) another term.
        // e.g. "happenin'" is listed as an alt_of of "happening".
        if raw_ety_nodes.is_none() || raw_ety_nodes.as_ref().unwrap().is_empty() {
            let alt_term = json_item
                .get_array("senses")
                .and_then(|senses| senses.get(0))
                .and_then(|sense| sense
                    .get_array("alt_of")
                    .or_else(|| sense.get_array("form_of")))
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
                },
                None => {
                    return None;
                },
            }
        }
        raw_ety_nodes
    }

    fn process_json_item(&mut self, json_item: BorrowedValue) -> Result<(), Box<dyn Error>> {
        // some wiktionary pages are redirects, which we don't want
        // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
        if json_item.contains_key("redirect") {
            return Ok(());
        }
        // 'word' field must be present
        let term = self.string_pool.get_or_intern(
            json_item
                .get_str("word")
                .ok_or_else(|| format!("failed parsing 'word' field in json item:\n{json_item}"))?,
        );
        // 'lang_code' field must be present
        let lang = self.string_pool.get_or_intern(
            json_item
                .get_str("lang_code")
                .ok_or_else(|| format!("failed parsing 'lang' field in json item:\n{json_item}"))?,
        );
        // 'etymology_text' field may be missing, in which case we return None
        let ety_text =
            json_item
                .get_str("etymology_text")
                .and_then(|str| Some(self.string_pool.get_or_intern(str)));
        // 'pos' field must be present
        let pos = self.string_pool.get_or_intern(
            json_item
                .get_str("pos")
                .ok_or_else(|| format!("failed parsing 'pos' field in json item:\n{json_item}"))?,
        );
        // 'senses' field should always be present and non-empty, but glosses
        // may be missing or empty. We just return None if gloss not found 
        // for any reason.
        // $$ For reconstructed terms, what should really be different entries
        // $$ are muddled together as different senses in the same entry.
        // $$ Need to implement adding different items for each sense for reconstructed terms.
        let gloss = 
            json_item
                .get_array("senses")
                .and_then(|senses| senses.get(0))
                .and_then(|sense| sense.get_array("glosses"))
                .and_then(|glosses| glosses.get(0))
                .and_then(|gloss| gloss.as_str())
                .and_then(|str| Some(self.string_pool.get_or_intern(str)));

        let raw_ety_nodes = self.process_json_ety_templates(json_item, &lang);


        let item = Rc::from(Item {
            term: term,
            lang: lang,
            ety_text: ety_text,
            pos: pos,
            gloss: gloss,
            raw_ety_nodes: raw_ety_nodes,
            source: None,
            source_of: None,
        });
        self.add_item(item)?;
        Ok(())
    }

    fn process_json_items<T: BufRead>(
        &mut self,
        lines: ByteLines<T>,
    ) -> Result<(), Box<dyn Error>> {
        lines
            .into_iter()
            .filter_map(Result::ok)
            .for_each(|mut line| {
                let json_item: BorrowedValue =
                    to_borrowed_value(&mut line).expect("parse json line from file");
                self.process_json_item(json_item).expect("process json item");
            });
        println!("Finished initial processing of wiktextract raw data");
        // self.print_all_items();
        Ok(())
    }

    fn process_file(&mut self, file: File) -> Result<(), Box<dyn Error>> {
        let reader = BufReader::new(file);
        let gz = GzDecoder::new(reader);
        let gz_reader = BufReader::new(gz);
        let lines = ByteLines::new(gz_reader);
        self.process_json_items(lines)?;
        Ok(())
    }

    pub async fn process_wiktextract_data(&mut self) -> Result<(), Box<dyn Error>> {
        match File::open(WIKTEXTRACT_PATH) {
            Ok(file) => {
                println!("Processing data from local file {WIKTEXTRACT_PATH}");
                self.process_file(file)?;
                Ok(())
            }
            Err(_) => {
                // file doesn't exist or error opening it; download it
                println!("No local file found, downloading from {WIKTEXTRACT_URL}");
                download_file(WIKTEXTRACT_URL, WIKTEXTRACT_PATH).await?;
                let file = File::open(WIKTEXTRACT_PATH)
                    .or_else(|_| Err(format!("Failed to open file '{WIKTEXTRACT_PATH}'")))?;
                println!("Processing data from downloaded file {WIKTEXTRACT_PATH}");
                self.process_file(file)?;
                Ok(())
            }
        }
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

// https://gist.github.com/giuliano-oliveira/4d11d6b3bb003dba3a1b53f43d81b30d
pub async fn download_file(url: &str, path: &str) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .or_else(|_| Err(format!("Failed to GET from '{url}'")))?;
    let total_size = response
        .content_length()
        .ok_or_else(|| format!("Failed to get content length from '{url}'"))?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));
    pb.set_message("Downloading...");

    if response.status() == reqwest::StatusCode::OK {
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut file =
            File::create(path).or_else(|_| Err(format!("Failed to create file '{path}'")))?;

        while let Some(item) = stream.next().await {
            let chunk = item.or_else(|_| Err(format!("Error while downloading file")))?;
            file.write_all(&chunk)
                .or_else(|_| Err(format!("Error while writing to file")))?;
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new);
        }
        pb.finish_with_message("Finished download.");
    }
    Ok(())
}
