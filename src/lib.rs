use std::io::{BufReader, BufRead};
use std::collections::HashMap;
use std::error::Error;
use std::cmp::min;
use std::fs::File;
use std::io::Write;

use flate2::read::GzDecoder;
use futures_util::StreamExt;
use bytelines::ByteLines;
use simd_json::{BorrowedValue, to_borrowed_value, ValueAccess};
use indicatif::{ProgressBar, ProgressStyle};
use string_interner::{StringInterner, symbol::SymbolU32};

const WIKTEXTRACT_URL: &str = "https://kaikki.org/dictionary/raw-wiktextract-data.json.gz";
const WIKTEXTRACT_PATH: &str = "data/raw-wiktextract-data.json.gz";
// const WIKTEXTRACT_URL: &str = "http://0.0.0.0:8000/data/test/bank.json.gz";
// const WIKTEXTRACT_PATH: &str = "data/test/bank.json.gz";

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Item {
    term: SymbolU32, // e.g. "bank"
    lang: SymbolU32, // e.g "English"
    ety_text: SymbolU32, // e.g. "From Middle English banke, from Middle French banque...
    pos: SymbolU32, // e.g. "noun"
    gloss: SymbolU32 // e.g. "An institution where one can place and borrow money...
}

type GlossMap = HashMap<SymbolU32, Item>;
type PosMap = HashMap<SymbolU32, GlossMap>; 
type EtyMap = HashMap<SymbolU32, PosMap>;
type LangMap = HashMap<SymbolU32, EtyMap>;
type TermMap = HashMap<SymbolU32, LangMap>;

#[derive(Default)]
pub struct Processor {
    term_map: TermMap,
    string_pool: StringInterner
}

