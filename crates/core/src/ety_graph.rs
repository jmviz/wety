use crate::{
    HashMap, HashSet,
    assertions::{self, Agent, Asserted, Claim, Work},
    descendant_langs::DescendantLangs,
    etymology_templates::EtyMode,
    items::{Item, ItemId},
    languages::{Family, Lang},
    synthesis::{Reading, primary},
};

use std::collections::VecDeque;

use itertools::{Itertools, izip};
use petgraph::{
    Direction,
    stable_graph::{EdgeReference, StableDiGraph},
    visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter, IntoStaticStr};

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
    IntoStaticStr,
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

#[derive(
    Serialize, Deserialize, Hash, Eq, PartialEq, Debug, Copy, Clone, AsRefStr, IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum EtyKind {
    /// parents, by a mode
    Derivation,
    /// the source says no origin is known ({{unk}}): known to be unknown,
    /// unlike nothing parsed
    Unknown,
    /// the source names the language or family the word came from, but no
    /// word of it ("From Welsh", "Borrowed from some Turkic language"): no
    /// parents, the exact source uncertain
    LanguageOnly(LanguageOnly),
}

impl EtyKind {
    /// The certainty of an etymology with no parents to carry one.
    #[must_use]
    pub fn certainty(self) -> Option<Certainty> {
        match self {
            Self::LanguageOnly(l) => Some(l.certainty),
            Self::Derivation | Self::Unknown => None,
        }
    }
}

/// An origin in a language, or a family of languages, with no word named.
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct LanguageOnly {
    pub source: SourceLang,
    pub mode: EtyMode,
    pub certainty: Certainty,
}

/// A language or a family of languages, named as a word's source.
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub enum SourceLang {
    Lang(Lang),
    Family(Family),
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
    /// How the primary was chosen. The default, one lineage, is that of every
    /// item not listed.
    pub reading: Reading,
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
    /// a claim of another source, its records matched to items: an etymology
    /// Wiktionary does not give (see `EtyGraph::claims`)
    Source,
}

