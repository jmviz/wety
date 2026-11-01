// Run from workspace root.
//
// Writes every etymology edge of a processed graph as one TSV line, keyed by
// language, term and etymology number rather than by item id, so that the
// edges of two builds can be compared with sort and diff. The last columns say
// which of the child's etymologies the edge belongs to, how firmly, and whether
// walks follow it; a child of unknown origin, which has no edges, is listed in
// `--unknown`.
//
// cargo run --release --bin graph-edges -- data/.../wety.json.gz edges.tsv

#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use wety_core::{
    ety_graph::{EtyEdgeAccess, EtyKind},
    items::ItemId,
    processed::Data,
};

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// The processed graph (wety.json or wety.json.gz)
    graph: PathBuf,
    /// Where to write the TSV
    out: PathBuf,
    /// Where to write the items whose origin is unknown, one key per line
    #[clap(long)]
    unknown: Option<PathBuf>,
}

// a few terms hold a tab or a line break
fn escape(term: &str) -> String {
    term.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn key(data: &Data, item: ItemId) -> String {
    let item = data.graph.item(item);
    format!(
        "{}\t{}\t{}\t{}",
        item.lang().code(),
        escape(item.term().resolve(&data.string_pool)),
        item.ety_num(),
        if item.is_imputed() {
            "imputed"
        } else {
            "entry"
        }
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    let data = Data::deserialize(&args.graph)?;
    let mut out = BufWriter::new(File::create(&args.out)?);
    writeln!(
        out,
        "child_lang\tchild_term\tchild_ety\tchild_kind\tparent_lang\tparent_term\tparent_ety\tparent_kind\tmode\torder\thead\tbasis\tetymology\tcertainty\tfollowed"
    )?;
    for edge in data.graph.edges() {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            key(&data, edge.child()),
            key(&data, edge.parent()),
            edge.mode().as_ref(),
            edge.order(),
            edge.head(),
            edge.basis().as_ref(),
            edge.ety(),
            edge.certainty().as_ref(),
            data.graph.follows(edge)
        )?;
    }
    out.flush()?;
    if let Some(path) = args.unknown {
        let mut out = BufWriter::new(File::create(path)?);
        for (&item, etys) in &data.graph.etymologies {
            if etys.kinds.contains(&EtyKind::Unknown) {
                writeln!(out, "{}", key(&data, item))?;
            }
        }
        out.flush()?;
    }
    Ok(())
}
