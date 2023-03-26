use crate::{
    etymology_templates::EtyMode,
    items::{Item, ItemId, ItemStore},
    langterm::LangTerm,
    pos::Pos,
};

use std::ops::Index;

use anyhow::{anyhow, Ok, Result};
use hashbrown::{HashMap, HashSet};
use itertools::{izip, Itertools};
use petgraph::{
    algo::greedy_feedback_arc_set,
    stable_graph::{EdgeIndex, NodeIndex, StableDiGraph},
    visit::EdgeRef,
};
use serde::{Deserialize, Serialize};
use serde_json_any_key::any_key_map;

// Quite often an etymology section on wiktionary will have multiple valid
// templates that don't actually link to anything (because the term has no page,
// or doesn't have the relevant page section), before an eventual valid template
// that does. See e.g. https://en.wiktionary.org/wiki/arsenic#English. The first
// two templates linking to Middle English and Middle French terms are both
// valid for our purposes, and the pages exist, but the language sections they
// link to do not exist. Therefore, both of these terms will not correspond to a
// findable item, and so the current procedure will give an ety of None. Instead
// we can go through the templates until we find the template linking Latin
// https://en.wiktionary.org/wiki/arsenicum#Latin, where the page and section
// both exist.
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct ImputedItems {
    pub(crate) store: ItemStore,
    #[serde(with = "any_key_map")]
    pub(crate) langterms: HashMap<LangTerm, ItemId>,
}

