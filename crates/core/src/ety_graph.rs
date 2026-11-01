use crate::{
    HashMap, HashSet,
    descendant_langs::DescendantLangs,
    etymology_templates::EtyMode,
    items::{Item, ItemId},
    languages::Lang,
};

use std::collections::VecDeque;

use itertools::{Itertools, izip};
use petgraph::{
    Direction,
    stable_graph::{EdgeReference, StableDiGraph},
    visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter};

pub type EtyEdge<'a> = EdgeReference<'a, EtyEdgeData>;

#[derive(Serialize, Deserialize)]
pub struct EtyEdgeData {
    pub mode: EtyMode,
    /// The parent's place among all of the child's parents, across its
    /// etymologies: unique per child.
    pub order: u8,
    pub head: bool,
    pub basis: EdgeBasis,
    /// Which of the child's etymologies the edge belongs to.
    pub ety: u8,
    pub certainty: Certainty,
    confidence: f32,
}

/// How firmly a source holds a link, in its own terms: no hedge is stated.
/// Lower is firmer.
#[derive(
    Serialize,
    Deserialize,
    Hash,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Debug,
    Copy,
    Clone,
    Default,
    AsRefStr,
    EnumIter,
)]
#[strum(serialize_all = "snake_case")]
pub enum Certainty {
    #[default]
    Stated,
    Probable,
    Possible,
    Doubtful,
}

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug, Copy, Clone, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum EtyKind {
    /// parents, by a mode
    Derivation,
    /// the source says no origin is known ({{unk}}): known to be unknown,
    /// unlike nothing parsed
    Unknown,
}

/// An item's etymologies, for an item with several (alternatives one source
/// gives, as "from A or B") or with an unknown origin. An item not listed has
/// at most one, a derivation, and it is followed.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Etymologies {
    pub kinds: Box<[EtyKind]>,
    /// The etymology walks follow. None: the source prefers none, and walks
    /// follow every alternative.
    pub primary: Option<u8>,
}

/// One etymology to add: its parents, each with the homograph confidence and
/// the certainty of its link.
pub struct NewEtymology<'a> {
    pub mode: EtyMode,
    pub head: Option<u8>,
    pub basis: EdgeBasis,
    pub items: &'a [ItemId],
    pub confidences: &'a [f32],
    pub certainties: &'a [Certainty],
}

/// What made an edge: a statement on Wiktionary, or a rule of wety's that
/// reads one. A rule's edge is wety's inference, not the source's claim.
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug, Copy, Clone, AsRefStr, EnumIter)]
#[strum(serialize_all = "snake_case")]
pub enum EdgeBasis {
    /// an {{etymon}} group, or the inline etymology it gives a single parent
    Etymon,
    /// the first classic etymology template ({{inh}}, {{bor}}, {{af}}, ...)
    Template,
    /// a later template read as the etymology of the parent before it, whose
    /// term is missing from the data ("from enm X, from ang Y")
    Chain,
    /// a lone {{m}}, in a section with no other etymology template
    Mention,
    /// a form-of or alt-of sense, for an entry with no etymology
    FormOf,
    /// a line of the parent's Descendants section
    Descendants,
    /// the entry's {{root}} or root category, for an entry with no other
    /// etymology
    Root,
    /// the root that most of a progenitor's derivatives name, for a progenitor
    /// with no etymology
    RootVote,
}

impl EdgeBasis {
    /// Whether the edge is wety's reading rather than a link Wiktionary gives.
    #[must_use]
    pub fn is_rule(self) -> bool {
        matches!(
            self,
            Self::Chain | Self::Mention | Self::FormOf | Self::Root | Self::RootVote
        )
    }
}

pub trait EtyEdgeAccess {
    fn child(&self) -> ItemId;
    fn parent(&self) -> ItemId;
    fn order(&self) -> u8;
    fn head(&self) -> bool;
    fn mode(&self) -> EtyMode;
    fn basis(&self) -> EdgeBasis;
    fn ety(&self) -> u8;
    fn certainty(&self) -> Certainty;
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
    fn basis(&self) -> EdgeBasis {
        self.weight().basis
    }
    fn ety(&self) -> u8 {
        self.weight().ety
    }
    fn certainty(&self) -> Certainty {
        self.weight().certainty
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
    pub etymologies: HashMap<ItemId, Etymologies>,
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

    pub fn edges(&self) -> impl Iterator<Item = EtyEdge<'_>> {
        self.graph.edge_references()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }

