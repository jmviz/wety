use crate::{
    etymology_templates::EtyMode,
    lang_phf::LANG_CODE2NAME,
    phf_ext::OrderedMapExt,
    raw_items::RawItem,
    string_pool::{StringPool, Symbol},
};

use std::{
    fs::File,
    io::{BufWriter, Write},
    ops::Index,
    rc::Rc,
};

use anyhow::{anyhow, Ok, Result};
use hashbrown::{HashMap, HashSet};
use petgraph::{
    algo::greedy_feedback_arc_set,
    stable_graph::{EdgeIndex, NodeIndex, StableDiGraph},
    visit::EdgeRef,
};

type ImputedLangMap = HashMap<usize, Rc<RawItem>>;
type ImputedTermMap = HashMap<Symbol, ImputedLangMap>;

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
    pub(crate) term_map: ImputedTermMap,
    pub(crate) n: usize,
}

impl ImputedItems {
    fn add(&mut self, item: &Rc<RawItem>) {
        // check if the item's term has been seen before
        if !self.term_map.contains_key(&item.term) {
            let mut lang_map = ImputedLangMap::new();
            let term = item.term;
            lang_map.insert(item.lang, Rc::clone(item));
            self.term_map.insert(term, lang_map);
            self.n += 1;
            return;
        }
        // since term has been seen before, there must be at least one lang for it
        // check if item's lang has been seen before
        let lang_map: &mut ImputedLangMap = self.term_map.get_mut(&item.term).unwrap();
        if !lang_map.contains_key(&item.lang) {
            lang_map.insert(item.lang, Rc::clone(item));
            self.n += 1;
        }
    }
    pub(crate) fn get(&self, lang: usize, term: Symbol) -> Option<&Rc<RawItem>> {
        self.term_map
            .get(&term)
            .and_then(|lang_map| lang_map.get(&lang))
    }
}

pub(crate) struct EtyLink {
    pub(crate) mode: EtyMode,
    pub(crate) order: u8,
    pub(crate) head: bool,
}

#[derive(Default)]
pub(crate) struct EtyGraph {
    pub(crate) graph: StableDiGraph<Rc<RawItem>, EtyLink>,
    pub(crate) imputed_items: ImputedItems,
    pub(crate) index: HashMap<Rc<RawItem>, NodeIndex>,
}

pub(crate) struct ImmediateEty {
    pub(crate) items: Vec<Rc<RawItem>>,
    pub(crate) head: u8,
    pub(crate) mode: EtyMode,
}

impl ImmediateEty {
    fn head(&self) -> &Rc<RawItem> {
        &self.items[self.head as usize]
    }
}

pub(crate) struct Progenitors {
    pub(crate) items: HashSet<Rc<RawItem>>,
    pub(crate) head: Rc<RawItem>,
}

