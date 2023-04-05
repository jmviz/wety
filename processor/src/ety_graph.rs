use crate::{
    etymology_templates::EtyMode,
    items::{Item, ItemId, ItemStore},
    langterm::{Lang, LangTerm},
    pos::Pos,
    HashMap, HashSet,
};

use std::ops::Index;

use anyhow::{anyhow, Ok, Result};
use itertools::{izip, Itertools};
use petgraph::{
    algo::greedy_feedback_arc_set,
    stable_graph::{EdgeIndex, StableDiGraph},
    visit::EdgeRef,
    Direction,
};
use serde::{Deserialize, Serialize};

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
#[derive(Default)]
pub(crate) struct ImputedItems {
    pub(crate) store: ItemStore,
    pub(crate) langterms: HashMap<LangTerm, ItemId>,
}

impl ImputedItems {
    pub(crate) fn new(start_id: ItemId) -> Self {
        Self {
            store: ItemStore::new(start_id),
            ..Default::default()
        }
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
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EtyLink {
    pub(crate) mode: EtyMode,
    pub(crate) order: u8,
    pub(crate) head: bool,
    confidence: f32,
}

// the parents of some item
pub(crate) struct ImmediateEty {
    pub(crate) items: Vec<ItemId>,
    pub(crate) head: Option<u8>,
    pub(crate) mode: EtyMode,
}

impl ImmediateEty {
    fn head(&self) -> Option<ItemId> {
        self.head.map(|head| self.items[head as usize])
    }
}

// all the terminal nodes of some item's ancestry tree
#[derive(Serialize, Deserialize)]
pub(crate) struct Progenitors {
    pub(crate) items: Box<[ItemId]>,
    // the terminal node reached by following the "head" parent at each step
    pub(crate) head: Option<ItemId>,
}

impl Progenitors {
    fn new(mut progenitors: HashSet<ItemId>, head: Option<ItemId>) -> Self {
        Self {
            items: progenitors.drain().collect_vec().into_boxed_slice(),
            head,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Graph {
    pub(crate) graph: StableDiGraph<ItemId, EtyLink, ItemId>,
}

impl Graph {
    pub(crate) fn add(&mut self, item_id: ItemId) {
        let node_index = self.graph.add_node(item_id);
        // Items are always inserted into the graph in the order they're
        // created. We rely on this invariance so that we can use ItemId to
        // directly index into the graph instead of storing a separate ItemId ->
        // NodeIndex map.
        assert_eq!(node_index, item_id.into());
    }

    pub(crate) fn get_immediate_ety(&self, item_id: ItemId) -> Option<ImmediateEty> {
        let mut ety_item_ids = vec![];
        let mut order = vec![];
        // Next two lines are dummy assignments. If there are any parents in the
        // ety_graph, they will get overwritten with correct values. If no
        // parents, they will not get returned.
        let mut head = None;
        let mut mode = EtyMode::Derived;
        for (ety_link, &ety_item_id) in self
            .graph
            .edges(item_id.into())
            .map(|e| (e.weight(), self.graph.index(e.target())))
        {
            ety_item_ids.push(ety_item_id);
            order.push(ety_link.order);
            mode = ety_link.mode;
            if ety_link.head {
                head = Some(ety_link.order);
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

    pub(crate) fn get_ety_mode(&self, item_id: ItemId) -> Option<EtyMode> {
        self.graph
            .edges(item_id.into())
            .next()
            .map(|e| e.weight().mode)
    }
}

struct Tracker {
    unexpanded: Vec<ItemId>,
    progenitors: HashSet<ItemId>,
    head: Option<ItemId>,
}

impl Graph {
    pub(crate) fn get_progenitors(&self, item: ItemId) -> Option<Progenitors> {
        let immediate_ety = self.get_immediate_ety(item)?;
        let head = immediate_ety.head();
        let mut t = Tracker {
            unexpanded: immediate_ety.items,
            progenitors: HashSet::default(),
            head,
        };
        self.get_progenitors_recurse(&mut t);
        let head = t.head;
        Some(Progenitors::new(t.progenitors, head))
    }

    fn get_progenitors_recurse(&self, t: &mut Tracker) {
        while let Some(item) = t.unexpanded.pop() {
            if let Some(immediate_ety) = self.get_immediate_ety(item) {
                let ety_head = immediate_ety.head();
                for &ety_item in &immediate_ety.items {
                    if t.head.is_some_and(|h| h == item)
                        && ety_head.is_some_and(|eh| eh == ety_item)
                    {
                        t.head = ety_head;
                    }
                    t.unexpanded.push(ety_item);
                }
                self.get_progenitors_recurse(t);
            } else {
                t.progenitors.insert(item);
            }
        }
    }

    pub(crate) fn get_all_progenitors(&self, items: &[Item]) -> HashMap<ItemId, Progenitors> {
        let mut progenitors = HashMap::default();
        for item in items.iter().map(|item| item.id) {
            if let Some(prog) = self.get_progenitors(item) {
                progenitors.insert(item, prog);
            }
        }
        progenitors
    }

    // all items for which the item is a head parent
    pub(crate) fn get_head_children(&self, item: ItemId) -> impl Iterator<Item = ItemId> + '_ {
        self.graph
            .edges_directed(item.into(), Direction::Incoming)
            .filter(|e| e.weight().head)
            .map(|e| *self.graph.index(e.source()))
    }

    // get all langs that have at least one item that is descended from item
    // through head parentage
    pub(crate) fn get_head_progeny_langs(
        &self,
        items: &[Item],
        item: ItemId,
    ) -> Option<HashSet<Lang>> {
        let mut progeny_langs = HashSet::default();
        let mut unexpanded = self.get_head_children(item).collect_vec();
        while let Some(descendant) = unexpanded.pop() {
            progeny_langs.insert(items[descendant as usize].lang);
            unexpanded.extend(self.get_head_children(descendant));
        }
        (!progeny_langs.is_empty()).then_some(progeny_langs)
    }

    pub(crate) fn get_all_head_progeny_langs(
        &self,
        items: &[Item],
    ) -> HashMap<ItemId, HashSet<Lang>> {
        let mut progeny_langs = HashMap::default();
        for item in items.iter().map(|item| item.id) {
            if let Some(prog) = self.get_head_progeny_langs(items, item) {
                progeny_langs.insert(item, prog);
            }
        }
        progeny_langs
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

    pub(crate) fn add_ety(
        &mut self,
        item: ItemId,
        mode: EtyMode,
        head: Option<u8>,
        ety_items: &[ItemId],
        confidences: &[f32],
    ) {
        // StableGraph allows adding multiple parallel edges from one node to
        // another. So we have to be careful to check for any already existing
        // ety links. If there are some, we keep them and don't add any new
        // ones, unless the least confidence for the new ety links is greater
        // than the greatest confidence for the old ety links. In that case, we
        // delete all the old ones and add the new ones in their stead.
        let mut old_edges = self.graph.edges(item.into()).peekable();
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
                let old_edge_ids = self.graph.edges(item.into()).map(|e| e.id()).collect_vec();
                for old_edge_id in old_edge_ids {
                    self.graph.remove_edge(old_edge_id);
                }
            } else {
                return;
            }
        }

        for (i, &ety_item, &confidence) in izip!(0u8.., ety_items, confidences) {
            let ety_link = EtyLink {
                mode,
                order: i,
                head: head.map_or(false, |head| head == i),
                confidence,
            };
            self.graph.add_edge(item.into(), ety_item.into(), ety_link);
        }
    }
}

#[derive(Default)]
pub(crate) struct EtyGraph {
    pub(crate) imputed_items: ImputedItems,
    pub(crate) graph: Graph,
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

    pub(crate) fn add(&mut self, item_id: ItemId) {
        self.graph.add(item_id);
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
        head: Option<u8>,
        ety_items: &[ItemId],
        confidences: &[f32],
    ) {
        self.graph.add_ety(item, mode, head, ety_items, confidences);
    }

    pub(crate) fn remove_cycles(&mut self) -> Result<()> {
        self.graph.remove_cycles()
    }

    pub(crate) fn get_immediate_ety(&self, item_id: ItemId) -> Option<ImmediateEty> {
        self.graph.get_immediate_ety(item_id)
    }
}
