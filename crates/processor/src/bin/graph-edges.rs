// Run from workspace root.
//
// Writes every etymology edge of a processed graph as one TSV line, keyed by
// language, term and etymology number rather than by item id, so that the
// edges of two builds can be compared with sort and diff. The last columns say
// which of the child's etymologies the edge belongs to, how firmly, and whether
// walks follow it; a child of unknown origin, which has no edges, is listed in
// `--unknown`. `--readings` writes, as JSON Lines, each word that another
// source's claims touch or whose reading is not one lineage: its tier, its
// primary, and each etymology with its origins and claims.
//
// cargo run --release --bin graph-edges -- GRAPH edges.tsv [--unknown unknown.tsv] \
//     [--readings readings.jsonl]
//
// cargo run --release --bin graph-edges -- data/.../wety.json.gz edges.tsv

#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use wety_core::{
    HashSet,
    assertions::record_id,
    ety_graph::{EdgeBasis, EtyEdgeAccess, EtyKind, SourceLang},
    items::ItemId,
    processed::Data,
    synthesis::Reading,
};

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use itertools::Itertools;
use serde_json::json;

#[derive(Parser)]
struct Args {
    /// The processed graph (wety.json or wety.json.gz)
    graph: PathBuf,
    /// Where to write the TSV
    out: PathBuf,
    /// Where to write the items whose origin is unknown, one key per line
    #[clap(long)]
    unknown: Option<PathBuf>,
    /// Where to write the etymologies that name a language or family but no
    /// word: the item's key, then the etymology's number, mode, source code,
    /// certainty, and whether walks follow it (TSV)
    #[clap(long)]
    language_only: Option<PathBuf>,
    /// Where to write each credit of each etymology's assertion: the item's
    /// key, then the etymology's number, the work's id, scope, role and
    /// locator (TSV)
    #[clap(long)]
    credits: Option<PathBuf>,
    /// Where to write the readings of words that claims touch or that are
    /// not read by one lineage (JSON Lines)
    #[clap(long)]
    readings: Option<PathBuf>,
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
    if let Some(path) = args.language_only {
        let mut out = BufWriter::new(File::create(path)?);
        for (&item, etys) in &data.graph.etymologies {
            for (n, kind) in etys.kinds.iter().enumerate() {
                if let EtyKind::LanguageOnly(l) = kind {
                    let code = match l.source {
                        SourceLang::Lang(lang) => lang.code(),
                        SourceLang::Family(family) => family.code(),
                    };
                    let followed = etys.primary.is_none_or(|p| usize::from(p) == n);
                    writeln!(
                        out,
                        "{}\t{n}\t{}\t{code}\t{}\t{followed}",
                        key(&data, item),
                        l.mode.as_str(),
                        l.certainty.as_ref()
                    )?;
                }
            }
        }
        out.flush()?;
    }
    if let Some(path) = args.credits {
        let mut out = BufWriter::new(File::create(path)?);
        let graph = &data.graph;
        for (&item, asserted) in &graph.assertions {
            for (n, a) in asserted.iter().enumerate() {
                for c in &a.credits {
                    let work = graph.works.get(c.work as usize).map_or("", |w| &*w.id);
                    writeln!(
                        out,
                        "{}\t{n}\t{work}\t{}\t{}\t{}",
                        key(&data, item),
                        c.scope.as_ref(),
                        c.role.as_ref(),
                        escape(c.locator.as_deref().unwrap_or_default())
                    )?;
                }
            }
        }
        out.flush()?;
    }
    if let Some(path) = args.readings {
        let mut out = BufWriter::new(File::create(path)?);
        let graph = &data.graph;
        let agent = |n: u32| graph.agents.get(n as usize).map_or("rule", |a| &*a.id);
        let words: HashSet<ItemId> = graph
            .claims
            .keys()
            .copied()
            .chain(
                graph
                    .etymologies
                    .iter()
                    .filter(|(_, e)| e.reading != Reading::default())
                    .map(|(&i, _)| i),
            )
            .collect();
        for item in words.into_iter().sorted() {
            let etys = graph.etymologies(item);
            let reading = etys.map(|e| e.reading).unwrap_or_default();
            let n = graph.ety_count(item);
            let etymologies = (0..n)
                .map(|i| {
                    let ety = u8::try_from(i).unwrap_or(u8::MAX);
                    let edges = graph
                        .all_parent_edges(item)
                        .filter(|e| e.ety() == ety)
                        .sorted_by_key(EtyEdgeAccess::order)
                        .collect_vec();
                    let wiktionary = edges
                        .first()
                        .is_some_and(|e| e.basis() != EdgeBasis::Source);
                    let claims = graph
                        .claims
                        .get(&item)
                        .into_iter()
                        .flatten()
                        .filter(|c| c.ety == ety)
                        .map(|c| {
                            json!({
                                "agent": agent(c.agent),
                                "origin": agent(c.origin),
                                "stance": c.stance.as_ref(),
                                "relation": c.relation.as_ref(),
                                "certainty": c.certainty.as_ref(),
                                "mode": c.mode.map(|m| m.as_str()),
                                "compiled": c.compiled,
                                "assertion": c.assertion,
                                "locator": c.locator,
                            })
                        })
                        .collect_vec();
                    json!({
                        "kind": etys.and_then(|e| e.kinds.get(i)).map_or("derivation", |k| k.as_ref()),
                        "source_lang": etys.and_then(|e| e.kinds.get(i)).and_then(|k| match k {
                            EtyKind::LanguageOnly(l) => Some(match l.source {
                                SourceLang::Lang(lang) => lang.code(),
                                SourceLang::Family(family) => family.code(),
                            }),
                            EtyKind::Derivation | EtyKind::Unknown => None,
                        }),
                        "parents": edges.iter().map(|e| record_id(graph.item(e.parent()), &data.string_pool)).collect_vec(),
                        "mode": edges.first().map(|e| e.mode().as_str()),
                        "basis": edges.first().map(|e| e.basis().as_ref().to_owned()),
                        "certainty": edges.iter().map(|e| e.certainty()).max().map(|c| c.as_ref().to_owned()),
                        "wiktionary": wiktionary.then(|| graph.asserted(item, ety).and_then(|a| a.origin).map_or("agent:wiktionary-editors", agent)),
                        "claims": claims,
                    })
                })
                .collect_vec();
            writeln!(
                out,
                "{}",
                json!({
                    "word": record_id(graph.item(item), &data.string_pool),
                    "tier": reading.tier.as_ref(),
                    "disputed": reading.disputed,
                    "primary": etys.map_or(Some(0), |e| e.primary),
                    "etymologies": etymologies,
                })
            )?;
        }
        out.flush()?;
    }
    Ok(())
}
