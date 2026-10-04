//! Stage-by-stage timing of the tree endpoints, without HTTP.
//!
//! Usage: cargo run --release --example bench -- <item_id>... [--desc-lang <code>]...
use std::{
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use serde_json::Value;
use wety_core::{Data, ItemId, Lang};

fn best<T>(reps: usize, mut f: impl FnMut() -> T) -> (Duration, T) {
    let mut best = Duration::MAX;
    let mut out = None;
    for _ in 0..reps {
        let t = Instant::now();
        let v = f();
        let d = t.elapsed();
        if d < best {
            best = d;
            out = Some(v);
        }
    }
    (best, out.unwrap())
}

fn count_nodes(v: &Value) -> usize {
    match v {
        Value::Array(a) => a.iter().map(count_nodes).sum(),
        Value::Object(o) => {
            1 + o
                .get("children")
                .or_else(|| o.get("parents"))
                .map_or(0, count_nodes)
        }
        _ => 0,
    }
}

/// Child edges the filter in `item_descendants_json_inner` had to inspect:
/// the out-degree (in the child direction) of every node that made it into
/// the tree.
fn scanned_edges(data: &Data, v: &Value) -> usize {
    match v {
        Value::Array(a) => a.iter().map(|x| scanned_edges(data, x)).sum(),
        Value::Object(o) => {
            let id = ItemId::new(o["item"]["id"].as_u64().unwrap() as usize);
            data.graph.child_edges(id).count()
                + o.get("children").map_or(0, |c| scanned_edges(data, c))
        }
        _ => 0,
    }
}

/// The same traversal + filter as `item_descendants_json_inner`, but only
/// counting nodes: isolates graph-walk cost from `Value` construction.
fn walk_only(data: &Data, item_id: ItemId, desc_langs: &[Lang], anc: &[ItemId]) -> usize {
    use wety_core::ety_graph::EtyEdgeAccess;
    let item_lang = data.lang(item_id);
    let mut n = 1;
    for e in data.graph.child_edges(item_id) {
        let child = e.child();
        let child_lang = data.lang(child);
        let keep = anc.contains(&item_id)
            || (desc_langs.contains(&child_lang) && child_lang != item_lang)
            || desc_langs
                .iter()
                .any(|&dl| dl != item_lang && data.descendant_langs.contains(child, dl));
        if keep {
            n += walk_only(data, child, desc_langs, anc);
        }
    }
    n
}

fn brotli(bytes: &[u8], q: u32) -> usize {
    let mut out = Vec::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: q as i32,
        ..Default::default()
    };
    let mut w = brotli::CompressorWriter::with_params(&mut out, 4096, &params);
    w.write_all(bytes).unwrap();
    drop(w);
    out.len()
}

fn gzip(bytes: &[u8], level: u32) -> usize {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(level));
    e.write_all(bytes).unwrap();
    e.finish().unwrap().len()
}

fn main() {
    let mut items: Vec<ItemId> = vec![];
    let mut desc_langs: Vec<Lang> = vec![];
    let mut worst = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--desc-lang" {
            desc_langs.push(Lang::from_code(&args.next().unwrap()).unwrap());
        } else if a == "--worst" {
            worst = true;
        } else {
            items.push(ItemId::new(a.parse().unwrap()));
        }
    }
    if desc_langs.is_empty() {
        desc_langs.push(Lang::from_code("en").unwrap());
    }

    let data = Data::deserialize(Path::new("data/wety.json")).unwrap();
    if worst {
        // The progenitor with the most raw descendant edges; bench the
        // cognates request from its perspective (its own lang as dist lang).
        let t = Instant::now();
        let mut seen = std::collections::HashSet::new();
        let mut top: Vec<(usize, ItemId)> = vec![];
        for p in data.progenitors.values() {
            for &pi in p.items.iter() {
                if seen.insert(pi) {
                    top.push((data.graph.descendant_edges(pi).count(), pi));
                }
            }
        }
        top.sort_unstable_by(|a, b| b.cmp(a));
        println!(
            "scanned {} distinct progenitors in {:?}",
            seen.len(),
            t.elapsed()
        );
        for (n, pi) in top.iter().take(5) {
            println!(
                "  {} ({}, {}): {n} raw descendant edges",
                pi.index(),
                data.graph.item(*pi).term().resolve(&data.string_pool),
                data.lang(*pi).name()
            );
        }
        items.extend(top.iter().take(2).map(|(_, pi)| *pi));
    }
    println!(
        "desc_langs: {:?}",
        desc_langs.iter().map(|l| l.name()).collect::<Vec<_>>()
    );

    for &item in &items {
        let lang = data.lang(item);
        println!(
            "\n=== item {} ({}, {}) ===",
            item.index(),
            data.graph.item(item).term().resolve(&data.string_pool),
            lang.name()
        );
        let (t_anc, anc) = best(5, || data.ancestors_in_langs(item, &desc_langs));
        println!("ancestors_in_langs: {t_anc:?} ({} ancestors)", anc.len());

        let n_prog = data.progenitors.get(&item).map_or(0, |p| p.items.len());
        let raw_desc: usize = data.progenitors.get(&item).map_or(0, |p| {
            p.items
                .iter()
                .map(|&pi| data.graph.descendant_edges(pi).count())
                .sum()
        });
        println!("progenitors: {n_prog}, raw descendant edges under them: {raw_desc}");

        let (t_walk, walked) = best(3, || {
            data.progenitors.get(&item).map_or(0, |p| {
                p.items
                    .iter()
                    .map(|&pi| walk_only(&data, pi, &desc_langs, &anc))
                    .sum()
            })
        });
        println!("cognates walk+filter only: {t_walk:?} ({walked} nodes)");
        let (t_build, cog) = best(3, || data.item_cognates_json(item, lang, &desc_langs, &anc, false));
        let (t_ser, bytes) = best(3, || serde_json::to_vec(&cog).unwrap());
        let cog: Value = serde_json::from_slice(&bytes).unwrap();
        let nodes = count_nodes(&cog);
        println!(
            "cognates build typed tree: {t_build:?} ({nodes} tree nodes, {} child edges scanned)",
            scanned_edges(&data, &cog)
        );
        println!("serde_json::to_vec: {t_ser:?} ({} bytes)", bytes.len());
        let (t_both, _) = best(3, || {
            serde_json::to_vec(&data.item_cognates_json(item, lang, &desc_langs, &anc, false)).unwrap()
        });
        println!("build + serialize (what a cache miss costs): {t_both:?}");
        for q in [11, 6, 4, 2] {
            let (t, n) = best(1, || brotli(&bytes, q));
            println!("  brotli q{q}: {t:?} -> {n} bytes");
        }
        for l in [6, 1] {
            let (t, n) = best(1, || gzip(&bytes, l));
            println!("  gzip l{l}: {t:?} -> {n} bytes");
        }

        let (t_d, desc) = best(3, || {
            serde_json::to_vec(&data.item_descendants_json(item, lang, &desc_langs, &anc, false)).unwrap()
        });
        println!(
            "descendants build + serialize: {t_d:?} ({} nodes, {} bytes)",
            count_nodes(&serde_json::from_slice(&desc).unwrap()),
            desc.len()
        );
        let (t_e, ety) = best(3, || {
            serde_json::to_vec(&data.item_etymology_json(item, 0, lang)).unwrap()
        });
        println!(
            "etymology build + serialize: {t_e:?} ({} nodes)",
            count_nodes(&serde_json::from_slice(&ety).unwrap())
        );
    }
}
