use crate::{
    ety_graph::{EtyGraph, Progenitors},
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
    head_progeny_langs: HashMap<ItemId, HashSet<Lang>>,
}

// methods for use within processor
impl Data {
    pub(crate) fn new(string_pool: StringPool, graph: EtyGraph) -> Self {
        let progenitors = graph.get_all_progenitors();
        let head_progeny_langs = graph.get_all_head_progeny_langs();
        Self {
            string_pool,
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
    fn get(&self, item: ItemId) -> &Item {
        self.graph.get(item)
    }

    fn term(&self, item: ItemId) -> &str {
        self.get(item).term().resolve(&self.string_pool)
    }

    fn ety_num(&self, item: ItemId) -> u8 {
        self.get(item).ety_num()
    }
}

// pub methods for server
impl Data {
    #[must_use]
    pub fn lang(&self, item: ItemId) -> Lang {
        self.get(item).lang()
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
        let item = self.get(item_id);
        json!({
            "id": item_id,
            "etyNum": item.ety_num(),
            "lang": item.lang().name(),
            "term": item.term().resolve(&self.string_pool),
            "etyMode": self.graph.get_ety_mode(item_id).map(|m| {let ms: &'static str = m.into(); ms}),
            "imputed": item.is_imputed(),
            "reconstructed": item.is_reconstructed(),
            "url": item.url(&self.string_pool),
            "pos": item.pos().as_ref().map(|pos| pos.iter().map(|p| p.name()).collect_vec()),
            "gloss": item.gloss().as_ref().map(|gloss| gloss.iter().map(|g| g.to_string(&self.string_pool)).collect_vec()),
            "romanization": item.romanization().map(|r| r.resolve(&self.string_pool)),
        })
    }

    #[must_use]
    pub fn expanded_item_json(
        &self,
        item_id: ItemId,
        input_lang: Lang,
        include_langs: &[Lang],
    ) -> Value {
        let item = self.get(item_id);
        let item_lang = item.lang();
        let children = (!include_langs.contains(&item_lang)).then_some(
            self.graph
                .get_head_children(item_id)
                .filter(|(child_id, child)| {
                    include_langs.contains(&child.lang())
                        || self
                            .head_progeny_langs
                            .get(child_id)
                            .is_some_and(|langs| include_langs.iter().any(|il| langs.contains(il)))
                })
                .map(|(child_id, _)| self.expanded_item_json(child_id, input_lang, include_langs))
                .collect_vec(),
        );
        json!({
            "item": self.item_json(item_id),
            "children": children,
            "langDistance": item_lang.distance_from(input_lang),
        })
    }

    #[must_use]
    pub fn head_progenitor_tree(&self, item_id: ItemId, include_langs: &[Lang]) -> Value {
        let input_lang = self.get(item_id).lang();
        self.progenitors
            .get(&item_id)
            .and_then(|p| p.head)
            .map_or_else(
                || self.expanded_item_json(item_id, input_lang, include_langs),
                |head| self.expanded_item_json(head, input_lang, include_langs),
            )
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
            .map(|(similarity, lang_data)| {
                json!({
                    "id": lang_data.lang.id(),
                    "code": lang_data.lang.code(),
                    "name": lang_data.lang.name(),
                    "similarity": similarity,
                    "items": lang_data.items,
                })
            })
            .collect_vec();
        json!({ "matches": matches })
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
