#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

use processor::{ClaimsSource, embeddings, process_entries};

use std::{path::PathBuf, time::Instant};

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// wety's entries of Wiktionary (`wiktionary-entries --out`)
    #[arg(long)]
    entries: PathBuf,
    /// The dump's redirects (`wiktionary-entries --redirects`)
    #[arg(long)]
    redirects: Option<PathBuf>,
    #[arg(
        short = 's',
        long,
        default_value = "data/workspaces/etymology_graph/wety.bin.gz"
    )]
    serialization_path: PathBuf,
    #[arg(short = 't', long)]
    turtle_path: Option<PathBuf>,
    /// The entry names mentions link to (`wiktionary-entries --entry-names`)
    #[arg(short = 'e', long)]
    entry_names: Option<PathBuf>,
    /// Each References section's credits (the wiktionary_references workflow)
    #[arg(long, requires = "bibliography")]
    references: Option<PathBuf>,
    /// The works those credits cite (the same workflow)
    #[arg(long, requires = "references")]
    bibliography: Option<PathBuf>,
    /// Another source's claims: its assertions and its matches to Wiktionary's
    /// records (a claims workflow's product), as two paths; repeatable
    #[arg(long, num_args = 2, value_names = ["ASSERTIONS", "MATCHES"])]
    claims: Vec<PathBuf>,
    /// Reviewers' overrides of words' readings
    #[arg(long)]
    overrides: Option<PathBuf>,
    #[arg(short = 'm', long, default_value = embeddings::DEFAULT_MODEL)]
    embeddings_model: String,
    #[arg(short = 'r', long, default_value = embeddings::DEFAULT_MODEL_REVISION)]
    embeddings_model_revision: String,
    #[arg(short = 'b', long, default_value_t = embeddings::DEFAULT_BATCH_SIZE)]
    embeddings_batch_size: usize,
    #[arg(short = 'c', long, default_value = "data/cache/embeddings/current")]
    embeddings_cache_path: PathBuf,
}

fn main() -> Result<()> {
    let total_time = Instant::now();
    let args = Args::parse();
    let embeddings_config = embeddings::Config {
        model_name: args.embeddings_model,
        model_revision: args.embeddings_model_revision,
        batch_size: args.embeddings_batch_size,
        cache_path: args.embeddings_cache_path,
    };
    process_entries(
        &args.entries,
        args.redirects.as_deref(),
        &args.serialization_path,
        args.turtle_path.as_deref(),
        args.entry_names.as_deref(),
        args.references.as_deref().zip(args.bibliography.as_deref()),
        &args
            .claims
            .chunks(2)
            .map(|c| ClaimsSource {
                assertions: &c[0],
                matches: &c[1],
            })
            .collect::<Vec<_>>(),
        args.overrides.as_deref(),
        &embeddings_config,
    )?;

    println!(
        "All done! Took {:.1?} overall. Exiting...",
        total_time.elapsed()
    );
    Ok(())
}
