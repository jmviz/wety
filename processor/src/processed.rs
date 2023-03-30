use crate::{
    ety_graph::{EtyGraph, Graph, Progenitors, Progeny},
    items::{Item, ItemId, RawItems},
    string_pool::StringPool,
    HashMap,
};

use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    time::Instant,
};

use anyhow::{Ok, Result};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use indicatif::HumanDuration;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub(crate) string_pool: StringPool,
    pub(crate) items: Vec<Item>,
    pub(crate) graph: Graph,
    pub(crate) progenitors: HashMap<ItemId, Progenitors>,
    head_progeny: HashMap<ItemId, Progeny>,
}

// crate implementations
impl Data {
    pub(crate) fn new(string_pool: StringPool, raw_items: RawItems, ety_graph: EtyGraph) -> Self {
        let mut items = raw_items.items.store.vec;
        let imputed_items = ety_graph.imputed_items.store.vec;
        items.extend(imputed_items);
        for (i, item) in items.iter().enumerate() {
            assert_eq!(i, item.id as usize);
        }
        let graph = ety_graph.graph;
        let progenitors = graph.get_all_progenitors(&items);
        let head_progeny = graph.get_all_head_progeny(&items);
        Self {
            string_pool,
            items,
            graph,
            progenitors,
            head_progeny,
        }
    }

    pub(crate) fn serialize(&self, path: &Path) -> Result<()> {
        let t = Instant::now();
        println!("Serializing processed data to {}...", path.display());
        let file = File::create(path)?;
        let should_gz_compress = path.extension().is_some_and(|ext| ext == "gz");
        let writer: Box<dyn Write> = if should_gz_compress {
            Box::new(GzEncoder::new(file, Compression::fast()))
        } else {
            Box::new(BufWriter::new(file))
        };
        serde_json::to_writer(writer, &self)?;
        println!("Finished. Took {}.", HumanDuration(t.elapsed()));
        Ok(())
    }
}

// pub implementations for server
impl Data {
    /// # Errors
    ///
    /// Will return `Err` if any unexpected issue arises in the deserialization.
    pub fn deserialize(path: &Path) -> Result<Self> {
        let t = Instant::now();
        println!("Deserializing processed data {}...", path.display());
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let is_gz_compressed = path.extension().is_some_and(|ext| ext == "gz");
        let uncompressed: Box<dyn Read> = if is_gz_compressed {
            Box::new(GzDecoder::new(reader))
        } else {
            Box::new(reader)
        };
        let data = serde_json::from_reader(uncompressed)?;
        println!("Finished. Took {:#?}.", t.elapsed());
        Ok(data)
    }
}
