use crate::{
    HashMap, HashSet,
    etymology_templates::EtyMode,
    items::{Item, ItemId},
    languages::Lang,
};

use std::collections::VecDeque;

use anyhow::{Ok, Result};
use itertools::{Itertools, izip};
use petgraph::{
    Direction,
    algo::greedy_feedback_arc_set,
    stable_graph::{EdgeIndex, EdgeReference, StableDiGraph},
    visit::{EdgeRef, IntoNodeReferences},
};
use serde::{Deserialize, Serialize};

pub type EtyEdge<'a> = EdgeReference<'a, EtyEdgeData>;

#[derive(Serialize, Deserialize)]
pub struct EtyEdgeData {
    pub mode: EtyMode,
    pub order: u8,
    pub head: bool,
    confidence: f32,
}

pub trait EtyEdgeAccess {
    fn child(&self) -> ItemId;
    fn parent(&self) -> ItemId;
    fn order(&self) -> u8;
    fn head(&self) -> bool;
    fn mode(&self) -> EtyMode;
    fn confidence(&self) -> f32;
}

impl EtyEdgeAccess for EtyEdge<'_> {
    fn child(&self) -> ItemId {
        self.source()
    }
    fn parent(&self) -> ItemId {
        self.target()
    }
    fn order(&self) -> u8 {
        self.weight().order
    }
    fn head(&self) -> bool {
        self.weight().head
    }
    fn mode(&self) -> EtyMode {
        self.weight().mode
    }
    fn confidence(&self) -> f32 {
        self.weight().confidence
    }
}

// the parents of some item
pub struct ImmediateEty {
    pub items: Vec<ItemId>,
    pub head: Option<u8>,
    pub mode: EtyMode,
}

impl ImmediateEty {
    fn head(&self) -> Option<ItemId> {
        self.head.map(|head| self.items[head as usize])
    }
}

pub type ItemIndex = u32;

#[derive(Default, Serialize, Deserialize)]
pub struct EtyGraph {
    pub graph: StableDiGraph<Item, EtyEdgeData, ItemIndex>,
}

impl EtyGraph {
    pub fn add(&mut self, item: Item) -> ItemId {
        self.graph.add_node(item)
    }

    /// get previously added item
    #[must_use]
    pub fn item(&self, id: ItemId) -> &Item {
        &self.graph[id]
    }

    /// get previously added item mutably
    pub fn item_mut(&mut self, id: ItemId) -> &mut Item {
        &mut self.graph[id]
    }

    pub fn iter(&self) -> impl Iterator<Item = (ItemId, &Item)> {
        self.graph.node_references()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }

    #[must_use]
    pub fn immediate_ety(&self, id: ItemId) -> Option<ImmediateEty> {
        let mut parents = vec![];
        let mut order = vec![];
        // Next two lines are dummy assignments. If there are any parents in the
        // ety_graph, they will get overwritten with correct values. If no
        // parents, they will not get returned.
        let mut head = None;
        let mut mode = EtyMode::Derived;
        for ety_edge in self.graph.edges(id) {
            parents.push(ety_edge.parent());
            order.push(ety_edge.order());
            mode = ety_edge.mode();
            if ety_edge.head() {
                head = Some(ety_edge.order());
            }
        }
        parents = order.iter().map(|&ord| parents[ord as usize]).collect();
        (!parents.is_empty()).then_some(ImmediateEty {
            items: parents,
            mode,
            head,
        })
    }