impl EdgeBasis {
    /// Whether the edge is wety's reading rather than a link Wiktionary gives.
    #[must_use]
    pub fn is_rule(self) -> bool {
        assertions::rule(self).is_some()
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
    /// Each etymology's assertion, by etymology number, for items where one
    /// is not the default (stated on the item's own page, uncredited).
    pub assertions: HashMap<ItemId, Box<[Asserted]>>,
    /// The works that assertions credit.
    pub works: Vec<Work>,
    /// Other sources' claims about items' etymologies, for items they matched.
    pub claims: HashMap<ItemId, Box<[Claim]>>,
    /// The asserters and origins that claims and relays name.
    pub agents: Vec<Agent>,
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
    /// the member that `weakness` ranks greatest, or only the alternative that
    /// leads into the cycle when it has others. Returns the number of etys
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
                // each member's parent in the cycle is the next member
                let (i, &weakest) = cycle
                    .iter()
                    .enumerate()
                    .max_by_key(|&(_, &item)| (weakness(self, item), item))
                    .expect("a cycle has members");
                self.remove_ety_through(weakest, cycle[(i + 1) % cycle.len()]);
                removed += 1;
            }
        }
        removed
    }

    // A cycle among `members` (following edges from child to parent, of every
    // etymology, not only those walks follow: walks over every alternative, as the
    // etymology chart's, must end too), as the list of its members, if there is one.
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
                self.all_parent_edges(item)
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

    /// Remove the etymology by which an item descends from `parent`, keeping
    /// its alternatives, if it has any; the primary among those left is chosen
    /// as add_etymologies chooses it.
    fn remove_ety_through(&mut self, item: ItemId, parent: ItemId) {
        let n = self.etymologies.get(&item).map_or(1, |e| e.kinds.len());
        let Some(ety) = self
            .all_parent_edges(item)
            .find(|e| e.parent() == parent)
            .map(|e| e.ety())
            .filter(|_| n > 1)
        else {
            self.remove_ety(item);
            return;
        };
        let mut kinds = self
            .etymologies
            .get(&item)
            .map_or_else(|| vec![EtyKind::Derivation; n], |e| e.kinds.to_vec());
        kinds.remove(usize::from(ety));
        let edge_ids = self.graph.edges(item).map(|e| e.id()).collect_vec();
        let mut weakest: Vec<Certainty> = kinds
            .iter()
            .map(|k| k.certainty().unwrap_or_default())
            .collect();
        for edge_id in edge_ids {
            let edge = &mut self.graph[edge_id];
            if edge.ety == ety {
                self.graph.remove_edge(edge_id);
                continue;
            }
            if edge.ety > ety {
                edge.ety -= 1;
            }
            let w = &mut weakest[edge.ety as usize];
            *w = (*w).max(edge.certainty);
        }
        if let Some(asserted) = self.assertions.remove(&item) {
            let mut asserted = asserted.into_vec();
            if usize::from(ety) < asserted.len() {
                asserted.remove(usize::from(ety));
            }
            self.set_asserted(item, asserted);
        }
        if let Some(claims) = self.claims.remove(&item) {
            let claims: Box<[Claim]> = claims
                .into_vec()
                .into_iter()
                .filter(|c| c.ety != ety)
                .map(|c| Claim {
                    ety: if c.ety > ety { c.ety - 1 } else { c.ety },
                    ..c
                })
                .collect();
            if !claims.is_empty() {
                self.claims.insert(item, claims);
            }
        }
        if kinds == [EtyKind::Derivation] {
            self.etymologies.remove(&item);
        } else {
            self.etymologies.insert(
                item,
                Etymologies {
                    kinds: kinds.into_boxed_slice(),
                    primary: primary(&weakest),
                    reading: Reading::default(),
                },
            );
        }
    }

    /// Remove an item's etys (its edges to its parents, of every etymology).
    pub fn remove_ety(&mut self, item: ItemId) {
        let edge_ids = self.graph.edges(item).map(|e| e.id()).collect_vec();
        for edge_id in edge_ids {
            self.graph.remove_edge(edge_id);
        }
        self.etymologies.remove(&item);
        self.assertions.remove(&item);
        self.claims.remove(&item);
    }

    /// Record what each of an item's etymologies' assertions adds to its edges,
    /// by etymology number (see `assertions`). Written after the etymologies;
    /// replacing or removing them forgets it.
    pub fn set_asserted(&mut self, item: ItemId, asserted: Vec<Asserted>) {
        if asserted.iter().all(Asserted::is_default) {
            self.assertions.remove(&item);
        } else {
            self.assertions.insert(item, asserted.into_boxed_slice());
        }
    }

    /// What an etymology's assertion adds to its edges, if anything.
    #[must_use]
    pub fn asserted(&self, item: ItemId, ety: u8) -> Option<&Asserted> {
        self.assertions
            .get(&item)
            .and_then(|a| a.get(usize::from(ety)))
            .filter(|a| !a.is_default())
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
            let weakest: Vec<Certainty> = etys
                .iter()
                .map(|new| new.certainties.iter().max().copied().unwrap_or_default())
                .collect();
            self.etymologies.insert(
                item,
                Etymologies {
                    kinds: vec![EtyKind::Derivation; etys.len()].into_boxed_slice(),
                    primary: primary(&weakest),
                    reading: Reading::default(),
                },
            );
        }
    }

    /// The number of etymologies an item has (an unknown origin is one).
    #[must_use]
    pub fn ety_count(&self, item: ItemId) -> usize {
        match self.etymologies.get(&item) {
            Some(e) => e.kinds.len(),
            None => usize::from(self.graph.edges(item).next().is_some()),
        }
    }

    /// Add one etymology to an item's, as another alternative (or its first),
    /// keeping theirs and their assertions; returns its number. Its reading is
    /// left to synthesis: the primary stays what it was, and an item that had
    /// one etymology keeps following it.
    ///
    /// # Panics
    ///
    /// Panics past 255 parents or etymologies.
    pub fn push_etymology(&mut self, item: ItemId, new: &NewEtymology<'_>) -> u8 {
        let n = self.ety_count(item);
        let ety = u8::try_from(n).expect("at most 255 etymologies");
        let mut order = self
            .graph
            .edges(item)
            .map(|e| e.weight().order + 1)
            .max()
            .unwrap_or(0);
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
        if n > 0 {
            let old = self.etymologies.remove(&item);
            let mut kinds = old
                .as_ref()
                .map_or_else(|| vec![EtyKind::Derivation; n], |e| e.kinds.to_vec());
            kinds.push(EtyKind::Derivation);
            self.etymologies.insert(
                item,
                Etymologies {
                    kinds: kinds.into_boxed_slice(),
                    primary: old.map_or(Some(0), |e| e.primary),
                    reading: Reading::default(),
                },
            );
        }
        ety
    }

    /// Record that an item's origin is unknown, if nothing gives it parents.
    /// Returns whether it was recorded.
    pub fn set_unknown(&mut self, item: ItemId) -> bool {
        let unknown = self.graph.edges(item).next().is_none();
        if unknown {
            self.etymologies.insert(
                item,
                Etymologies {
                    kinds: Box::new([EtyKind::Unknown]),
                    primary: Some(0),
                    reading: Reading::default(),
                },
            );
        }
        unknown
    }

    /// Give an item etymologies naming only a language or family, if nothing
    /// gives it parents or another origin. Returns whether they were given.
    pub fn set_language_only(&mut self, item: ItemId, origins: &[LanguageOnly]) -> bool {
        let free = !origins.is_empty()
            && self.graph.edges(item).next().is_none()
            && !self.etymologies.contains_key(&item);
        if free {
            let weakest = origins.iter().map(|o| o.certainty).collect_vec();
            self.etymologies.insert(
                item,
                Etymologies {
                    kinds: origins.iter().map(|&o| EtyKind::LanguageOnly(o)).collect(),
                    primary: primary(&weakest),
                    reading: Reading::default(),
                },
            );
        }
        free
    }

    /// Add etymologies naming only a language or family after an item's
    /// others, as alternatives to them ("from Latin X, or from a Celtic
    /// language"); the primary is chosen anew, as add_etymologies chooses it.
    pub fn push_language_only(&mut self, item: ItemId, origins: &[LanguageOnly]) {
        if origins.is_empty() {
            return;
        }
        let n = self.ety_count(item);
        let mut kinds = self
            .etymologies
            .remove(&item)
            .map_or_else(|| vec![EtyKind::Derivation; n], |e| e.kinds.into_vec());
        let mut weakest: Vec<Certainty> = kinds
            .iter()
            .map(|k| k.certainty().unwrap_or_default())
            .collect();
        for e in self.graph.edges(item) {
            let w = &mut weakest[usize::from(e.weight().ety)];
            *w = (*w).max(e.weight().certainty);
        }
        for &o in origins {
            kinds.push(EtyKind::LanguageOnly(o));
            weakest.push(o.certainty);
        }
        self.etymologies.insert(
            item,
            Etymologies {
                kinds: kinds.into_boxed_slice(),
                primary: primary(&weakest),
                reading: Reading::default(),
            },
        );
    }

    /// Give an item one etymology, unless it has one whose links are at least
    /// as confident. Returns whether it was given.
    ///
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
    ) -> bool {
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
                return false;
            }
            let old_edge_ids = self.graph.edges(item).map(|e| e.id()).collect_vec();
            for old_edge_id in old_edge_ids {
                self.graph.remove_edge(old_edge_id);
            }
        }
        self.etymologies.remove(&item);
        self.assertions.remove(&item);
        self.claims.remove(&item);

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
        true
    }
}