    /// The parents walks follow from an item, in order: its primary
    /// etymology's, or, when it has none, every alternative's. The mode is the
    /// first followed etymology's, and there is a head only when one etymology
    /// is followed.
    #[must_use]
    pub fn immediate_ety(&self, id: ItemId) -> Option<ImmediateEty> {
        let mut edges = self.parent_edges(id).collect_vec();
        edges.sort_unstable_by_key(|e| e.order());
        let first = edges.first()?;
        let mode = first.mode();
        let single = edges.iter().all(|e| e.ety() == first.ety());
        let head = edges
            .iter()
            .position(|e| e.head())
            .filter(|_| single)
            .and_then(|i| u8::try_from(i).ok());
        Some(ImmediateEty {
            items: edges.iter().map(|e| e.parent()).collect(),
            mode,
            head,
        })
    }

    /// Whether walks follow an edge: it belongs to its child's primary
    /// etymology, or the child has none.
    #[must_use]
    pub fn follows(&self, edge: EtyEdge<'_>) -> bool {
        self.etymologies
            .get(&edge.child())
            .and_then(|e| e.primary)
            .is_none_or(|primary| primary == edge.ety())
    }

    /// An item's etymologies, if it has several or an unknown origin.
    #[must_use]
    pub fn etymologies(&self, id: ItemId) -> Option<&Etymologies> {
        self.etymologies.get(&id)
    }

    /// Break every cycle at its weakest link: in each cycle, remove the ety of
    /// the member that `weakness` ranks greatest. Returns the number of etys
    /// removed.
    pub fn break_cycles<K: Ord>(&mut self, weakness: impl Fn(&Self, ItemId) -> K) -> usize {
        let mut removed = 0;
        for scc in petgraph::algo::tarjan_scc(&self.graph) {
            if scc.len() == 1 && !self.graph.contains_edge(scc[0], scc[0]) {
                continue;
            }
            let mut scc = scc;
            scc.sort_unstable(); // a deterministic search order
            let members: HashSet<ItemId> = scc.iter().copied().collect();
            while let Some(cycle) = self.find_cycle(&scc, &members) {
                let weakest = cycle
                    .into_iter()
                    .max_by_key(|&item| (weakness(self, item), item))
                    .expect("a cycle has members");
                self.remove_ety(weakest);
                removed += 1;
            }
        }
        removed
    }

    // A cycle among `members` (following edges from child to parent), as the
    // list of its members, if there is one.
    fn find_cycle(&self, order: &[ItemId], members: &HashSet<ItemId>) -> Option<Vec<ItemId>> {
        #[derive(Clone, Copy, PartialEq)]
        enum State {
            OnPath,
            Done,
        }
        let mut state: HashMap<ItemId, State> = HashMap::default();
        for &start in order {
            if state.contains_key(&start) {
                continue;
            }
            // (item, its parents within members, next parent to visit)
            let mut path: Vec<(ItemId, Vec<ItemId>, usize)> = vec![];
            let parents = |item: ItemId| -> Vec<ItemId> {
                self.parent_edges(item)
                    .map(|e| e.target())
                    .filter(|p| members.contains(p))
                    .collect()
            };
            state.insert(start, State::OnPath);
            path.push((start, parents(start), 0));
            while let Some((item, item_parents, next)) = path.last_mut() {
                if let Some(&parent) = item_parents.get(*next) {
                    *next += 1;
                    match state.get(&parent) {
                        Some(State::OnPath) => {
                            let from = path.iter().position(|(i, _, _)| *i == parent)?;
                            return Some(path[from..].iter().map(|(i, _, _)| *i).collect());
                        }
                        Some(State::Done) => {}
                        None => {
                            state.insert(parent, State::OnPath);
                            path.push((parent, parents(parent), 0));
                        }
                    }
                } else {
                    state.insert(*item, State::Done);
                    path.pop();
                }
            }
        }
        None
    }

    /// Remove an item's etys (its edges to its parents, of every etymology).
    pub fn remove_ety(&mut self, item: ItemId) {
        let edge_ids = self.graph.edges(item).map(|e| e.id()).collect_vec();
        for edge_id in edge_ids {
            self.graph.remove_edge(edge_id);
        }
        self.etymologies.remove(&item);
    }

