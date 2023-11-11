use crate::{
    ety_graph::{EtyEdgeAccess, EtyGraph, Progenitors},
    items::{Item, ItemId},
    languages::Lang,
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
use ngrammatic::{Corpus, CorpusBuilder, Pad};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub(crate) string_pool: StringPool,
    pub(crate) graph: EtyGraph,
    pub(crate) progenitors: HashMap<ItemId, Progenitors>,
    descendant_langs: HashMap<ItemId, HashSet<Lang>>,
}

// methods for use within processor
impl Data {
    pub(crate) fn new(string_pool: StringPool, graph: EtyGraph) -> Self {
        let progenitors = graph.all_progenitors();
        let descendant_langs = graph.all_descendant_langs();
        Self {
            string_pool,
            graph,
            progenitors,
            descendant_langs,
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
    fn item(&self, id: ItemId) -> &Item {
        self.graph.item(id)
    }

    fn term(&self, item: ItemId) -> &str {
        self.item(item).term().resolve(&self.string_pool)
    }

    fn ety_num(&self, item: ItemId) -> u8 {
        self.item(item).ety_num()
    }
}

// pub methods for server
impl Data {
    #[must_use]
    pub fn lang(&self, item: ItemId) -> Lang {
        self.item(item).lang()
    }

    #[must_use]
    pub fn ancestors_in_langs(&self, item: ItemId, langs: &[Lang]) -> Vec<ItemId> {
        self.graph.ancestors_in_langs(item, langs).collect()
    }

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

    fn item_json(&self, item_id: ItemId) -> Value {
        let item = self.item(item_id);
        json!({
            "id": item_id,
            "etyNum": item.ety_num(),
            "lang": item.lang().json(),
            "term": item.term().resolve(&self.string_pool),
            "imputed": item.is_imputed(),
            "reconstructed": item.is_reconstructed(),
            "url": item.url(&self.string_pool),
            "pos": item.pos().as_ref().map(|pos| pos.iter().map(|p| p.name()).collect_vec()),
            "gloss": item.gloss().as_ref().map(|gloss| gloss.iter().map(|g| g.to_string(&self.string_pool)).collect_vec()),
            "romanization": item.romanization().map(|r| r.resolve(&self.string_pool)),
        })
    }

    #[must_use]
    pub fn item_descendants_json(
        &self,
        item_id: ItemId,
        dist_lang: Lang,
        desc_langs: &[Lang],
        req_item_ancestors_within_desc_langs: &[ItemId],
        item_parent_id: Option<ItemId>,
        item_parent_ety_order: Option<u8>,
    ) -> Value {
        let item = self.item(item_id);
        let item_lang = item.lang();

        let children = self
            .graph
            .child_edges(item_id)
            .filter(|e| {
                let child = e.child();
                let child_lang = self.item(child).lang();
                // Make sure that the request item is included in the tree, even
                // if it would be disallowed otherwise.
                req_item_ancestors_within_desc_langs.contains(&item_id)
                // Include children that are in desc_langs, as long as they are
                // not the same language as their parent (which would indicate
                // an uninteresting derived term, like all the declensions of a
                // greek noun).
                    || (desc_langs.contains(&child_lang) && child_lang != item_lang)
                // Include children that are not themselves in desc_langs, but
                // who have descendants that are, as long as one of those
                // descendants is not the same language as item_lang. This
                // avoids cases like English -> German -> Swedish -> English
                // (where English is a requested desc_lang and German and
                // Swedish are not, and the first English item has no
                // descendants in any other desc_langs). The later English item
                // WILL be included if German is also a desc_lang though, even
                // though we don't really want this. How common are such
                // circuitous routes? If they are too common, we could track a
                // set of encountered_desc_langs for each call to this function
                // and filter based on that instead.
                    || self.descendant_langs.get(&child).is_some_and(|cdl| {
                        desc_langs
                            .iter()
                            .any(|dl| dl != &item_lang && cdl.contains(dl))
                    })
            })
            .map(|e| {
                self.item_descendants_json(
                    e.child(),
                    dist_lang,
                    desc_langs,
                    req_item_ancestors_within_desc_langs,
                    Some(item_id),
                    Some(e.order()),
                )
            })
            .collect_vec();

        let mut ety_mode = None;
        let other_parents = self
            .graph
            .parent_edges(item_id)
            .inspect(|e| {
                ety_mode = Some(e.mode());
            })
            .filter(|&e| !(item_parent_id.is_some_and(|id| id == e.parent())))
            .map(|e| {
                json!({
                    "item": self.item_json(e.parent()),
                    "etyOrder": e.order(),
                    "langDistance": self.item(e.parent()).lang().distance_from(dist_lang),
                })
            })
            .collect_vec();

        json!({
            "item": self.item_json(item_id),
            "children": children,
            "langDistance": item_lang.distance_from(dist_lang),
            "etyMode": ety_mode.map(|m| m.as_str()),
            "otherParents": other_parents,
            "parentEtyOrder": item_parent_ety_order,
        })
    }

    #[must_use]
    pub fn head_progenitor_tree(&self, item_id: ItemId, desc_langs: &[Lang]) -> Value {
        let lang = self.item(item_id).lang();
        let ancestors_within_desc_langs = self.ancestors_in_langs(item_id, desc_langs);
        self.progenitors
            .get(&item_id)
            .and_then(|p| p.head)
            .map_or_else(
                || {
                    self.item_descendants_json(
                        item_id,
                        lang,
                        desc_langs,
                        &ancestors_within_desc_langs,
                        None,
                        None,
                    )
                },
                |head| {
                    self.item_descendants_json(
                        head,
                        lang,
                        desc_langs,
                        &ancestors_within_desc_langs,
                        None,
                        None,
                    )
                },
            )
    }

    #[must_use]
    pub fn etymology_json(&self, item_id: ItemId, item_ety_order: u8, req_lang: Lang) -> Value {
        let mut ety_mode = None;
        let parents = self
            .graph
            .parent_edges(item_id)
            .map(|e| {
                ety_mode = Some(e.mode());
                self.etymology_json(e.parent(), e.order(), req_lang)
            })
            .collect_vec();

        json!({
            "item": self.item_json(item_id),
            "etyMode": ety_mode.map(|m| m.as_str()),
            "etyOrder": item_ety_order,
            "parents": parents,
            "langDistance": self.item(item_id).lang().distance_from(req_lang),
        })
    }
}

#[derive(Default)]
struct LangData {
    lang: Lang,
    items: usize,
}

pub struct Search {
    normalized_langs: HashMap<String, LangData>,
    langs: Corpus,
    terms: HashMap<Lang, FuzzyTrie<ItemId>>,
}

fn normalize_lang_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '(' | ')'))
        .map(|c| match c {
            '-' => ' ',
            _ => c.to_ascii_lowercase(),
        })
        .collect()
}