/// all of the ultimate ancestors of some item, i.e. all of the leaf nodes on
/// the ancestry tree rooted by the item
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

impl EtyGraph {
    /// The items an item's etymology ends at, along every followed line, and
    /// the end of its head line. None if the lines run into a cycle; two lines
    /// may meet (alternatives from one source word) without one.
    #[must_use]
    pub fn progenitors(&self, item: ItemId) -> Option<Progenitors> {
        let immediate_ety = self.immediate_ety(item)?;
        let mut head = immediate_ety.head();
        let mut progenitors = HashSet::default();
        let mut done: HashSet<ItemId> = HashSet::default();
        let mut on_path: HashSet<ItemId> = HashSet::from_iter([item]);
        let mut path = vec![(item, immediate_ety.items.into_iter())];
        while let Some((current, parents)) = path.last_mut() {
            let Some(parent) = parents.next() else {
                let current = *current;
                on_path.remove(&current);
                done.insert(current);
                path.pop();
                continue;
            };
            if on_path.contains(&parent) {
                return None;
            }
            if done.contains(&parent) {
                continue;
            }
            if let Some(ety) = self.immediate_ety(parent) {
                on_path.insert(parent);
                path.push((parent, ety.items.into_iter()));
            } else {
                progenitors.insert(parent);
                done.insert(parent);
            }
        }
        // the head line, up to an item without a head (a compound) or a
        // progenitor
        while let Some(next) = head.and_then(|h| self.immediate_ety(h)?.head()) {
            head = Some(next);
        }
        Some(Progenitors::new(progenitors, head))
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

    /// The edges from `item` to children that name it in an etymology another
    /// of theirs is preferred to: words it is possibly related to, which walks
    /// do not follow.
    pub fn unfollowed_child_edges(&self, item: ItemId) -> impl Iterator<Item = EtyEdge<'_>> + '_ {
        self.graph
            .edges_directed(item, Direction::Incoming)
            .filter(|&e| !self.follows(e))
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
        // each descendant once, however many lines lead to it
        let mut descendant_langs = HashSet::default();
        let mut seen = HashSet::default();
        let mut stack = vec![item];
        while let Some(current) = stack.pop() {
            for edge in self.child_edges(current) {
                let child = edge.child();
                if seen.insert(child) {
                    descendant_langs.insert(self.item(child).lang());
                    stack.push(child);
                }
            }
        }
        descendant_langs
    }

    /// For each item, get all langs that have at least one item that is
    /// descended from that item.
    #[must_use]
    pub fn all_descendant_langs(&self) -> DescendantLangs {
        let items: Vec<ItemId> = self.graph.node_indices().collect();
        DescendantLangs::new(
            items
                .into_par_iter()
                .map(|item| (item, self.descendant_langs(item).into_iter().collect()))
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
    fn alternatives_that_meet_are_no_cycle_and_a_cycle_loses_one() {
        let mut graph = EtyGraph::default();
        let [w, a, b, c, d] = items(&mut graph, &["w", "a", "b", "c", "d"])[..] else {
            unreachable!()
        };
        let stated = [Certainty::Stated];
        // "From A or B", both from C: C is reached twice, and is the one
        // progenitor
        graph.add_etymologies(w, &[new_ety(&[a], &stated), new_ety(&[b], &stated)]);
        graph.add_etymologies(a, &[new_ety(&[c], &stated)]);
        graph.add_etymologies(b, &[new_ety(&[c], &stated)]);
        let progenitors = graph.progenitors(w).unwrap();
        assert_eq!((&progenitors.items[..], progenitors.head), (&[c][..], None));
        // A from W closes a cycle through W's first alternative; breaking it at
        // W removes that alternative only
        graph.add_etymologies(a, &[new_ety(&[w], &stated)]);
        assert!(graph.progenitors(w).is_none());
        graph.break_cycles(|_, item| u8::from(item == w));
        assert!(graph.etymologies(w).is_none());
        let ety = graph.immediate_ety(w).unwrap();
        assert_eq!((ety.items, ety.head), (vec![b], Some(0)));
        // of three, the two left keep no primary between them
        let [x, y, z] = items(&mut graph, &["x", "y", "z"])[..] else {
            unreachable!()
        };
        graph.add_etymologies(
            d,
            &[
                new_ety(&[x], &stated),
                new_ety(&[y], &stated),
                new_ety(&[z], &stated),
            ],
        );
        graph.add_etymologies(x, &[new_ety(&[d], &stated)]);
        // each alternative's assertion, told apart by its page
        let on = |page| Asserted {
            page: Some(page),
            ..Asserted::default()
        };
        graph.set_asserted(d, vec![on(x), on(y), on(z)]);
        graph.break_cycles(|_, item| u8::from(item == d));
        assert_eq!(graph.asserted(d, 0), Some(&on(y)));
        assert_eq!(graph.asserted(d, 1), Some(&on(z)));
        assert_eq!(graph.etymologies(d).unwrap().primary, None);
        assert_eq!(graph.immediate_ety(d).unwrap().items, [y, z]);
        let etys = graph.all_parent_edges(d).map(|e| e.ety()).sorted();
        assert_eq!(etys.collect_vec(), [0, 1]);
    }

    #[test]
    fn a_cycle_through_an_alternative_walks_do_not_follow_is_broken() {
        let mut graph = EtyGraph::default();
        let [w, a, b] = items(&mut graph, &["w", "a", "b"])[..] else {
            unreachable!()
        };
        // W from A, or possibly from B (the alternative walks do not follow),
        // and B from W: lītera and littera
        graph.add_etymologies(
            w,
            &[
                new_ety(&[a], &[Certainty::Stated]),
                new_ety(&[b], &[Certainty::Possible]),
            ],
        );
        graph.add_etymologies(b, &[new_ety(&[w], &[Certainty::Stated])]);
        assert_eq!(graph.break_cycles(|_, item| u8::from(item == w)), 1);
        assert!(graph.etymologies(w).is_none());
        assert_eq!(graph.immediate_ety(w).unwrap().items, [a]);
        assert_eq!(graph.immediate_ety(b).unwrap().items, [w]);
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
    fn origins_naming_only_a_language_stay_alternatives() {
        let mut graph = EtyGraph::default();
        let [w, a, b, l] = items(&mut graph, &["w", "a", "b", "l"])[..] else {
            unreachable!()
        };
        let celtic = LanguageOnly {
            source: SourceLang::Family(Family::from_str("cel").unwrap()),
            mode: EtyMode::Borrowed,
            certainty: Certainty::Possible,
        };
        // alone, as for an unknown origin
        assert!(graph.set_language_only(l, &[celtic]));
        assert_eq!(graph.ety_count(l), 1);
        assert!(graph.immediate_ety(l).is_none());
        assert!(!graph.set_language_only(l, &[celtic]));
        // after words: an alternative, the firmer word preferred
        let stated = [Certainty::Stated];
        let new_ety = |items| NewEtymology {
            mode: EtyMode::Borrowed,
            head: Some(0),
            basis: EdgeBasis::Template,
            items,
            confidences: &[1.0],
            certainties: &stated,
        };
        graph.add_etymologies(w, &[new_ety(&[a]), new_ety(&[b])]);
        graph.push_language_only(w, &[celtic]);
        let etys = graph.etymologies(w).unwrap();
        assert_eq!(etys.kinds[2], EtyKind::LanguageOnly(celtic));
        assert_eq!(etys.primary, None);
        // removing a word's etymology keeps it
        graph.remove_ety_through(w, a);
        let etys = graph.etymologies(w).unwrap();
        assert_eq!(
            etys.kinds[..],
            [EtyKind::Derivation, EtyKind::LanguageOnly(celtic)]
        );
        assert_eq!(etys.primary, Some(0));
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