impl ImputedItems {
    pub(crate) fn new(start_id: ItemId) -> Self {
        Self {
            store: ItemStore::new(start_id),
            ..Default::default()
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.store.len()
    }

    pub(crate) fn add(&mut self, item: Item) -> ItemId {
        let langterm = item.langterm();
        if let Some(&item_id) = self.langterms.get(&langterm) {
            return item_id;
        }
        let item_id = self.store.add(item);
        self.langterms.insert(langterm, item_id);
        item_id
    }

    pub(crate) fn get_item_id(&self, langterm: LangTerm) -> Option<ItemId> {
        self.langterms.get(&langterm).copied()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Item> {
        self.store.iter()
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EtyLink {
    pub(crate) mode: EtyMode,
    pub(crate) order: u8,
    pub(crate) head: bool,
    confidence: f32,
}

pub(crate) struct ImmediateEty {
    pub(crate) items: Vec<ItemId>,
    pub(crate) head: u8,
    pub(crate) mode: EtyMode,
}

impl ImmediateEty {
    fn head(&self) -> ItemId {
        self.items[self.head as usize]
    }
}

pub(crate) struct Progenitors {
    pub(crate) items: HashSet<ItemId>,
    pub(crate) head: ItemId,
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct EtyGraph {
    pub(crate) imputed_items: ImputedItems,
    pub(crate) graph: StableDiGraph<ItemId, EtyLink>,
    #[serde(with = "any_key_map")]
    pub(crate) index: HashMap<ItemId, NodeIndex>,
}

impl EtyGraph {
    pub(crate) fn new(start_id: ItemId) -> Self {
        Self {
            imputed_items: ImputedItems::new(start_id),
            ..Default::default()
        }
    }

    pub(crate) fn get_imputed_item_id(&self, langterm: LangTerm) -> Option<ItemId> {
        self.imputed_items.get_item_id(langterm)
    }

    fn get_index(&self, item_id: ItemId) -> NodeIndex {
        *self
            .index
            .get(&item_id)
            .expect("index previously added item")
    }

    pub(crate) fn get_immediate_ety(&self, item_id: ItemId) -> Option<ImmediateEty> {
        let item_index = self.get_index(item_id);
        let mut ety_item_ids = vec![];
        let mut order = vec![];
        // Next two lines are dummy assignments. If there are any parents in the
        // ety_graph, they will get overwritten with correct values. If no
        // parents, they will not get returned.
        let mut head = 0;
        let mut mode = EtyMode::Derived;
        for (ety_link, &ety_item_id) in self
            .graph
            .edges(item_index)
            .map(|e| (e.weight(), self.graph.index(e.target())))
        {
            ety_item_ids.push(ety_item_id);
            order.push(ety_link.order);
            mode = ety_link.mode;
            if ety_link.head {
                head = ety_link.order;
            }
        }
        ety_item_ids = order
            .iter()
            .map(|&ord| ety_item_ids[ord as usize])
            .collect();
        (!ety_item_ids.is_empty()).then_some(ImmediateEty {
            items: ety_item_ids,
            mode,
            head,
        })
    }

    pub(crate) fn get_progenitors(&self, item: ItemId) -> Option<Progenitors> {
        struct Tracker {
            unexpanded: Vec<ItemId>,
            progenitors: HashSet<ItemId>,
            head: ItemId,
        }
        fn recurse(ety_graph: &EtyGraph, t: &mut Tracker) {
            while let Some(item) = t.unexpanded.pop() {
                if let Some(immediate_ety) = ety_graph.get_immediate_ety(item) {
                    let ety_head = immediate_ety.head();
                    for &ety_item in &immediate_ety.items {
                        if t.head == item && ety_item == ety_head {
                            t.head = ety_head;
                        }
                        t.unexpanded.push(ety_item);
                    }
                    recurse(ety_graph, t);
                } else {
                    t.progenitors.insert(item);
                }
            }
        }
        let immediate_ety = self.get_immediate_ety(item)?;
        let head = immediate_ety.head();
        let mut t = Tracker {
            unexpanded: immediate_ety.items,
            progenitors: HashSet::new(),
            head,
        };
        recurse(self, &mut t);
        let head = t.head;
        Some(Progenitors {
            items: t.progenitors,
            head,
        })
    }

    pub(crate) fn add(&mut self, item_id: ItemId) {
        if !self.index.contains_key(&item_id) {
            let node_index = self.graph.add_node(item_id);
            self.index.insert(item_id, node_index);
        }
    }

    pub(crate) fn add_imputed(&mut self, langterm: LangTerm, pos: Option<Pos>) -> ItemId {
        let item = Item::new_imputed(langterm, pos);
        let item_id = self.imputed_items.add(item);
        self.add(item_id);
        item_id
    }

    pub(crate) fn add_ety(
        &mut self,
        item: ItemId,
        mode: EtyMode,
        head: u8,
        ety_items: &[ItemId],
        confidences: &[f32],
    ) {
        let item_index = self.get_index(item);
        // StableGraph allows adding multiple parallel edges from one node to
        // another. So we have to be careful to check for any already existing
        // ety links. If there are some, we keep them and don't add any new
        // ones, unless the least confidence for the new ety links is greater
        // than the greatest confidence for the old ety links. In that case, we
        // delete all the old ones and add the new ones in their stead.
        let mut old_edges = self.graph.edges(item_index).peekable();
        if old_edges.peek().is_some() {
            let min_new_confidence = confidences
                .iter()
                .min_by(|a, b| a.total_cmp(b))
                .expect("at least one");
            let max_old_confidence = old_edges
                .map(|e| e.weight().confidence)
                .max_by(|a, b| a.total_cmp(b))
                .expect("at least one");
            if min_new_confidence > &max_old_confidence {
                let old_edge_ids = self.graph.edges(item_index).map(|e| e.id()).collect_vec();
                for old_edge_id in old_edge_ids {
                    self.graph.remove_edge(old_edge_id);
                }
            } else {
                return;
            }
        }

        for (i, &ety_item, &confidence) in izip!(0u8.., ety_items, confidences) {
            let ety_item_index = self.get_index(ety_item);
            let ety_link = EtyLink {
                mode,
                order: i,
                head: head == i,
                confidence,
            };
            self.graph.add_edge(item_index, ety_item_index, ety_link);
        }
    }
    pub(crate) fn remove_cycles(&mut self) -> Result<()> {
        print!("  Checking for ety link feedback arc set... ");
        let fas: Vec<EdgeIndex> = greedy_feedback_arc_set(&self.graph)
            .map(|e| e.id())
            .collect();
        if fas.is_empty() {
            println!("Found none.");
        } else {
            println!("Found set of size {}. Removing.", fas.len());
            for edge in fas {
                let (source, _) = self
                    .graph
                    .edge_endpoints(edge)
                    .ok_or_else(|| anyhow!("feedback arc set edge endpoints not found"))?;
                // We take not only the edges forming the fas, but all edges
                // that share the same source of any of the fas edges (recall:
                // the edge source is a child and the edge target is an
                // etymological parent). This is to ensure there are no
                // degenerate etys in the graph once we remove the edges.
                let edges_from_source: Vec<EdgeIndex> =
                    self.graph.edges(source).map(|e| e.id()).collect();
                for e in edges_from_source {
                    self.graph.remove_edge(e);
                }
            }
        }
        Ok(())
    }
}
