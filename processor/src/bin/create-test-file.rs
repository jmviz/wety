// Run from workspace root.
//
// Expects a CSV file with lang_code,term columns indicating requested items for test file.
//
// See:
//
// cargo run --release --bin create-test-file -- --help
//
//
// Example usage:
//
// cargo run --release --bin create-test-file data/test/example.csv

#![feature(let_chains)]

#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use processor::wiktextract_lines;

use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    time::Instant,
};

use anyhow::Result;
use clap::Parser;
use indicatif::HumanDuration;
use serde::Deserialize;
use simd_json::{to_borrowed_value, value::borrowed::Value, ValueAccess};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(
        help = "Path to CSV file with lang_code,term columns indicating requested items for test file"
    )]
    csv: PathBuf,
    #[clap(
        short = 'f',
        long,
        default_value = "data/raw-wiktextract-data.json.gz",
        help = "Path to full wiktextract raw data file",
        value_parser
    )]
    full: PathBuf,
    #[clap(
        short = 't',
        long,
        help = "Test file name. If not provided, test file will be named after CSV file.",
        value_parser
    )]
    test: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq, Hash, Deserialize)]
struct RequestedItem {
    lang: String,
    term: String,
}

struct RequestedItems {
    items: Vec<RequestedItem>,
}

impl RequestedItems {
    fn matches(&self, json: &Value) -> bool {
        if let Some(lang) = json.get_str("lang_code")
            && let Some(term) = json.get_str("word")
        {
            return self.items.iter().any(|item| item.lang == lang && item.term == term);
        }
        false
    }
}

fn read_csv(path: &PathBuf) -> Result<RequestedItems> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)?;
    let mut items = Vec::new();
    for result in rdr.deserialize() {
        let item: RequestedItem = result?;
        items.push(item);
    }
    Ok(RequestedItems { items })
}

fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");
    let t = Instant::now();
    let args = Args::parse();

    let requested_items = read_csv(&args.csv)?;

    let test_file_path = args.test.unwrap_or_else(|| {
        let mut path = args.csv.clone();
        path.set_extension("jsonl");
        path
    });

    let mut test_file = BufWriter::new(File::create(&test_file_path)?);

    let mut n = 0;

    for mut line in wiktextract_lines(&args.full)? {
        let json = to_borrowed_value(&mut line)?;
        if requested_items.matches(&json) {
            serde_json::to_writer(&mut test_file, &json)?;
            test_file.write_all(b"\n")?;
            n += 1;
        }
    }

    test_file.flush()?;

    println!(
        "Wrote {n} items to test file {}. Took {}.",
        test_file_path.display(),
        HumanDuration(t.elapsed())
    );
    Ok(())
}
