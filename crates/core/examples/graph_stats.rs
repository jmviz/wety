//! Summary counts for comparing graph builds: cargo run --release --example graph_stats -- PATH
use std::collections::BTreeMap;

use petgraph::visit::IntoEdgeReferences;
use wety_core::{ety_graph::EtyEdgeAccess, items::Item, processed::Data};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("graph path required");
    let data = Data::deserialize(std::path::Path::new(&path))?;
    let graph = &data.graph;
    let (mut real, mut imputed, mut real_with_ety, mut imputed_with_ety) = (0, 0, 0, 0);
    let mut en_la = [(0, 0); 2]; // (items, with ety) for en, la
    for (id, item) in graph.iter() {
        let has_ety = graph.immediate_ety(id).is_some();
        match item {
            Item::Real(_) => {
                real += 1;
                real_with_ety += usize::from(has_ety);
                for (i, code) in ["en", "la"].iter().enumerate() {
                    if item.lang().code() == *code {
                        en_la[i].0 += 1;
                        en_la[i].1 += usize::from(has_ety);
                    }
                }
            }
            Item::Imputed(_) => {
                imputed += 1;
                imputed_with_ety += usize::from(has_ety);
            }
        }
    }
    let mut modes: BTreeMap<&str, usize> = BTreeMap::new();
    for edge in graph.graph.edge_references() {
        *modes.entry(edge.mode().as_str()).or_default() += 1;
    }
    println!("real items {real}, with an ety {real_with_ety}");
    println!("imputed items {imputed}, with an ety {imputed_with_ety}");
    println!("en items {}, with an ety {}", en_la[0].0, en_la[0].1);
    println!("la items {}, with an ety {}", en_la[1].0, en_la[1].1);
    println!("edges {}", graph.graph.edge_count());
    for (mode, n) in modes {
        println!("  {mode}: {n}");
    }
    Ok(())
}