    /// Replace an item's etymologies with `etys`, alternatives one source gives.
    /// The primary is the one whose weakest link is firmer than every other's;
    /// where two tie, the source prefers none and walks follow each.
    ///
    /// # Panics
    ///
    /// Panics if `etys` is empty or has more than 255 parents in all.
    pub fn add_etymologies(&mut self, item: ItemId, etys: &[NewEtymology<'_>]) {
        assert!(!etys.is_empty(), "at least one etymology");
        self.remove_ety(item);
        let mut order = 0u8;
        for (ety, new) in izip!(0u8.., etys) {
            for (i, &parent, &confidence, &certainty) in
                izip!(0u8.., new.items, new.confidences, new.certainties)
            {
                let data = EtyEdgeData {
                    mode: new.mode,
                    order,
                    head: new.head == Some(i),
                    basis: new.basis,
                    ety,
                    certainty,
                    confidence,
                };
                self.graph.add_edge(item, parent, data);
                order = order.checked_add(1).expect("at most 255 parents");
            }
        }
        if etys.len() > 1 {
            let weakest = |new: &NewEtymology<'_>| new.certainties.iter().max().copied();
            let firmest = etys.iter().map(weakest).min().flatten();
            let firmest_etys = etys.iter().filter(|e| weakest(e) == firmest).count();
            let primary = etys
                .iter()
                .position(|e| weakest(e) == firmest)
                .filter(|_| firmest_etys == 1)
                .and_then(|i| u8::try_from(i).ok());
            self.etymologies.insert(
                item,
                Etymologies {
                    kinds: vec![EtyKind::Derivation; etys.len()].into_boxed_slice(),
                    primary,
                },
            );
        }
    }

    /// Record that an item's origin is unknown, if nothing gives it parents.
    pub fn set_unknown(&mut self, item: ItemId) {
        if self.graph.edges(item).next().is_none() {
            self.etymologies.insert(
                item,
                Etymologies {
                    kinds: Box::new([EtyKind::Unknown]),
                    primary: Some(0),
                },
            );
        }
    }

    /// # Panics
    ///
    /// Panics if `confidences` is empty.
    pub fn add_ety(
        &mut self,
        item: ItemId,
        mode: EtyMode,
        head: Option<u8>,
        basis: EdgeBasis,
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
        self.etymologies.remove(&item);

        for (i, &ety_item, &confidence) in izip!(0u8.., ety_items, confidences) {
            let ety_link = EtyEdgeData {
                mode,
                order: i,
                head: head.is_some_and(|head| head == i),
                basis,
                ety: 0,
                certainty: Certainty::Stated,
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
    /// The edges walks follow from `item` to its children.
    pub fn child_edges(&self, item: ItemId) -> impl Iterator<Item = EtyEdge<'_>> + '_ {
        self.graph
            .edges_directed(item, Direction::Incoming)
            .filter(|&e| self.follows(e))
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
    pub fn all_descendant_langs(&self) -> DescendantLangs {
        DescendantLangs::new(
            self.iter()
                .map(|(item_id, _)| {
                    (
                        item_id,
                        self.descendant_langs(item_id).into_iter().collect(),
                    )
                })
                .collect(),
        )
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
    /// The edges walks follow from `item` to its parents.
    pub fn parent_edges(&self, item: ItemId) -> impl Iterator<Item = EtyEdge<'_>> + '_ {
        self.graph
            .edges_directed(item, Direction::Outgoing)
            .filter(|&e| self.follows(e))
    }

    /// Every edge from `item` to its parents, of every etymology.
    pub fn all_parent_edges(&self, item: ItemId) -> impl Iterator<Item = EtyEdge<'_>> + '_ {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{items::ImputedItem, langterm::Term, languages::Lang, string_pool::StringPool};
    use std::str::FromStr;

    fn items(graph: &mut EtyGraph, terms: &[&str]) -> Vec<ItemId> {
        let mut sp = StringPool::new();
        let lang = Lang::from_str("en").unwrap();
        terms
            .iter()
            .map(|t| {
                graph.add(Item::Imputed(ImputedItem {
                    ety_num: 1,
                    lang,
                    term: Term::new(&mut sp, t),
                    romanization: None,
                    from: ItemId::from(0),
                }))
            })
            .collect()
    }

    fn new_ety<'a>(items: &'a [ItemId], certainties: &'a [Certainty]) -> NewEtymology<'a> {
        NewEtymology {
            mode: EtyMode::Borrowed,
            head: Some(0),
            basis: EdgeBasis::Etymon,
            items,
            confidences: &[1.0, 1.0][..items.len()],
            certainties,
        }
    }

    #[test]
    fn walks_follow_the_primary_or_every_alternative() {
        let mut graph = EtyGraph::default();
        let [w, a, b, v] = items(&mut graph, &["w", "a", "b", "v"])[..] else {
            unreachable!()
        };
        let (stated, possible) = ([Certainty::Stated], [Certainty::Possible]);
        // "From A, or possibly from B": A is primary
        graph.add_etymologies(w, &[new_ety(&[a], &stated), new_ety(&[b], &possible)]);
        assert_eq!(graph.etymologies(w).unwrap().primary, Some(0));
        assert_eq!(graph.immediate_ety(w).unwrap().items, [a]);
        assert_eq!(graph.child_edges(b).count(), 0);
        assert_eq!(graph.all_parent_edges(w).count(), 2);
        // "From A or B": no primary, both followed, no head
        graph.add_etymologies(v, &[new_ety(&[a], &stated), new_ety(&[b], &stated)]);
        assert_eq!(graph.etymologies(v).unwrap().primary, None);
        let ety = graph.immediate_ety(v).unwrap();
        assert_eq!((ety.items, ety.head), (vec![a, b], None));
        assert_eq!(graph.child_edges(b).map(|e| e.child()).collect_vec(), [v]);
        // one etymology needs no table, and replaces the alternatives
        graph.add_etymologies(v, &[new_ety(&[b], &possible)]);
        assert!(graph.etymologies(v).is_none());
        let ety = graph.immediate_ety(v).unwrap();
        assert_eq!((ety.items, ety.head), (vec![b], Some(0)));
        assert_eq!(
            graph.parent_edges(v).next().unwrap().certainty(),
            Certainty::Possible
        );
    }

    #[test]
    fn an_unknown_origin_is_kept_only_without_parents() {
        let mut graph = EtyGraph::default();
        let [w, a, u] = items(&mut graph, &["w", "a", "u"])[..] else {
            unreachable!()
        };
        graph.set_unknown(u);
        assert_eq!(graph.etymologies(u).unwrap().kinds[..], [EtyKind::Unknown]);
        assert!(graph.immediate_ety(u).is_none());
        graph.add_ety(
            w,
            EtyMode::Inherited,
            Some(0),
            EdgeBasis::Descendants,
            &[a],
            &[1.0],
        );
        graph.set_unknown(w);
        assert!(graph.etymologies(w).is_none());
        // a later etymology replaces it
        graph.add_ety(u, EtyMode::Root, Some(0), EdgeBasis::Root, &[a], &[0.5]);
        assert!(graph.etymologies(u).is_none());
    }

    #[test]
    fn cycles_break_at_their_weakest_link() {
        let mut sp = StringPool::new();
        let mut graph = EtyGraph::default();
        let lang = Lang::from_str("en").unwrap();
        let mut add = |graph: &mut EtyGraph, term: &str| {
            graph.add(Item::Imputed(ImputedItem {
                ety_num: 1,
                lang,
                term: Term::new(&mut sp, term),
                romanization: None,
                from: ItemId::from(0),
            }))
        };
        let [a, b, c, d] = ["a", "b", "c", "d"].map(|t| add(&mut graph, t));
        // a -> b -> c -> a, and c -> d outside the cycle
        graph.add_ety(
            a,
            EtyMode::Inherited,
            Some(0),
            EdgeBasis::Template,
            &[b],
            &[0.9],
        );
        graph.add_ety(
            b,
            EtyMode::Inherited,
            Some(0),
            EdgeBasis::Template,
            &[c],
            &[0.2],
        );
        graph.add_ety(
            c,
            EtyMode::Compound,
            None,
            EdgeBasis::Template,
            &[a, d],
            &[0.9, 0.9],
        );
        // lower confidence is weaker
        let removed = graph.break_cycles(|g, item| {
            let confidence = g
                .graph
                .edges(item)
                .map(|e| e.confidence())
                .fold(1.0, f32::min);
            std::cmp::Reverse((confidence * 100.0) as u32)
        });
        assert_eq!(removed, 1);
        assert!(graph.immediate_ety(b).is_none());
        assert!(graph.immediate_ety(a).is_some() && graph.immediate_ety(c).is_some());
    }
}
