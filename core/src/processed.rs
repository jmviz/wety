use crate::{
    HashMap, HashSet,
    ety_graph::{EtyEdgeAccess, EtyGraph, Progenitors},
    items::{Item, ItemId},
    languages::Lang,
    string_pool::StringPool,
};

use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    time::Instant,
};

use anyhow::{Ok, Result};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use itertools::Itertools;
use ngrammatic::{Corpus, CorpusBuilder, Pad};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use trie_rs::map::{Trie as TrieMap, TrieBuilder as TrieMapBuilder};

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub string_pool: StringPool,
    pub graph: EtyGraph,
    pub progenitors: HashMap<ItemId, Progenitors>,
    pub descendant_langs: HashMap<ItemId, HashSet<Lang>>,
}

impl Data {
    #[must_use]
    pub fn new(string_pool: StringPool, graph: EtyGraph) -> Self {
        let progenitors = graph.all_progenitors();
        let descendant_langs = graph.all_descendant_langs();
        Self {
            string_pool,
            graph,
            progenitors,
            descendant_langs,
        }
    }

    /// # Errors
    ///
    /// Returns an error if the output file cannot be created or written.
    pub fn serialize(&self, path: &Path) -> Result<()> {
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
        println!("Finished. Took {:.2?}.", t.elapsed());
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
    ) -> Value {
        self.item_descendants_json_inner(
            item_id,
            dist_lang,
            desc_langs,
            req_item_ancestors_within_desc_langs,
            None,
            None,
        )
    }

    fn item_descendants_json_inner(
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
                req_item_ancestors_within_desc_langs.contains(&item_id)
                    || (desc_langs.contains(&child_lang) && child_lang != item_lang)
                    || self.descendant_langs.get(&child).is_some_and(|cdl| {
                        desc_langs
                            .iter()
                            .any(|dl| dl != &item_lang && cdl.contains(dl))
                    })
            })
            .map(|e| {
                self.item_descendants_json_inner(
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
            .filter(|&e| item_parent_id != Some(e.parent()))
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
    pub fn item_cognates_json(
        &self,
        item_id: ItemId,
        dist_lang: Lang,
        desc_langs: &[Lang],
        req_item_ancestors_within_desc_langs: &[ItemId],
    ) -> Value {
        self.progenitors.get(&item_id).map_or_else(
            || json!([]),
            |progenitors| {
                json!(
                    progenitors
                        .items
                        .iter()
                        .map(|&p| {
                            self.item_descendants_json(
                                p,
                                dist_lang,
                                desc_langs,
                                req_item_ancestors_within_desc_langs,
                            )
                        })
                        .collect_vec()
                )
            },
        )
    }

    #[must_use]
    pub fn item_etymology_json(
        &self,
        item_id: ItemId,
        item_ety_order: u8,
        req_lang: Lang,
    ) -> Value {
        let mut ety_mode = None;
        let parents = self
            .graph
            .parent_edges(item_id)
            .map(|e| {
                ety_mode = Some(e.mode());
                self.item_etymology_json(e.parent(), e.order(), req_lang)
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
    terms: HashMap<Lang, TrieMap<u8, Vec<ItemId>>>,
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
        // Pre-aggregate items by (lang, lowered_term) since the map trie
        // only stores one value per key.
        let mut term_items = HashMap::<Lang, HashMap<String, Vec<ItemId>>>::default();
        for (item_id, item) in self.graph.iter().filter(|(_, item)| !item.is_imputed()) {
            let norm_lang = normalize_lang_name(item.lang().name());
            let term = item.term().resolve(&self.string_pool);
            let lowered = term.to_lowercase();
            term_items
                .entry(item.lang())
                .or_default()
                .entry(lowered)
                .or_default()
                .push(item_id);
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
        let mut terms = HashMap::<Lang, TrieMap<u8, Vec<ItemId>>>::default();
        for (lang, lang_terms) in term_items {
            let mut builder = TrieMapBuilder::new();
            for (term, ids) in lang_terms {
                builder.push(term, ids);
            }
            terms.insert(lang, builder.build());
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

const MAX_SEARCH_RESULTS: usize = 20;

impl Search {
    #[must_use]
    pub fn items(&self, data: &Data, lang: Lang, term: &str) -> Value {
        let lowered = term.to_lowercase();
        let mut items: Vec<ItemId> = Vec::new();
        if let Some(lang_trie) = self.terms.get(&lang) {
            // predictive_search returns results in lexicographic byte order,
            // so shorter terms come first. Break early once we have enough.
            for (_, item_ids) in lang_trie.predictive_search::<String, _>(&lowered) {
                items.extend(item_ids.iter().copied());
                if items.len() >= MAX_SEARCH_RESULTS {
                    break;
                }
            }
        }
        items.sort_unstable_by(|&a, &b| {
            let a_term = data.term(a);
            let b_term = data.term(b);
            let a_len = a_term.chars().count();
            let b_len = b_term.chars().count();
            if a_len == b_len {
                if a_term == b_term {
                    data.ety_num(a).cmp(&data.ety_num(b))
                } else {
                    b_term.cmp(a_term)
                }
            } else {
                a_len.cmp(&b_len)
            }
        });
        items.truncate(MAX_SEARCH_RESULTS);
        json!(items.iter().map(|&id| data.item_json(id)).collect_vec())
    }
}
