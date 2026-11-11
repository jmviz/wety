// Copies an embedding store of f32 vectors (as the processor kept them before
// storing f16) into an f16 store, rounding each value to nearest, so that nothing is
// embedded again:
//
// cargo run --release --bin embeddings-f16 -- --from OLD --to NEW
//
// NEW is the namespace directory the processor now opens (`cargo test --release -p
// processor parity::namespace -- --ignored --nocapture`). A one-off: delete this
// binary once the stores in use are converted.

use processor::embeddings::Store;

use std::{fs, path::PathBuf};

use anyhow::{Result, ensure};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    from: PathBuf,
    #[arg(long)]
    to: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let dim: usize = fs::read_to_string(args.from.join("dim"))?.trim().parse()?;
    let keys = fs::read(args.from.join("keys"))?;
    let vectors = fs::read(args.from.join("vectors"))?;
    let n = (keys.len() / 8).min(vectors.len() / (dim * 4));
    ensure!(n > 0, "{} holds no embeddings", args.from.display());
    let mut store = Store::open(&args.to)?;
    let before = store.len();
    let keys = keys.as_chunks::<8>().0;
    let values = vectors.as_chunks::<4>().0;
    let mut batch: Vec<(u64, Vec<f32>)> = Vec::with_capacity(10_000);
    for i in 0..n {
        let vector = values[i * dim..(i + 1) * dim]
            .iter()
            .map(|&b| f32::from_le_bytes(b))
            .collect();
        batch.push((u64::from_le_bytes(keys[i]), vector));
        if batch.len() == batch.capacity() {
            store.append(batch.iter().map(|(k, v)| (*k, v.as_slice())))?;
            batch.clear();
        }
    }
    store.append(batch.iter().map(|(k, v)| (*k, v.as_slice())))?;
    store.remap()?;
    println!(
        "Read {n} embeddings from {}; {} now holds {} ({} new).",
        args.from.display(),
        args.to.display(),
        store.len(),
        store.len() - before
    );
    Ok(())
}
