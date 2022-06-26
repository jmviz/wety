use std::io::{self, Read};
use std::collections::{HashMap, HashSet};

use flate2::read::GzDecoder;
use reqwest::Response;
use futures_util::StreamExt;
use std::io::{BufReader, BufRead};
use bytelines::ByteLines;
use simd_json::{BorrowedValue, to_borrowed_value, ValueAccess};
use std::fs::File;

// const WIKTEXTRACT_URL: &str = "https://kaikki.org/dictionary/raw-wiktextract-data.json.gz";
const WIKTEXTRACT_URL: &str = "http://0.0.0.0:8000/test/bank.json.gz";
// const WIKTEXTRACT_PATH: &str = "data/raw-wiktextract-data.json.gz";
const WIKTEXTRACT_PATH: &str = "data/test/bank.json.gz";

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Item {
    term: String, // e.g. "bank"
    lang: String, // e.g "English"
    pos: String, // e.g. "noun"
    ety_text: String, // e.g. "From Middle English banke, from Middle French banque...
    gloss: String // e.g. "An institution where one can place and borrow money...
}

type ItemSet = HashSet<Item>;
type EtyMap = HashMap<String, ItemSet>;
type LangMap = HashMap<String, EtyMap>;
type TermMap = HashMap<String, LangMap>;

fn add_item(term_map: &mut TermMap, item: Item) {
    let term = item.term.clone();
    let lang = item.lang.clone();
    let ety_text = item.ety_text.clone();
    // check if the item's term has been seen before
    if !term_map.contains_key(&item.term) {
        let mut item_set: ItemSet = HashSet::new();
        let mut ety_map: EtyMap = HashMap::new();
        let mut lang_map: LangMap = HashMap::new();
        item_set.insert(item);
        ety_map.insert(ety_text, item_set);
        lang_map.insert(lang, ety_map);
        term_map.insert(term, lang_map);
        return
    }
    // since term has been seen before, there must be at least one lang for it
    // check if item's lang has been seen before
    let lang_map: &mut LangMap = term_map.get_mut(&item.term).unwrap();
    if !lang_map.contains_key(&lang) {
        let mut item_set: ItemSet = HashSet::new();
        let mut ety_map: EtyMap = HashMap::new();
        item_set.insert(item);
        ety_map.insert(ety_text, item_set);
        lang_map.insert(lang, ety_map);
        return
    }
    // since lang has been seen before, there must be at least one ety
    // and for any ety, there must be at least one item
    let ety_map: &mut EtyMap = lang_map.get_mut(&lang).unwrap();
    if ety_map.contains_key(&ety_text) {
        let item_set: &mut ItemSet = ety_map.get_mut(&ety_text).unwrap();
        item_set.insert(item);
    } else {
        let mut item_set: ItemSet = HashSet::new();
        item_set.insert(item);
        ety_map.insert(ety_text, item_set);
    }
}

pub fn print_all_items(term_map: &TermMap) {
    for (term, lang_map) in term_map.iter() {
        println!("{term}");
        for (lang, ety_map) in lang_map.iter() {
            println!("  {lang}");
            for (ety_text, item_set) in ety_map.iter() {
                println!("    {ety_text}");
                for item in item_set.iter() {
                    println!("      ({}), {}", item.pos, item.gloss);
                }
            }
        }
    }
}

// fn print_item(item: &Item) {
//     println!("{}", item.term);
//     println!("  {}", item.lang);
//     println!("    {}", item.ety_text);
//     println!("      ({}), {}", item.pos, item.gloss);
// }

fn is_valid_json_item(json_item: &BorrowedValue) -> bool {
    // some wiktionary pages are redirects, which we don't want
    // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
    !json_item.contains_key("redirect") &&
    // as of 2022-06-20, there is exactly one json_item that has no senses
    // https://github.com/tatuylonen/wiktextract/issues/139
    json_item["senses"].get_idx(0).is_some()
}