impl Processor {
    fn add_item(&mut self, item: Item) -> Result<(), Box<dyn Error>> {
        let term = item.term.clone();
        let lang = item.lang.clone();
        let ety_text = item.ety_text.clone();
        let pos = item.pos.clone();
        let gloss = item.gloss.clone();
        // check if the item's term has been seen before
        if !self.term_map.contains_key(&term) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map =  PosMap::new();
            let mut ety_map =  EtyMap::new();
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
        let lang_map: &mut LangMap = self.term_map.get_mut(&term)
            .ok_or_else(|| format!("no LangMap for term when adding:\n{:#?}", item))?;
        if !lang_map.contains_key(&lang) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map =  PosMap::new();
            let mut ety_map =  EtyMap::new();
            gloss_map.insert(gloss, item);
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text, pos_map);
            lang_map.insert(lang, ety_map);
            return Ok(());
        }
        // since lang has been seen before, there must be at least one ety
        // check if this ety has been seen in this lang before
        let ety_map: &mut EtyMap = lang_map.get_mut(&lang)
            .ok_or_else(|| format!("no EtyMap for lang when adding:\n{:#?}", item))?;
        if !ety_map.contains_key(&ety_text) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map =  PosMap::new();
            gloss_map.insert(gloss, item);
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_text, pos_map);
            return Ok(());
        }
        // since ety has been seen before, there must be at least one pos
        // check if this pos has been seen for this ety before
        let pos_map: &mut PosMap = ety_map.get_mut(&ety_text)
            .ok_or_else(|| format!("no PosMap for ety when adding:\n{:#?}", item))?;
        if !pos_map.contains_key(&pos) {
            let mut gloss_map = GlossMap::new();
            gloss_map.insert(gloss, item);
            pos_map.insert(pos, gloss_map);
            return Ok(());
        }
        // since pos has been seen before, there must be at least one gloss (possibly "")
        let gloss_map: &mut GlossMap = pos_map.get_mut(&pos)
            .ok_or_else(|| format!("no GlossMap for pos when adding:\n{:#?}", item))?;
        if !gloss_map.contains_key(&gloss) {
            gloss_map.insert(gloss, item);
            return Ok(());
        }
        Ok(())
    }
    
    // just for debugging, so unwraps are fine
    pub fn print_all_items(&self) {
        for (term, lang_map) in self.term_map.iter() {
            println!("{}", self.string_pool.resolve(*term).unwrap());
            for (lang, ety_map) in lang_map.iter() {
                println!("  {}", self.string_pool.resolve(*lang).unwrap());
                for (ety_text, pos_map) in ety_map.iter() {
                    println!("    {}", self.string_pool.resolve(*ety_text).unwrap());
                    for (pos, gloss_map) in pos_map.iter() {
                        println!("      {}", self.string_pool.resolve(*pos).unwrap());
                        for (gloss, _) in gloss_map.iter() {
                            println!("        {}", self.string_pool.resolve(*gloss).unwrap());
                        }
                    }
                }
            }
        }
    }
    
    fn process_json_item(&mut self, json_item: BorrowedValue) -> Result<(), Box<dyn Error>> {
        let term: SymbolU32 = self.string_pool.get_or_intern(json_item
            .get("word")
            .ok_or_else(|| format!("json item has no 'word' field:\n{json_item}"))?
            .as_str()
            .ok_or_else(|| format!("failed parsing str for 'word' in json item:\n{json_item}"))?
        );
        let lang: SymbolU32 = self.string_pool.get_or_intern(json_item
            .get("lang")
            .ok_or_else(|| format!("json item has no 'lang' field:\n{json_item}"))?
            .as_str()
            .ok_or_else(|| format!("failed parsing str for 'lang' in json item:\n{json_item}"))?
        );
        let ety_text: SymbolU32 = self.string_pool.get_or_intern(json_item
            .get("etymology_text")
            .map_or_else(|| Some(""), |v| v.as_str())
            .ok_or_else(|| format!("failed parsing str for 'etymology_text' in json item:\n{json_item}"))?
        );
        let pos: SymbolU32 = self.string_pool.get_or_intern(json_item
            .get("pos")
            .ok_or_else(|| format!("json item has no 'pos' field:\n{json_item}"))?
            .as_str()
            .ok_or_else(|| format!("failed parsing str for 'pos' in json item:\n{json_item}"))?
        );
        let gloss: SymbolU32 = self.string_pool.get_or_intern(json_item["senses"][0] // we check this is safe in is_valid_json_item()
            .get("glosses")
            .map_or_else(|| Ok(""), |v| v
                .get_idx(0)
                .ok_or_else(|| format!("'senses[0].glosses' is empty in json item:\n{json_item}"))?
                .as_str()
                .ok_or_else(|| format!("failed parsing str for 'gloss' in json item:\n{json_item}"))
            )?
        );    
    
        let item = Item {
            term: term,
            lang: lang,
            ety_text: ety_text,
            pos: pos,
            gloss: gloss
        };
        self.add_item(item)?;
        Ok(())
    }
    
    fn process_json_items<T: BufRead>(&mut self, lines: ByteLines<T>) -> Result<(), Box<dyn Error>> {
        lines
            .into_iter()
            .filter_map(Result::ok)
            .for_each(|mut line| {
                let json_item: BorrowedValue = to_borrowed_value(&mut line)
                    .expect("parse json line from file");
                if is_valid_json_item(&json_item) {
                    self.process_json_item(json_item)
                    .expect("process json item");
                }
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
            Err(_) => { // file doesn't exist or error opening it; download it
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

fn is_valid_json_item(json_item: &BorrowedValue) -> bool {
    // some wiktionary pages are redirects, which we don't want
    // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
    !json_item.contains_key("redirect") &&
    // as of 2022-06-20, there is exactly one json_item that has no senses
    // https://github.com/tatuylonen/wiktextract/issues/139
    json_item
        .get("senses")
        .ok_or_else(|| format!("json item has no 'senses' field:\n{json_item}"))
        .expect("assume canonical fields in json") 
        .get_idx(0)
        .is_some()
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
        let mut file = File::create(path)
            .or_else(|_| Err(format!("Failed to create file '{path}'")))?;

        while let Some(item) = stream.next().await {
            let chunk = item.or_else(|_| Err(format!("Error while downloading file")))?;
            file.write_all(&chunk).or_else(|_| Err(format!("Error while writing to file")))?;
            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new);
        }
        pb.finish_with_message("Finished download.");
    }
    Ok(())
}