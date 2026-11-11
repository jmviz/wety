// Copies a sled embedding cache, as the processor kept one before its embedding
// store (a directory under the cache path with a `db` file), into the store of the
// current model code, so that nothing is embedded again:
//
// cargo run --release --bin embeddings-import -- --sled data/cache/embeddings/current/287d08a6114155b7
//
// Only for a sled cache written by the same model code (model.rs) and model: the
// values are copied as they are. A one-off: delete this binary, and the sled
// dependency, once the caches in use are imported.

use processor::embeddings::{self, Config, Store};

use std::path::PathBuf;

use anyhow::{Result, ensure};
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// The sled cache: a namespace directory holding `db`
    #[clap(long, value_parser)]
    sled: PathBuf,
    #[clap(long, default_value = "data/cache/embeddings/current", value_parser)]
    cache_path: PathBuf,
    #[clap(long, default_value = embeddings::DEFAULT_MODEL, value_parser)]
    model: String,
    #[clap(long, default_value = embeddings::DEFAULT_MODEL_REVISION, value_parser)]
    revision: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.sled.join("db").is_file(),
        "{} is not a sled cache",
        args.sled.display()
    );
    let config = Config {
        model_name: args.model,
        model_revision: args.revision,
        batch_size: embeddings::DEFAULT_BATCH_SIZE,
        cache_path: args.cache_path,
    };
    let dir = config.store_dir()?;
    ensure!(
        dir != args.sled,
        "the store and the sled cache are one directory"
    );
    let mut store = Store::open(&dir)?;
    let before = store.len();
    let db = sled::open(&args.sled)?;
    let mut batch: Vec<(u64, Vec<f32>)> = Vec::with_capacity(10_000);
    let mut read = 0usize;
    for entry in db.iter() {
        let (key, value) = entry?;
        // sled kept keys and values big-endian
        let key = u64::from_be_bytes(key.as_ref().try_into()?);
        let (chunks, rest) = value.as_ref().as_chunks::<4>();
        ensure!(rest.is_empty(), "a value of {} bytes", value.len());
        batch.push((key, chunks.iter().map(|&b| f32::from_be_bytes(b)).collect()));
        read += 1;
        if batch.len() == batch.capacity() {
            store.append(batch.iter().map(|(k, v)| (*k, v.as_slice())))?;
            batch.clear();
        }
    }
    store.append(batch.iter().map(|(k, v)| (*k, v.as_slice())))?;
    store.remap()?;
    println!(
        "Read {read} embeddings from {}; {} now holds {} ({} new).",
        args.sled.display(),
        dir.display(),
        store.len(),
        store.len() - before
    );
    Ok(())
}
