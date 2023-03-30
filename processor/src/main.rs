#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use processor::{
    build_oxigraph_store,
    embeddings::{
        EmbeddingsConfig, EmbeddingsModel, DEFAULT_BATCH_SIZE, DEFAULT_MODEL,
        DEFAULT_PROGRESS_UPDATE_INTERVAL,
    },
    process_wiktextract,
};

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
        default_value = "data/raw-wiktextract-data.json.gz",
        value_parser
    )]
    wiktextract_path: PathBuf,
    #[clap(short = 'j', long, default_value = "data/wety.json.gz", value_parser)]
    serialization_path: PathBuf,
    #[clap(short = 't', long, action)]
    write_turtle: bool,
    #[clap(short = 'l', long, default_value = "data/wety.ttl", value_parser)]
    turtle_path: PathBuf,
    #[clap(short = 'b', long, action)]
    build_oxigraph_store: bool,
    #[clap(short = 's', long, default_value = "data/wety.db", value_parser)]
    oxigraph_store_path: PathBuf,
    #[clap(short = 'o', long, action)]
    optimize_oxigraph_store: bool,
    #[clap(short = 'm', long, value_enum, default_value_t = DEFAULT_MODEL, value_parser)]
    embeddings_model: EmbeddingsModel,
    #[clap(short = 'z', long, default_value_t = DEFAULT_BATCH_SIZE, value_parser)]
    embeddings_batch_size: usize,
    #[clap(short = 'u', long, default_value_t = DEFAULT_PROGRESS_UPDATE_INTERVAL, value_parser)]
    embeddings_progress_update_interval: usize,
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
    let embeddings_config = EmbeddingsConfig {
        model: args.embeddings_model,
        batch_size: args.embeddings_batch_size,
        progress_update_interval: args.embeddings_progress_update_interval,
        cache_path: args.embeddings_cache_path,
    };
    let t = process_wiktextract(
        &args.wiktextract_path,
        &args.serialization_path,
        &args.turtle_path,
        &embeddings_config,
    )?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    if args.build_oxigraph_store {
        build_oxigraph_store(
            &args.turtle_path,
            &args.oxigraph_store_path,
            args.optimize_oxigraph_store,
        )?;
    }
    println!(
        "All done! Took {} overall. Exiting...",
        HumanDuration(total_time.elapsed())
    );
    Ok(())
}
