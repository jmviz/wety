#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use wety::{build_store, wiktextract_to_turtle};

use std::time::Instant;

use anyhow::Result;
use clap::Parser;
use indicatif::HumanDuration;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(
        short,
        long,
        default_value = "data/raw-wiktextract-data.json.gz",
        value_parser
    )]
    wiktextract_path: String,
    #[clap(short, long, default_value = "data/wety.ttl", value_parser)]
    turtle_path: String,
    #[clap(short, long, default_value = "data/wety.db", value_parser)]
    store_path: String,
    #[clap(short = 'b', long, action)]
    skip_building_store: bool,
    #[clap(short = 'o', long, action)]
    skip_optimizing_store: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let total_time = Instant::now();
    let t = wiktextract_to_turtle(&args.wiktextract_path, &args.turtle_path)?;
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
