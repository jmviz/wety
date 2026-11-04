//! WIP attempt to digest etymologies from wiktextract data

#![allow(clippy::redundant_closure_for_method_calls)]

mod claims;
mod descendants;
// The embedding cache is keyed by this file's text, so editing it, even to
// reformat or to satisfy a lint, discards the cache: hours of embedding.
#[allow(clippy::large_enum_variant)]
#[rustfmt::skip]
pub mod embeddings;
mod etymology;
mod etymon;
mod items;
mod notes;
mod redirects;
mod references;
mod resolve;
mod root;
pub mod turtle;
mod wiktextract_json;
pub use crate::wiktextract_json::wiktextract_lines;

use wety_core::string_pool::StringPool;
pub use wety_core::{self, Data, HashMap, HashSet, ItemId, Lang, Search};

pub use crate::claims::ClaimsSource;
use crate::{items::Items, references::References};

use std::{convert::TryFrom, path::Path, time::Instant};

use anyhow::{Ok, Result};
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};

pub(crate) fn progress_bar(n: usize, message: &str) -> Result<ProgressBar> {
    let pb = ProgressBar::new(u64::try_from(n)?);
    let template = format!(
        "{{spinner:.green}} {message}: [{{elapsed}}] [{{bar:.cyan/blue}}] {{human_pos}}/{{human_len}} ({{per_sec}}, {{eta}})"
    );
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
#[allow(clippy::too_many_arguments)]
pub fn process_wiktextract(
    wiktextract_path: &Path,
    serialization_path: &Path,
    turtle_path: Option<&Path>,
    entry_names_path: Option<&Path>,
    references_paths: Option<(&Path, &Path)>,
    claims: &[ClaimsSource],
    overrides: Option<&Path>,
    embeddings_config: &embeddings::Config,
) -> Result<()> {
    let mut t = Instant::now();
    println!(
        "Processing raw wiktextract data from {}...",
        wiktextract_path.display()
    );
    let mut string_pool = StringPool::new();
    let mut items = Items::new()?;
    if let Some(entry_names_path) = entry_names_path {
        items.load_entry_names(&mut string_pool, entry_names_path)?;
        println!("Loaded {} entry names.", items.entry_names.len());
    }
    if let Some((sections, bibliography)) = references_paths {
        items.references = References::load(sections, bibliography)?;
        println!(
            "Loaded {} References sections.",
            items.references.sections()
        );
    }
    items.process_wiktextract_lines(&mut string_pool, wiktextract_path)?;
    if references_paths.is_some() {
        println!(
            "{} References sections cover {} items.",
            items.references.covering(),
            items.references.items()
        );
    }
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    let embeddings =
        items.generate_embeddings(&string_pool, wiktextract_path, embeddings_config)?;
    t = Instant::now();
    println!("Generating ety graph...");
    items.generate_ety_graph(&string_pool, &embeddings)?;
    items.renumber_imputed_dupes(&string_pool);
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    if !claims.is_empty() || overrides.is_some() {
        t = Instant::now();
        println!("Joining other sources' claims...");
        let counts = items.apply_claims(&string_pool, claims, overrides)?;
        println!("{}", serde_json::to_string_pretty(&counts)?);
        println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    }
    let data = Data::new(string_pool, items.graph);
    if let Some(turtle_path) = turtle_path {
        turtle::write_turtle(&data, turtle_path)?;
    }
    data.serialize(serialization_path)?;
    Ok(())
}
