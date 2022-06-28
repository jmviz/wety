use std::io::{BufReader, BufRead};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::cmp::min;
use std::fs::File;
use std::io::Write;

use flate2::read::GzDecoder;
use futures_util::StreamExt;
use bytelines::ByteLines;
use simd_json::{BorrowedValue, to_borrowed_value, ValueAccess};
use indicatif::{ProgressBar, ProgressStyle};

const WIKTEXTRACT_URL: &str = "https://kaikki.org/dictionary/raw-wiktextract-data.json.gz";
const WIKTEXTRACT_PATH: &str = "data/raw-wiktextract-data.json.gz";
// const WIKTEXTRACT_URL: &str = "http://0.0.0.0:8000/data/test/bank.json.gz";
// const WIKTEXTRACT_PATH: &str = "data/test/bank.json.gz";

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Item {
    term: Box<str>, // e.g. "bank"
    lang: Box<str>, // e.g "English"
    ety_text: Box<str>, // e.g. "From Middle English banke, from Middle French banque...
    pos: Box<str>, // e.g. "noun"
    gloss: Box<str> // e.g. "An institution where one can place and borrow money...
}

type ItemSet = HashSet<Item>;
type EtyMap = HashMap<Box<str>, ItemSet>;
type LangMap = HashMap<Box<str>, EtyMap>;
type TermMap = HashMap<Box<str>, LangMap>;

fn add_item(term_map: &mut TermMap, item: Item) -> Result<(), Box<dyn Error>> {
    // check if the item's term has been seen before
    if !term_map.contains_key(&item.term) {
        let mut item_set: ItemSet = HashSet::new();
        let mut ety_map: EtyMap = HashMap::new();
        let mut lang_map: LangMap = HashMap::new();
        let term = item.term.clone();
        let lang = item.lang.clone();
        let ety_text = item.ety_text.clone();
        item_set.insert(item);
        ety_map.insert(ety_text, item_set);
        lang_map.insert(lang, ety_map);
        term_map.insert(term, lang_map);
        return Ok(());
    }
    // since term has been seen before, there must be at least one lang for it
    // check if item's lang has been seen before
    let lang_map: &mut LangMap = term_map.get_mut(&item.term)
        .ok_or_else(|| format!("no LangMap for term when adding:\n{:#?}", item))?;
    if !lang_map.contains_key(&item.lang) {
        let mut item_set: ItemSet = HashSet::new();
        let mut ety_map: EtyMap = HashMap::new();
        let ety_text = item.ety_text.clone();
        let lang = item.lang.clone();
        item_set.insert(item);
        ety_map.insert(ety_text, item_set);
        lang_map.insert(lang, ety_map);
        return Ok(());
    }
    // since lang has been seen before, there must be at least one ety
    // and for any ety, there must be at least one item
    let ety_map: &mut EtyMap = lang_map.get_mut(&item.lang)
        .ok_or_else(|| format!("no EtyMap for lang when adding:\n{:#?}", item))?;
    if ety_map.contains_key(&item.ety_text) {
        let item_set: &mut ItemSet = ety_map.get_mut(&item.ety_text)
            .ok_or_else(|| format!("no ItemSet for ety_text when adding:\n{:#?}", item))?;
        item_set.insert(item);
    } else {
        let mut item_set: ItemSet = HashSet::new();
        let ety_text = item.ety_text.clone();
        item_set.insert(item);
        ety_map.insert(ety_text, item_set);
    }
    Ok(())
}

pub fn print_all_items(term_map: &TermMap) {
    for (term, lang_map) in term_map.iter() {
        println!("{term}");
        for (lang, ety_map) in lang_map.iter() {
            println!("  {lang}");
            for (ety_text, item_set) in ety_map.iter() {
                println!("    {ety_text}");
                for item in item_set.iter() {
                    println!("      ({}) {}", item.pos, item.gloss);
                }
            }
        }
    }
}

pub fn print_item(item: &Item) {
    println!("{}", item.term);
    println!("  {}", item.lang);
    println!("    {}", item.ety_text);
    println!("      ({}), {}", item.pos, item.gloss);
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

fn process_json_item(term_map: &mut TermMap, json_item: BorrowedValue) -> Result<(), Box<dyn Error>> {
    let term: Box<str> = Box::from(
        json_item
            .get("word")
            .ok_or_else(|| format!("json item has no 'word' field:\n{json_item}"))?
            .as_str()
            .ok_or_else(|| format!("failed parsing str for 'word' in json item:\n{json_item}"))?
);
    let lang: Box<str> = Box::from(
        json_item
            .get("lang")
            .ok_or_else(|| format!("json item has no 'lang' field:\n{json_item}"))?
            .as_str()
            .ok_or_else(|| format!("failed parsing str for 'lang' in json item:\n{json_item}"))?
);
    let ety_text: Box<str> = Box::from(
        json_item
            .get("etymology_text")
            .map_or_else(|| Some(""), |v| v.as_str())
            .ok_or_else(|| format!("failed parsing str for 'etymology_text' in json item:\n{json_item}"))?
);
    let pos: Box<str> = Box::from(
        json_item
            .get("pos")
            .ok_or_else(|| format!("json item has no 'pos' field:\n{json_item}"))?
            .as_str()
            .ok_or_else(|| format!("failed parsing str for 'pos' in json item:\n{json_item}"))?
);
    let gloss: Box<str> = Box::from(
        json_item["senses"][0] // we check this is safe in is_valid_json_item()
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
    // print_item(&item);
    add_item(term_map, item)?;
    Ok(())
}

fn process_json_items<T: BufRead>(lines: ByteLines<T>) -> Result<TermMap, Box<dyn Error>> {
    let mut term_map: TermMap = HashMap::new();
    lines
        .into_iter()
        .filter_map(Result::ok)
        .for_each(|mut line| {
            let json_item: BorrowedValue = to_borrowed_value(&mut line)
                .expect("parse json line from file");
            if is_valid_json_item(&json_item) {
                process_json_item(&mut term_map, json_item)
                .expect("process json item");
            }
        });
    println!("Finished initial processing of wiktextract raw data");
    // print_all_items(&term_map);
    Ok(term_map)
}

fn process_file(file: File) -> Result<TermMap, Box<dyn Error>> {
    let reader = BufReader::new(file);
    let gz = GzDecoder::new(reader);
    let gz_reader = BufReader::new(gz);
    let lines = ByteLines::new(gz_reader);
    Ok(process_json_items(lines)?)
}

pub async fn download_file(url: &str, path: &str) -> Result<(), Box<dyn Error>> {
    // https://gist.github.com/giuliano-oliveira/4d11d6b3bb003dba3a1b53f43d81b30d
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

pub async fn process_wiktextract_data() -> Result<TermMap, Box<dyn Error>> {
    match File::open(WIKTEXTRACT_PATH) {
        Ok(file) => {
            println!("Processing data from local file {WIKTEXTRACT_PATH}");
            Ok(process_file(file)?)
        }
        Err(_) => { // file doesn't exist or error opening it; download it
            println!("No local file found, downloading from {WIKTEXTRACT_URL}");
            download_file(WIKTEXTRACT_URL, WIKTEXTRACT_PATH).await?;
            let file = File::open(WIKTEXTRACT_PATH)
                .or_else(|_| Err(format!("Failed to open file '{WIKTEXTRACT_PATH}'")))?;
            println!("Processing data from downloaded file {WIKTEXTRACT_PATH}");
            Ok(process_file(file)?)
        }
    }
}