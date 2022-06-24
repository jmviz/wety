#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use bytelines::ByteLines;
use rayon::iter::ParallelBridge;
use rayon::prelude::ParallelIterator;
use simd_json::{BorrowedValue, to_borrowed_value, ValueAccess};
use std::sync::Arc;
use dashmap::DashMap;

fn is_valid(entry: &BorrowedValue) -> bool {
    // some wiktionary pages are redirects, which we don't want
    // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
    !entry.contains_key("redirect") &&
    // as of 2022-06-20, there is exactly one entry that has no senses
    // https://github.com/tatuylonen/wiktextract/issues/139
    entry["senses"].get_idx(0).is_some()
}

fn main() -> Result<(), Box<dyn Error>> {
    // let file = File::open("data/test/sekʷ-.json")?;
    // let file = File::open("data/download/raw-wiktextract-data.json")?;
    let file = File::open("data/download/kaikki.org-dictionary-all.json")?;
    let entries: Arc<DashMap<String, String>> = Arc::new(DashMap::new());
    let reader = BufReader::new(file);
    let lines = ByteLines::new(reader);
    lines
        .into_iter()
        .par_bridge()
        .filter_map(Result::ok)
        .for_each(|mut line| {
            let entry: BorrowedValue = to_borrowed_value(&mut line).expect("read json line");
            if is_valid(&entry) {
                if !entry["senses"][0].contains_key("id") {
                    println!("{}", entry)
                }
                // let id = entry["senses"][0]["id"].as_str().expect("convert borrowed value to string").to_string();
                // let word = entry["word"].as_str().expect("convert borrowed value to string").to_string();
                // entries.insert(id, word);

                // let lang = entry["lang"].as_str().expect("get lang");
                // let pos = entry["pos"].as_str().expect("get pos");
            }
        });
    println!("{}", entries.len());
    Ok(())
}

