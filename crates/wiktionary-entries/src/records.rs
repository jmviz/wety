//! Page records: lines of an Enterprise snapshot (a tar.gz of NDJSON) or of an NDJSON.gz
//! in the same shape (`source_tools.wiktionary_html`).

use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

use anyhow::{Context, Result};
use flate2::read::MultiGzDecoder;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Record {
    pub name: String,
    pub identifier: u64,
    #[serde(default)]
    pub namespace: Option<Identified<i64>>,
    pub version: Identified<u64>,
    pub article_body: Body,
}

#[derive(Deserialize)]
pub struct Identified<T> {
    pub identifier: T,
}

#[derive(Deserialize)]
pub struct Body {
    #[serde(default)]
    pub html: String,
    #[serde(default)]
    pub wikitext: String,
}

/// Each line of `path`, in order, to `f`; stops early when `f` returns false.
/// `tar` says whether the gzip holds a tar archive (Enterprise) or NDJSON itself.
pub fn each_line(path: &Path, tar: bool, mut f: impl FnMut(String) -> bool) -> Result<()> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let gz = MultiGzDecoder::new(BufReader::with_capacity(1 << 20, file));
    if !tar {
        return lines(gz, &mut f).map(|_| ());
    }
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries()? {
        if !lines(entry?, &mut f)? {
            break;
        }
    }
    Ok(())
}

fn lines(reader: impl Read, f: &mut impl FnMut(String) -> bool) -> Result<bool> {
    for line in BufReader::with_capacity(1 << 20, reader).lines() {
        let line = line?;
        if !line.is_empty() && !f(line) {
            return Ok(false);
        }
    }
    Ok(true)
}