impl Data {
    #[must_use]
    pub fn build_search(&self) -> Search {
        let t = Instant::now();
        println!("Building search tries...");
        let mut normalized_langs = HashMap::<String, LangData>::default();
        let mut langs = CorpusBuilder::new()
            .arity(4)
            .pad_full(Pad::Auto)
            .key_trans(Box::new(normalize_lang_name))
            .finish();
        let mut terms = HashMap::<Lang, FuzzyTrie<ItemId>>::default();
        for (item_id, item) in self.graph.iter().filter(|(_, item)| !item.is_imputed()) {
            let norm_lang = normalize_lang_name(item.lang().name());
            let term = item.term().resolve(&self.string_pool);
            match terms.entry(item.lang()) {
                Entry::Occupied(mut t) => {
                    t.get_mut().insert(&term.to_lowercase()).insert(item_id);
                }
                Entry::Vacant(e) => {
                    let t = e.insert(FuzzyTrie::new(0, false));
                    t.insert(term).insert(item_id);
                }
            }
            if let Some(lang_data) = normalized_langs.get_mut(&norm_lang) {
                lang_data.items += 1;
            } else {
                normalized_langs.insert(
                    norm_lang,
                    LangData {
                        lang: item.lang(),
                        items: 1,
                    },
                );
                langs.add_text(item.lang().name());
            }
        }
        println!("Finished. Took {:#?}.", t.elapsed());
        Search {
            normalized_langs,
            langs,
            terms,
        }
    }
}

impl Search {
    #[must_use]
    pub fn langs(&self, lang: &str) -> Value {
        let mut matches = self
            .langs
            .search(lang, 0.4)
            .iter()
            .filter_map(|r| {
                self.normalized_langs
                    .get(&r.text)
                    .map(|lang_data| (r.similarity, lang_data))
            })
            .collect_vec();
        matches.sort_unstable_by(|a, b| {
            if (a.0 - b.0).abs() < 0.1 {
                b.1.items.cmp(&a.1.items)
            } else {
                b.0.total_cmp(&a.0)
            }
        });
        let matches = matches
            .iter()
            .map(|(_, lang_data)| lang_data.lang.json())
            .collect_vec();
        json!(matches)
    }
}

struct ItemMatch {
    distance: u8,
    item: ItemId,
}

impl ItemMatch {
    fn json(&self, data: &Data) -> Value {
        data.item_json(self.item)
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

    fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    fn sort(&mut self, data: &Data) {
        self.matches.sort_unstable_by(|a, b| {
            if a.distance == b.distance {
                let a_term = data.term(a.item);
                let b_term = data.term(b.item);
                let a_len = a_term.chars().count();
                let b_len = b_term.chars().count();
                if a_len == b_len {
                    if a_term == b_term {
                        data.ety_num(a.item).cmp(&data.ety_num(b.item))
                    } else {
                        // we want words that start with a lowercase to appear
                        // before words that start with an uppercase
                        b_term.cmp(a_term)
                    }
                } else {
                    a_len.cmp(&b_len)
                }
            } else {
                a.distance.cmp(&b.distance)
            }
        });
    }

    fn json(&self, data: &Data) -> Value {
        json!(self.matches.iter().map(|m| m.json(data)).collect_vec())
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
    pub fn items(&self, data: &Data, lang: Lang, term: &str) -> Value {
        let mut matches = ItemMatches::new();
        if let Some(lang_terms) = self.terms.get(&lang) {
            lang_terms.fuzzy_search(term, &mut matches);
            if matches.is_empty() && term.chars().count() > 5 {
                lang_terms.prefix_fuzzy_search(term, &mut matches);
            }
        }
        matches.sort(data);
        matches.json(data)
    }
}
