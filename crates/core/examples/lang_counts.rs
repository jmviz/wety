//! Items per language in a graph, written as JSON to OUT: `{"<language id>": <items>,
//! ...}`, the ids wety_core gives languages (crates/core/src/languages.rs). For tools
//! that read the graph without wety_core, such as the dating targets:
//! cargo run --release -p wety-core --example lang_counts -- GRAPH OUT
use std::collections::BTreeMap;

use wety_core::processed::Data;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("graph path required");
    let out = std::env::args().nth(2).expect("output path required");
    let data = Data::deserialize(std::path::Path::new(&path))?;
    let mut counts: BTreeMap<u16, usize> = BTreeMap::new();
    for (_, item) in data.graph.iter() {
        *counts.entry(item.lang().id()).or_default() += 1;
    }
    std::fs::write(out, serde_json::to_string(&counts)?)?;
    Ok(())
}