fn process_json_item(term_map: &mut TermMap, json_item: BorrowedValue) {
    let term = json_item["word"].as_str().unwrap().to_string();
    let lang = json_item["lang"].as_str().unwrap().to_string();
    let pos = json_item["pos"].as_str().unwrap().to_string();
    let mut ety_text = "".to_string();
    if json_item.contains_key("etymology_text") {
        ety_text = json_item["etymology_text"].as_str().unwrap().to_string();
    }
    let mut gloss = "".to_string();
    if json_item["senses"][0].contains_key("glosses") &&
            json_item["senses"][0]["glosses"].get_idx(0).is_some() {
        gloss = json_item["senses"][0]["glosses"][0].as_str().unwrap().to_string();
    }
    let item = Item {
        term: term,
        lang: lang,
        pos: pos,
        ety_text: ety_text,
        gloss: gloss
    };
    // print_item(&item);
    add_item(term_map, item);
}

fn process_json_items<T: BufRead>(lines: ByteLines<T>) -> TermMap {
    let mut term_map: TermMap = HashMap::new();
    lines
        .into_iter()
        .filter_map(Result::ok)
        .for_each(|mut line| {
            let json_item: BorrowedValue = to_borrowed_value(&mut line).unwrap();
            if is_valid_json_item(&json_item) {
                process_json_item(&mut term_map, json_item);
            }
        });
    return term_map;
}

pub async fn process_wiktextract_data() -> io::Result<TermMap> {
    match File::open(WIKTEXTRACT_PATH) {
        Ok(file) => {
            println!("processing data from local file {WIKTEXTRACT_PATH}");
            let reader = BufReader::new(file);
            let gz = GzDecoder::new(reader);
            let gz_reader = BufReader::new(gz);
            let lines = ByteLines::new(gz_reader);
            Ok(process_json_items(lines))
        }
        Err(_) => { // file doesn't exist or error opening it; download instead
            println!("processing data from {WIKTEXTRACT_URL}");
            Ok(process_download(WIKTEXTRACT_URL).await.unwrap())
        }
    }
}

// based on https://stackoverflow.com/a/69967522
pub async fn process_download(url: &str) -> io::Result<TermMap> {
    let client = reqwest::Client::new();

    let response: Response;

    match client.get(url).send().await {
        Ok(res) => response = res,
        Err(error) => {
            return Err(io::Error::new(io::ErrorKind::InvalidData, error));
        }
    };

    let (tx, rx) = flume::bounded(0);

    let decoder_thread = std::thread::spawn(move || {
        let input = ChannelRead::new(rx);
        let gz = GzDecoder::new(input);
        let reader = BufReader::new(gz);
        let lines = ByteLines::new(reader);
        let term_map = process_json_items(lines);
        return term_map;
    });

    if response.status() == reqwest::StatusCode::OK {
        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item
                .or(Err(format!("Error while downloading file")))
                .unwrap();
            tx.send_async(chunk.to_vec()).await.unwrap();
        }
        drop(tx); // close the channel to signal EOF
    }

    let term_map = tokio::task::spawn_blocking(|| decoder_thread.join())
        .await
        .unwrap()
        .unwrap();

    Ok(term_map)
}

// Wrap a channel into something that impls `io::Read`
struct ChannelRead {
    rx: flume::Receiver<Vec<u8>>,
    current: io::Cursor<Vec<u8>>,
}

impl ChannelRead {
    fn new(rx: flume::Receiver<Vec<u8>>) -> ChannelRead {
        ChannelRead {
            rx,
            current: io::Cursor::new(vec![]),
        }
    }
}

impl Read for ChannelRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.current.position() == self.current.get_ref().len() as u64 {
            // We've exhausted the previous chunk, get a new one.
            if let Ok(vec) = self.rx.recv() {
                self.current = io::Cursor::new(vec);
            }
            // If recv() "fails", it means the sender closed its part of
            // the channel, which means EOF. Propagate EOF by allowing
            // a read from the exhausted cursor.
        }
        self.current.read(buf)
    }
}