    /// # Errors
    ///
    /// Returns an error if cycle-detection internals fail unexpectedly.
    pub fn remove_cycles(&mut self) -> Result<()> {
        print!("  Checking for ety link feedback arc set... ");
        let fas: Vec<EdgeIndex> = greedy_feedback_arc_set(&self.graph)
            .map(|e| e.id())
            .collect();
        if fas.is_empty() {
            println!("Found none.");
        } else {
            println!("Found set of size {}. Removing.", fas.len());
            for edge in fas {
                if let Some((source, _)) = self.graph.edge_endpoints(edge) {
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
        }
        Ok(())
    }

    /// # Panics
    ///
    /// Panics if `confidences` is empty.
    pub fn add_ety(
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
        let min_new_confidence = confidences
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .expect("at least one");
        let mut old_edges = self.graph.edges(item).peekable();
        if old_edges.peek().is_some() {
            let max_old_confidence = old_edges
                .map(|e| e.confidence())
                .max_by(|a, b| a.total_cmp(b))
                .expect("at least one");
            if min_new_confidence <= &max_old_confidence {
                return;
            }
            let old_edge_ids = self.graph.edges(item).map(|e| e.id()).collect_vec();
            for old_edge_id in old_edge_ids {
                self.graph.remove_edge(old_edge_id);
            }
        }

        for (i, &ety_item, &confidence) in izip!(0u8.., ety_items, confidences) {
            let ety_link = EtyEdgeData {
                mode,
                order: i,
                head: head.is_some_and(|head| head == i),
                confidence,
            };
            self.graph.add_edge(item, ety_item, ety_link);
        }
    }
}

/// all of the ultimate ancestors of some item, i.e. all of the leaf nodes on
/// the ancestry tree rooted by the item
#[derive(Serialize, Deserialize)]
pub struct Progenitors {
    pub items: Box<[ItemId]>,
    pub head: Option<ItemId>,
}

impl Progenitors {
    fn new(mut progenitors: HashSet<ItemId>, head: Option<ItemId>) -> Self {
        Self {
            items: progenitors.drain().collect_vec().into_boxed_slice(),
            head,
        }
    }
}

struct Tracker {
    unexpanded: Vec<ItemId>,
    progenitors: HashSet<ItemId>,
    head: Option<ItemId>,
    expanded: HashSet<ItemId>,
    cycle_found: bool,
}

impl EtyGraph {
    #[must_use]
    pub fn progenitors(&self, item: ItemId) -> Option<Progenitors> {
        let immediate_ety = self.immediate_ety(item)?;
        let head = immediate_ety.head();
        let mut t = Tracker {
            unexpanded: immediate_ety.items,
            progenitors: HashSet::default(),
            head,
            expanded: HashSet::default(),
            cycle_found: false,
        };
        self.progenitors_recurse(&mut t);
        if t.cycle_found {
            return None;
        }
        let head = t.head;
        Some(Progenitors::new(t.progenitors, head))
    }

    fn progenitors_recurse(&self, t: &mut Tracker) {
        while !t.cycle_found
            && let Some(item) = t.unexpanded.pop()
        {
            if !t.expanded.insert(item) {
                t.cycle_found = true;
                return;
            }
            if let Some(immediate_ety) = self.immediate_ety(item) {
                let ety_head = immediate_ety.head();
                for &ety_item in &immediate_ety.items {
                    if t.head.is_some_and(|h| h == item)
                        && ety_head.is_some_and(|eh| eh == ety_item)
                    {
                        t.head = ety_head;
                    }
                    t.unexpanded.push(ety_item);
                }
                self.progenitors_recurse(t);
            } else {
                t.progenitors.insert(item);
            }
        }
    }

    #[must_use]
    pub fn all_progenitors(&self) -> HashMap<ItemId, Progenitors> {
        let mut progenitors = HashMap::default();
        for (item_id, _) in self.iter() {
            if let Some(prog) = self.progenitors(item_id) {
                progenitors.insert(item_id, prog);
            }
        }
        progenitors
    }
}

/// Breadth-first iterator over the edges connecting `item` and its descendants.
struct DescendantEdgeIterator<'a> {
    graph: &'a EtyGraph,
    queue: VecDeque<EtyEdge<'a>>,
}

impl<'a> Iterator for DescendantEdgeIterator<'a> {
    type Item = EtyEdge<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(descendant_edge) = self.queue.pop_front() {
            self.queue
                .extend(self.graph.child_edges(descendant_edge.child()));
            return Some(descendant_edge);
        }
        None
    }
}

impl EtyGraph {
    /// All of the edges connecting `item` to its children.
    pub fn child_edges(&self, item: ItemId) -> impl Iterator<Item = EtyEdge<'_>> + '_ {
        self.graph.edges_directed(item, Direction::Incoming)
    }

    /// Iterate breadth-first over the edges connecting `item` and its descendants.
    pub fn descendant_edges(&self, item: ItemId) -> impl Iterator<Item = EtyEdge<'_>> + '_ {
        DescendantEdgeIterator {
            graph: self,
            queue: VecDeque::from(self.child_edges(item).collect_vec()),
        }
    }

    /// Get all langs that have at least one item that is descended from `item`.
    #[must_use]
    pub fn descendant_langs(&self, item: ItemId) -> HashSet<Lang> {
        let mut descendant_langs = HashSet::default();
        for descendant_edge in self.descendant_edges(item) {
            descendant_langs.insert(self.item(descendant_edge.child()).lang());
        }
        descendant_langs
    }

    /// For each item, get all langs that have at least one item that is
    /// descended from that item.
    #[must_use]
    pub fn all_descendant_langs(&self) -> HashMap<ItemId, HashSet<Lang>> {
        let mut descendant_langs = HashMap::default();
        for (item_id, _) in self.iter() {
            descendant_langs.insert(item_id, self.descendant_langs(item_id));
        }
        descendant_langs
    }
}

/// Breadth-first iterator over the edges connecting `item` and its ancestors.
struct AncestorEdgeIterator<'a> {
    graph: &'a EtyGraph,
    queue: VecDeque<EtyEdge<'a>>,
}

impl<'a> Iterator for AncestorEdgeIterator<'a> {
    type Item = EtyEdge<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ancestor_edge) = self.queue.pop_front() {
            self.queue
                .extend(self.graph.parent_edges(ancestor_edge.parent()));
            return Some(ancestor_edge);
        }
        None
    }
}

impl EtyGraph {
    /// All of the edges connecting `item` to its parents.
    pub fn parent_edges(&self, item: ItemId) -> impl Iterator<Item = EtyEdge<'_>> + '_ {
        self.graph.edges_directed(item, Direction::Outgoing)
    }

    /// Iterate breadth-first over the edges connecting `item` and its ancestors.
    pub fn ancestor_edges(&self, item: ItemId) -> impl Iterator<Item = EtyEdge<'_>> + '_ {
        AncestorEdgeIterator {
            graph: self,
            queue: VecDeque::from(self.parent_edges(item).collect_vec()),
        }
    }

    /// Get all ancestors of `item` within `langs`.
    pub fn ancestors_in_langs<'a>(
        &'a self,
        item: ItemId,
        langs: &'a [Lang],
    ) -> impl Iterator<Item = ItemId> + 'a {
        self.ancestor_edges(item)
            .filter(|e| langs.contains(&self.item(e.parent()).lang()))
            .map(|e| e.parent())
    }
}
