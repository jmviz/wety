//! WIP attempt to digest etymologies from wiktextract data

#![feature(is_some_and, let_chains)]
#![allow(clippy::redundant_closure_for_method_calls)]

mod descendants;
pub mod embeddings;
mod ety_graph;
mod etymology;
mod etymology_templates;
mod lang;
mod lang_phf;
mod phf_ext;
mod pos_phf;
mod raw_items;
mod redirects;
mod root;
mod string_pool;
mod turtle;
mod wiktextract_json;

use crate::{
    ety_graph::EtyGraph,
    lang::{etylang2lang, is_reconstructed_lang},
    raw_items::RawItems,
    string_pool::StringPool,
    turtle::write_turtle_file,
};

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

struct RawDataProcessor {
    string_pool: StringPool,
}

impl RawDataProcessor {
    fn new() -> Result<Self> {
        Ok(Self {
            string_pool: StringPool::default(),
        })
    }
}

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

pub(crate) struct ProcessedData {
    string_pool: StringPool,
    items: RawItems,
    ety_graph: EtyGraph,
}

/// # Errors
///
/// Will return `Err` if any unexpected issue arises parsing the wiktextract
/// data or writing to Turtle file.
pub fn wiktextract_to_turtle(
    wiktextract_path: &Path,
    turtle_path: &Path,
    embeddings_config: &EmbeddingsConfig,
) -> Result<Instant> {
    let mut t = Instant::now();
    println!(
        "Processing raw wiktextract data from {}...",
        wiktextract_path.display()
    );
    let mut processor = RawDataProcessor::new()?;
    let items = processor.process_json_items(wiktextract_path)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    let embeddings =
        items.generate_embeddings(&processor.string_pool, wiktextract_path, embeddings_config)?;
    t = Instant::now();
    println!("Generating ety graph...");
    let ety_graph = items.generate_ety_graph(&processor.string_pool, &embeddings)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    let data = ProcessedData {
        string_pool: processor.string_pool,
        items,
        ety_graph,
    };
    write_turtle_file(&data, turtle_path)?;
    t = Instant::now();
    println!("Dropping all processed data...");
    Ok(t)
}

/// # Errors
///
/// Will return `Err` if any unexpected issue arises building the Oxigraph store.
pub fn build_store(turtle_path: &Path, store_path: &Path, skip_optimizing: bool) -> Result<()> {
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
    if !skip_optimizing {
        t = Instant::now();
        println!("Optimizing oxigraph store {}...", store_path.display());
        store.optimize()?;
        store.flush()?;
        println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    }
    Ok(())
}
