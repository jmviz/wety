//! WIP attempt to digest etymologies from wiktextract data

#![feature(is_some_and, let_chains, array_chunks)]
#![allow(clippy::redundant_closure_for_method_calls)]

mod descendants;
pub mod embeddings;
mod ety_graph;
mod etymology;
mod etymology_templates;
mod gloss;
mod items;
#[cfg(feature = "processed")]
pub use crate::items::ItemId;
mod lang_phf;
mod langterm;
mod languages;
use crate::items::Items;
#[cfg(feature = "processed")]
pub use crate::langterm::{Lang, LangId};
mod phf_ext;
mod pos;
mod pos_phf;
mod processed;
#[cfg(feature = "processed")]
pub use crate::processed::{Data, Search};
mod redirects;
mod root;
mod string_pool;
mod turtle;
mod wiktextract_json;

use crate::string_pool::StringPool;

use std::{
    convert::TryFrom,
    fs::{remove_dir_all, File},
    io::BufReader,
    path::Path,
    time::Instant,
};

use anyhow::{Ok, Result};
use embeddings::EmbeddingsConfig;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use oxigraph::{io::GraphFormat::Turtle, model::GraphNameRef::DefaultGraph, store::Store};
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
    write_turtle: bool,
    turtle_path: &Path,
    embeddings_config: &EmbeddingsConfig,
) -> Result<Instant> {
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
    if write_turtle {
        data.write_turtle(turtle_path)?;
    }
    data.serialize(serialization_path)?;
    t = Instant::now();
    println!("Dropping all processed data...");
    Ok(t)
}

/// # Errors
///
/// Will return `Err` if any unexpected issue arises building the Oxigraph store.
pub fn build_oxigraph_store(turtle_path: &Path, store_path: &Path, optimize: bool) -> Result<()> {
    let mut t = Instant::now();
    println!("Building oxigraph store {}...", store_path.display());
    let turtle = BufReader::new(File::open(turtle_path)?);
    // delete any previous oxigraph db
    if store_path.is_dir() {
        remove_dir_all(store_path)?;
    }
    let store = Store::open(store_path)?;
    store
        .bulk_loader()
        .load_graph(turtle, Turtle, DefaultGraph, None)?;
    store.flush()?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    if optimize {
        t = Instant::now();
        println!("Optimizing oxigraph store {}...", store_path.display());
        store.optimize()?;
        store.flush()?;
        println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    }
    Ok(())
}
