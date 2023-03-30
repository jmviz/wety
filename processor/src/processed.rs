use crate::{
    ety_graph::{EtyGraph, Graph, Progenitors},
    items::{Item, ItemId, RawItems},
    string_pool::StringPool,
    HashMap,
};

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};

use anyhow::{Ok, Result};
use flate2::{write::GzEncoder, Compression};
use indicatif::HumanDuration;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub(crate) string_pool: StringPool,
    pub(crate) items: Vec<Item>,
    pub(crate) graph: Graph,
    pub(crate) progenitors: HashMap<ItemId, Progenitors>,
}

impl Data {
    pub(crate) fn new(string_pool: StringPool, raw_items: RawItems, ety_graph: EtyGraph) -> Self {
        let mut items = raw_items.items.store.vec;
        let imputed_items = ety_graph.imputed_items.store.vec;
        items.extend(imputed_items);
        for (i, item) in items.iter().enumerate() {
            assert_eq!(i, item.id as usize);
        }

        let graph = ety_graph.graph;
        let progenitors = generate_progenitors(&items, &graph);
        Self {
            string_pool,
            items,
            graph,
            progenitors,
        }
    }

    pub(crate) fn serialize(&self, path: &Path) -> Result<()> {
        let t = Instant::now();
        println!("Serializing processed data to {}...", path.display());
        let file = File::create(path)?;
        let should_gz_compress = path.extension().is_some_and(|ext| ext == "gz");
        let writer: Box<dyn Write> = if should_gz_compress {
            Box::new(GzEncoder::new(file, Compression::best()))
        } else {
            Box::new(BufWriter::new(file))
        };
        serde_json::to_writer(writer, &self)?;
        println!("Finished. Took {}.", HumanDuration(t.elapsed()));
        Ok(())
    }
}

fn generate_progenitors(items: &[Item], graph: &Graph) -> HashMap<ItemId, Progenitors> {
    let mut progenitors = HashMap::default();
    for item in items.iter().map(|item| item.id) {
        if let Some(prog) = graph.get_progenitors(item) {
            progenitors.insert(item, prog);
        }
    }
    progenitors
}
