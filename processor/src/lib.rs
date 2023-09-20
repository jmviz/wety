//! WIP attempt to digest etymologies from wiktextract data

#![feature(let_chains, array_chunks)]
#![allow(clippy::redundant_closure_for_method_calls)]

mod descendants;
pub mod embeddings;
mod ety_graph;
mod etymology;
mod etymology_templates;
mod gloss;
mod items;
pub use crate::items::ItemId;
mod langterm;
mod languages;
use crate::items::Items;
pub use crate::languages::Lang;
mod pos;
mod pos_phf;
mod processed;
pub use crate::processed::{Data, Search};
mod redirects;
mod root;
mod string_pool;
mod turtle;
mod wiktextract_json;
pub use crate::wiktextract_json::wiktextract_lines;

use crate::string_pool::StringPool;

use std::{convert::TryFrom, path::Path, time::Instant};

use anyhow::{Ok, Result};
use embeddings::EmbeddingsConfig;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use xxhash_rust::xxh3::Xxh3Builder;

pub(crate) type HashMap<K, V> = std::collections::HashMap<K, V, Xxh3Builder>;
pub(crate) type HashSet<T> = std::collections::HashSet<T, Xxh3Builder>;

pub(crate) fn progress_bar(n: usize, message: &str) -> Result<ProgressBar> {
    let pb = ProgressBar::new(u64::try_from(n)?);
    let template = format!("{{spinner:.green}} {message}: [{{elapsed}}] [{{bar:.cyan/blue}}] {{human_pos}}/{{human_len}} ({{per_sec}}, {{eta}})");
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&template)?
            .progress_chars("#>-"),
    );
    Ok(pb)
}

/// # Errors
///
/// Will return `Err` if any unexpected issue arises parsing the wiktextract
/// data or writing to Turtle file.
pub fn process_wiktextract(
    wiktextract_path: &Path,
    serialization_path: &Path,
    turtle_path: Option<&Path>,
    embeddings_config: &EmbeddingsConfig,
) -> Result<()> {
    let mut t = Instant::now();
    println!(
        "Processing raw wiktextract data from {}...",
        wiktextract_path.display()
    );
    let mut string_pool = StringPool::new();
    let mut items = Items::new()?;
    items.process_wiktextract_lines(&mut string_pool, wiktextract_path)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    let embeddings =
        items.generate_embeddings(&string_pool, wiktextract_path, embeddings_config)?;
    t = Instant::now();
    println!("Generating ety graph...");
    items.generate_ety_graph(&embeddings)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    let data = Data::new(string_pool, items.graph);
    if let Some(turtle_path) = turtle_path {
        data.write_turtle(turtle_path)?;
    }
    data.serialize(serialization_path)?;
    Ok(())
}
