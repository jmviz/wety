#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use processor::{embeddings, process_wiktextract};

use std::{env, path::PathBuf, time::Instant};

use anyhow::Result;
use clap::Parser;
use indicatif::HumanDuration;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(
        short = 'w',
        long,
        default_value = "data/raw-wiktextract-data.jsonl.gz",
        value_parser
    )]
    wiktextract_path: PathBuf,
    #[clap(short = 's', long, default_value = "data/wety.json.gz", value_parser)]
    serialization_path: PathBuf,
    #[clap(short = 't', long, value_parser)]
    turtle_path: Option<PathBuf>,
    #[clap(short = 'm', long, default_value = embeddings::DEFAULT_MODEL, value_parser)]
    embeddings_model: String,
    #[clap(short = 'r', long, default_value = embeddings::DEFAULT_MODEL_REVISION, value_parser)]
    embeddings_model_revision: String,
    #[clap(short = 'b', long, default_value_t = embeddings::DEFAULT_BATCH_SIZE, value_parser)]
    embeddings_batch_size: usize,
    #[clap(
        short = 'c',
        long,
        default_value = "data/embeddings_cache",
        value_parser
    )]
    embeddings_cache_path: PathBuf,
}

fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");
    let total_time = Instant::now();
    let args = Args::parse();
    let embeddings_config = embeddings::Config {
        model_name: args.embeddings_model,
        model_revision: args.embeddings_model_revision,
        batch_size: args.embeddings_batch_size,
        cache_path: args.embeddings_cache_path,
    };
    process_wiktextract(
        &args.wiktextract_path,
        &args.serialization_path,
        args.turtle_path.as_deref(),
        &embeddings_config,
    )?;

    println!(
        "All done! Took {} overall. Exiting...",
        HumanDuration(total_time.elapsed())
    );
    Ok(())
}
