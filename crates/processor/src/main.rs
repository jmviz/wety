#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use processor::{ClaimsSource, embeddings, process_entries};

use std::{path::PathBuf, time::Instant};

use anyhow::Result;
use clap::Parser;
use indicatif::HumanDuration;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// wety's entries of Wiktionary (`wiktionary-entries --out`)
    #[clap(long, value_parser)]
    entries: PathBuf,
    /// The dump's redirects (`wiktionary-entries --redirects`)
    #[clap(long, value_parser)]
    redirects: Option<PathBuf>,
    #[clap(
        short = 's',
        long,
        default_value = "data/workspaces/etymology_graph/wety.json.gz",
        value_parser
    )]
    serialization_path: PathBuf,
    #[clap(short = 't', long, value_parser)]
    turtle_path: Option<PathBuf>,
    /// The entry names mentions link to (`wiktionary-entries --entry-names`)
    #[clap(short = 'e', long, value_parser)]
    entry_names: Option<PathBuf>,
    /// Each References section's credits (the wiktionary_references workflow)
    #[clap(long, value_parser, requires = "bibliography")]
    references: Option<PathBuf>,
    /// The works those credits cite (the same workflow)
    #[clap(long, value_parser, requires = "references")]
    bibliography: Option<PathBuf>,
    /// Another source's claims: its assertions and its matches to Wiktionary's
    /// records (a claims workflow's product), as two paths; repeatable
    #[clap(long, value_parser, number_of_values = 2, multiple_occurrences = true, value_names = &["ASSERTIONS", "MATCHES"])]
    claims: Vec<PathBuf>,
    /// Reviewers' overrides of words' readings
    #[clap(long, value_parser)]
    overrides: Option<PathBuf>,
    #[clap(short = 'm', long, default_value = embeddings::DEFAULT_MODEL, value_parser)]
    embeddings_model: String,
    #[clap(short = 'r', long, default_value = embeddings::DEFAULT_MODEL_REVISION, value_parser)]
    embeddings_model_revision: String,
    #[clap(short = 'b', long, default_value_t = embeddings::DEFAULT_BATCH_SIZE, value_parser)]
    embeddings_batch_size: usize,
    #[clap(
        short = 'c',
        long,
        default_value = "data/cache/embeddings/current",
        value_parser
    )]
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
        "All done! Took {} overall. Exiting...",
        HumanDuration(total_time.elapsed())
    );
    Ok(())
}
