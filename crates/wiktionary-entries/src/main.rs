use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::mpsc::sync_channel,
    thread,
};

use anyhow::{Result, bail};
use clap::Parser;
use flate2::{Compression, read::MultiGzDecoder, write::GzEncoder};
use rayon::prelude::*;
use wety_wiktionary_entries::{
    model::Counts,
    page::{PageInfo, entries},
    records::{Record, each_line},
};

/// Wiktionary's Enterprise snapshot (and pages in its record shape, such as
/// Reconstruction pages fetched by revision) to wety's entries file.
#[derive(Parser)]
struct Args {
    /// An Enterprise snapshot: a tar.gz of NDJSON page records.
    #[clap(long)]
    enterprise: Option<PathBuf>,
    /// NDJSON.gz files of page records in the same shape.
    #[clap(long, multiple_occurrences = true)]
    records: Vec<PathBuf>,
    /// The entries file to write (JSON lines, gzipped).
    #[clap(long)]
    out: PathBuf,
    /// Where to write the counts (JSON).
    #[clap(long)]
    counts: PathBuf,
    /// Read at most this many records from each input.
    #[clap(long)]
    limit: Option<usize>,
    /// Keep only the pages named in this file, one title per line.
    #[clap(long)]
    titles: Option<PathBuf>,
}

const BATCH: usize = 256;

#[derive(serde::Deserialize)]
struct Key {
    page_id: u64,
    rev: u64,
}

/// Copy `from` to `to`, keeping one record per page: its highest revision, first seen.
/// A record's entries are consecutive lines with one (page, revision). Returns the
/// number of lines dropped.
fn keep_latest(from: &Path, to: &Path, latest: &HashMap<u64, u64>) -> Result<u64> {
    let reader = BufReader::with_capacity(1 << 20, MultiGzDecoder::new(File::open(from)?));
    let mut out = GzEncoder::new(BufWriter::new(File::create(to)?), Compression::fast());
    let (mut done, mut current, mut keep, mut dropped) = (HashSet::new(), None, false, 0);
    for line in reader.lines() {
        let line = line?;
        let key: Key = serde_json::from_str(&line)?;
        if current != Some((key.page_id, key.rev)) {
            if let (Some((id, _)), true) = (current, keep) {
                done.insert(id);
            }
            current = Some((key.page_id, key.rev));
            keep = latest.get(&key.page_id) == Some(&key.rev) && !done.contains(&key.page_id);
        }
        if keep {
            out.write_all(line.as_bytes())?;
            out.write_all(b"\n")?;
        } else {
            dropped += 1;
        }
    }
    out.finish()?.flush()?;
    Ok(dropped)
}

/// A record's page and revision, its entries as JSON lines, and what it counted.
type Processed = (Option<(u64, u64)>, Vec<String>, Counts);

fn main() -> Result<()> {
    let args = Args::parse();
    let mut inputs: Vec<(PathBuf, bool)> =
        args.enterprise.iter().map(|p| (p.clone(), true)).collect();
    inputs.extend(args.records.iter().map(|p| (p.clone(), false)));
    if inputs.is_empty() {
        bail!("give --enterprise or --records");
    }
    let titles: Option<HashSet<String>> = match &args.titles {
        Some(p) => Some(fs::read_to_string(p)?.lines().map(str::to_string).collect()),
        None => None,
    };
    let limit = args.limit;
    let (tx, rx) = sync_channel::<Vec<String>>(16);
    let reader = thread::spawn(move || -> Result<()> {
        for (path, tar) in inputs {
            let mut batch = Vec::with_capacity(BATCH);
            let mut n = 0;
            each_line(&path, tar, |line| {
                batch.push(line);
                n += 1;
                if batch.len() == BATCH {
                    let full = std::mem::replace(&mut batch, Vec::with_capacity(BATCH));
                    if tx.send(full).is_err() {
                        return false;
                    }
                }
                limit.is_none_or(|l| n < l)
            })?;
            if !batch.is_empty() {
                let _ = tx.send(batch);
            }
        }
        Ok(())
    });

    // Every record's entries go here first; superseded revisions are dropped after.
    let partial = args.out.with_extension("partial.gz");
    let mut out = GzEncoder::new(BufWriter::new(File::create(&partial)?), Compression::fast());
    let mut counts = Counts::default();
    let mut revisions: HashMap<u64, u64> = HashMap::new();
    for batch in rx {
        let results: Vec<Processed> = batch
            .par_iter()
            .map(|line| {
                let mut c = Counts::default();
                c.add("records", 1);
                let record: Record = match serde_json::from_str(line) {
                    Ok(r) => r,
                    Err(_) => {
                        c.add("unreadable records", 1);
                        return (None, vec![], c);
                    }
                };
                if titles.as_ref().is_some_and(|t| !t.contains(&record.name)) {
                    return (None, vec![], c);
                }
                c.add("records read", 1);
                if record.article_body.html.is_empty() {
                    c.add("records without html", 1);
                }
                let info = PageInfo {
                    title: &record.name,
                    page_id: record.identifier,
                    rev: record.version.identifier,
                    ns: record.namespace.as_ref().map_or(0, |n| n.identifier),
                };
                let lines = entries(&info, &record.article_body.html, &mut c)
                    .iter()
                    .filter_map(|e| serde_json::to_string(e).ok())
                    .collect();
                (
                    Some((record.identifier, record.version.identifier)),
                    lines,
                    c,
                )
            })
            .collect();
        for (page, lines, c) in results {
            counts.merge(c);
            if let Some((id, rev)) = page
                && let Some(seen) = revisions.insert(id, rev)
            {
                // Enterprise repeats about 1 % of pages.
                counts.add("records repeating a page", 1);
                revisions.insert(id, seen.max(rev));
                if seen == rev {
                    continue;
                }
            }
            for line in lines {
                out.write_all(line.as_bytes())?;
                out.write_all(b"\n")?;
            }
        }
    }
    reader.join().expect("the reader thread")?;
    out.finish()?.flush()?;
    let dropped = keep_latest(&partial, &args.out, &revisions)?;
    counts.add("entries of superseded records dropped", dropped);
    fs::remove_file(&partial)?;
    fs::write(
        &args.counts,
        serde_json::to_string_pretty(&counts.0)? + "\n",
    )?;
    Ok(())
}