impl EtyGraph {
    fn get_index(&self, item: &Rc<RawItem>) -> NodeIndex {
        *self.index.get(item).expect("index previously added item")
    }
    pub(crate) fn get_immediate_ety(&self, item: &Rc<RawItem>) -> Option<ImmediateEty> {
        let item_index = self.get_index(item);
        let mut ety_items = vec![];
        let mut order = vec![];
        // Next two lines are dummy assignments. If there are any parents in the
        // ety_graph, they will get overwritten with correct values. If no
        // parents, they will not get returned.
        let mut head = 0;
        let mut mode = EtyMode::Derived;
        for (ety_link, ety_item) in self
            .graph
            .edges(item_index)
            .map(|e| (e.weight(), self.graph.index(e.target())))
        {
            ety_items.push(Rc::clone(ety_item));
            order.push(ety_link.order);
            mode = ety_link.mode;
            if ety_link.head {
                head = ety_link.order;
            }
        }
        ety_items = order
            .iter()
            .map(|&ord| Rc::clone(&ety_items[ord as usize]))
            .collect();
        (!ety_items.is_empty()).then_some(ImmediateEty {
            items: ety_items,
            mode,
            head,
        })
    }
    pub(crate) fn get_progenitors(&self, item: &Rc<RawItem>) -> Option<Progenitors> {
        struct Tracker {
            unexpanded: Vec<Rc<RawItem>>,
            progenitors: HashSet<Rc<RawItem>>,
            head: Rc<RawItem>,
        }
        fn recurse(ety_graph: &EtyGraph, t: &mut Tracker) {
            while let Some(item) = t.unexpanded.pop() {
                if let Some(immediate_ety) = ety_graph.get_immediate_ety(&item) {
                    let ety_head = immediate_ety.head();
                    for ety_item in &immediate_ety.items {
                        if t.head == item && ety_item == ety_head {
                            t.head = Rc::clone(ety_head);
                        }
                        t.unexpanded.push(Rc::clone(ety_item));
                    }
                    recurse(ety_graph, t);
                } else {
                    t.progenitors.insert(item);
                }
            }
        }
        let immediate_ety = self.get_immediate_ety(item)?;
        let head = Rc::clone(immediate_ety.head());
        let mut t = Tracker {
            unexpanded: immediate_ety.items,
            progenitors: HashSet::new(),
            head,
        };
        recurse(self, &mut t);
        let head = Rc::clone(&t.head);
        Some(Progenitors {
            items: t.progenitors,
            head,
        })
    }
    pub(crate) fn add(&mut self, item: &Rc<RawItem>) {
        let node_index = self.graph.add_node(Rc::clone(item));
        self.index.insert(Rc::clone(item), node_index);
    }
    pub(crate) fn add_imputed(&mut self, item: &Rc<RawItem>) {
        self.imputed_items.add(&Rc::clone(item));
        self.add(&Rc::clone(item));
    }
    pub(crate) fn add_ety(
        &mut self,
        item: &Rc<RawItem>,
        mode: EtyMode,
        head: u8,
        ety_items: &[Rc<RawItem>],
    ) {
        let item_index = self.get_index(item);
        // StableGraph allows adding multiple parallel edges from one node
        // to another. So we have to be careful not to override any already
        // existing ety links (e.g. from raw descendants which have been
        // processed before raw etymology.)
        if self.graph.edges(item_index).next().is_some() {
            return;
        }
        for (i, ety_item) in (0u8..).zip(ety_items.iter()) {
            let ety_item_index = self.get_index(ety_item);
            let ety_link = EtyLink {
                mode,
                order: i,
                head: head == i,
            };
            self.graph.add_edge(item_index, ety_item_index, ety_link);
        }
    }
    pub(crate) fn remove_cycles(&mut self, string_pool: &StringPool, pass: u8) -> Result<()> {
        println!("  Checking for ety link feedback arc set, pass {pass}...");
        let filename = format!("data/feedback_arc_set_pass_{pass}.tsv");
        let mut f = BufWriter::new(File::create(&filename)?);
        writeln!(f, "child_lang\tchild_term\tparent_lang\tparent_term")?;
        let fas: Vec<EdgeIndex> = greedy_feedback_arc_set(&self.graph)
            .map(|e| e.id())
            .collect();
        if fas.is_empty() {
            println!("    Found none. Writing blank {filename}.");
        } else {
            println!(
                "    Found ety link feedback arc set of size {}. Writing to {filename}.",
                fas.len()
            );

            for edge in fas {
                let (source, target) = self
                    .graph
                    .edge_endpoints(edge)
                    .ok_or_else(|| anyhow!("feedback arc set edge endpoints not found"))?;
                let source_item = self.graph.index(source);
                let target_item = self.graph.index(target);
                writeln!(
                    f,
                    "{}\t{}\t{}\t{}",
                    LANG_CODE2NAME
                        .get_expected_index_value(source_item.lang)
                        .unwrap(),
                    string_pool.resolve(source_item.term),
                    LANG_CODE2NAME
                        .get_expected_index_value(target_item.lang)
                        .unwrap(),
                    string_pool.resolve(target_item.term),
                )?;
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
        f.flush()?;
        Ok(())
    }
}
