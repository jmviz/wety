use crate::{
    ety_graph::{EtyGraph, Graph, Progenitors},
    items::{Item, ItemId, RawItems},
    langterm::Lang,
    string_pool::StringPool,
    HashMap, HashSet,
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
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub(crate) string_pool: StringPool,
    pub(crate) items: Vec<Item>,
    pub(crate) graph: Graph,
    pub(crate) progenitors: HashMap<ItemId, Progenitors>,
    head_progeny_langs: HashMap<ItemId, HashSet<Lang>>,
}

// methods for use within processor
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
        let head_progeny_langs = graph.get_all_head_progeny_langs(&items);
        Self {
            string_pool,
            items,
            graph,
            progenitors,
            head_progeny_langs,
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

// private methods for use within pub methods below
impl Data {
    fn get_item(&self, item: ItemId) -> &Item {
        &self.items[item as usize]
    }
}

// pub methods for server
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

    #[must_use]
    pub fn expand(&self, id: ItemId) -> Value {
        let filter_lang = crate::langterm::Lang::try_from("en").unwrap();
        let item = self.get_item(id);
        let children = (item.lang != filter_lang).then_some(
            self.graph
                .get_head_children(id)
                .filter(|child| {
                    self.get_item(*child).lang == filter_lang
                        || self
                            .head_progeny_langs
                            .get(child)
                            .is_some_and(|langs| langs.contains(&filter_lang))
                })
                .map(|child| self.expand(child))
                .collect_vec(),
        );
        json!({
            "id": item.id,
            "ety": item.ety_num,
            "lang": item.lang.name(),
            "term": item.term.resolve(&self.string_pool),
            "imputed": item.is_imputed,
            "url": item.url(&self.string_pool),
            "pos": item.pos.as_ref().map(|pos| pos.iter().map(|p| p.name()).collect_vec()),
            "gloss": item.gloss.as_ref().map(|gloss| gloss.iter().map(|g| g.to_string(&self.string_pool)).collect_vec()),
            "children": children,
        })
    }
}
