#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use wety::{
    build_store,
    embeddings::{
        EmbeddingsConfig, EmbeddingsModel, DEFAULT_BATCH_SIZE, DEFAULT_MODEL,
        DEFAULT_PROGRESS_UPDATE_INTERVAL,
    },
    wiktextract_to_turtle,
};

use std::{path::PathBuf, time::Instant};

use anyhow::Result;
use clap::Parser;
use indicatif::HumanDuration;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(
        short = 'w',
        long,
        default_value = "data/raw-wiktextract-data.json.gz",
        value_parser
    )]
    wiktextract_path: PathBuf,
    #[clap(short = 't', long, default_value = "data/wety.ttl", value_parser)]
    turtle_path: PathBuf,
    #[clap(short = 's', long, default_value = "data/wety.db", value_parser)]
    store_path: PathBuf,
    #[clap(short = 'b', long, action)]
    skip_building_store: bool,
    #[clap(short = 'o', long, action)]
    skip_optimizing_store: bool,
    #[clap(short = 'm', long, value_enum, default_value_t = DEFAULT_MODEL, value_parser)]
    embeddings_model: EmbeddingsModel,
    #[clap(short = 'z', long, default_value_t = DEFAULT_BATCH_SIZE, value_parser)]
    embeddings_batch_size: usize,
    #[clap(short = 'p', long, default_value_t = DEFAULT_PROGRESS_UPDATE_INTERVAL, value_parser)]
    embeddings_progress_update_interval: usize,
}

fn main() -> Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    let total_time = Instant::now();
    let args = Args::parse();
    let embeddings_config = EmbeddingsConfig {
        model: args.embeddings_model,
        batch_size: args.embeddings_batch_size,
        progress_update_interval: args.embeddings_progress_update_interval,
    };
    let t = wiktextract_to_turtle(
        &args.wiktextract_path,
        &args.turtle_path,
        &embeddings_config,
    )?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    if !args.skip_building_store {
        build_store(
            &args.turtle_path,
            &args.store_path,
            args.skip_optimizing_store,
        )?;
    }
    println!(
        "All done! Took {} overall. Exiting...",
        HumanDuration(total_time.elapsed())
    );
    Ok(())
}
