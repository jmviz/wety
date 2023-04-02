use crate::{
    ety_graph::{EtyGraph, Graph, Progenitors},
    items::{Item, ItemId, RawItems},
    langterm::Lang,
    string_pool::StringPool,
    HashMap, HashSet,
};

use std::{
    collections::hash_map::Entry,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    time::Instant,
};

use anyhow::{Ok, Result};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use fuzzy_trie::{Collector, FuzzyTrie};
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

    fn item_json(&self, item: ItemId) -> Value {
        let item = self.get_item(item);
        json!({
            "id": item.id,
            "ety_num": item.ety_num,
            "lang": item.lang.name(),
            "term": item.term.resolve(&self.string_pool),
            "imputed": item.is_imputed,
            "reconstructed": item.lang.is_reconstructed(),
            "url": item.url(&self.string_pool),
            "pos": item.pos.as_ref().map(|pos| pos.iter().map(|p| p.name()).collect_vec()),
            "gloss": item.gloss.as_ref().map(|gloss| gloss.iter().map(|g| g.to_string(&self.string_pool)).collect_vec()),
        })
    }

    #[must_use]
    pub fn expanded_item_json(&self, item_id: ItemId, filter_lang: Lang) -> Value {
        let item = self.get_item(item_id);
        let children = (item.lang != filter_lang).then_some(
            self.graph
                .get_head_children(item_id)
                .filter(|child| {
                    self.get_item(*child).lang == filter_lang
                        || self
                            .head_progeny_langs
                            .get(child)
                            .is_some_and(|langs| langs.contains(&filter_lang))
                })
                .map(|child| self.expanded_item_json(child, filter_lang))
                .collect_vec(),
        );
        json!({
            "item": self.item_json(item_id),
            "children": children,
        })
    }
}

pub struct Search {
    langs: FuzzyTrie<Lang>,
    terms: HashMap<Lang, FuzzyTrie<ItemId>>,
}

impl Data {
    #[must_use]
    pub fn build_search(&self) -> Search {
        let t = Instant::now();
        println!("Building search tries...");
        let mut langs = FuzzyTrie::new(1, true);
        let mut terms = HashMap::<Lang, FuzzyTrie<ItemId>>::default();
        for item in &self.items {
            let term = item.term.resolve(&self.string_pool);
            match terms.entry(item.lang) {
                Entry::Occupied(mut t) => {
                    t.get_mut().insert(term).insert(item.id);
                }
                Entry::Vacant(e) => {
                    langs
                        .insert(&item.lang.name().to_ascii_lowercase())
                        .insert(item.lang);
                    let t = e.insert(FuzzyTrie::new(1, true));
                    t.insert(term).insert(item.id);
                }
            }
        }
        println!("Finished. Took {:#?}.", t.elapsed());
        Search { langs, terms }
    }
}

struct LangMatch {
    distance: u8,
    lang: Lang,
}

impl LangMatch {
    fn json(&self) -> Value {
        json!({
            "distance": self.distance,
            "lang": self.lang.name(),
            "id": self.lang.id(),
        })
    }
}

pub struct LangMatches {
    matches: Vec<LangMatch>,
}

impl LangMatches {
    fn new() -> Self {
        Self { matches: vec![] }
    }

    pub fn sort(&mut self) {
        self.matches
            .sort_unstable_by(|a, b| a.distance.cmp(&b.distance));
    }

    pub fn json(&self) -> Value {
        json!({"matches": self.matches.iter().map(|m| m.json()).collect_vec()})
    }
}

impl<'a> Collector<'a, Lang> for LangMatches {
    fn push(&mut self, distance: u8, lang: &'a Lang) {
        self.matches.push(LangMatch {
            distance,
            lang: *lang,
        });
    }
}

struct ItemMatch {
    distance: u8,
    item: ItemId,
}

impl ItemMatch {
    fn json(&self, data: &Data) -> Value {
        json!({
            "distance": self.distance,
            "item": data.item_json(self.item),
        })
    }
}

#[derive(Default)]
pub struct ItemMatches {
    matches: Vec<ItemMatch>,
}

impl ItemMatches {
    fn new() -> Self {
        Self { matches: vec![] }
    }

    pub fn sort(&mut self, data: &Data) {
        self.matches.sort_unstable_by(|a, b| {
            if a.distance == b.distance {
                data.get_item(a.item)
                    .ety_num
                    .cmp(&data.get_item(b.item).ety_num)
            } else {
                a.distance.cmp(&b.distance)
            }
        });
    }

    pub fn json(&self, data: &Data) -> Value {
        json!({"matches": self.matches.iter().map(|m| m.json(data)).collect_vec()})
    }
}

impl<'a> Collector<'a, ItemId> for ItemMatches {
    fn push(&mut self, distance: u8, item: &'a ItemId) {
        self.matches.push(ItemMatch {
            distance,
            item: *item,
        });
    }
}

impl Search {
    #[must_use]
    pub fn langs(&self, lang: &str) -> LangMatches {
        let mut matches = LangMatches::new();
        if lang.chars().count() < 5 {
            self.langs.fuzzy_search(lang, &mut matches);
        } else {
            self.langs.prefix_fuzzy_search(lang, &mut matches);
        }
        matches
    }

    #[must_use]
    pub fn items(&self, lang: Lang, term: &str) -> ItemMatches {
        let mut matches = ItemMatches::new();
        if let Some(lang_terms) = self.terms.get(&lang) {
            if term.chars().count() < 5 {
                lang_terms.fuzzy_search(term, &mut matches);
            } else {
                lang_terms.prefix_fuzzy_search(term, &mut matches);
            }
        }
        matches
    }
}
