//! WIP attempt to digest etymologies from wiktextract data

#![feature(is_some_and, let_chains, vec_push_within_capacity)]
#![allow(clippy::redundant_closure_for_method_calls)]

mod embeddings;
mod etymology_templates;
mod lang;
mod pos;
mod turtle;

use crate::{
    etymology_templates::{EtyMode, TemplateKind},
    lang::{LANG_CODE2NAME, LANG_ETYCODE2CODE, LANG_NAME2CODE, LANG_RECONSTRUCTED},
    pos::POS,
    turtle::write_turtle_file,
};

use std::{
    convert::TryFrom,
    fs::{remove_dir_all, File},
    io::{BufReader, BufWriter, Read, Write},
    ops::Index,
    path::Path,
    rc::Rc,
    str::FromStr,
    time::Instant,
};

use anyhow::{anyhow, Ok, Result};
use bytelines::ByteLines;
use embeddings::Embeddings;
use flate2::read::GzDecoder;
use hashbrown::{HashMap, HashSet};
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use oxigraph::{io::GraphFormat::Turtle, model::GraphNameRef::DefaultGraph, store::Store};
use petgraph::{
    algo::greedy_feedback_arc_set,
    stable_graph::{EdgeIndex, NodeIndex, StableDiGraph},
    visit::EdgeRef,
};
use phf::{phf_set, OrderedMap, OrderedSet, Set};
use regex::Regex;
use simd_json::{to_borrowed_value, value::borrowed::Value, ValueAccess};
use string_interner::{backend::StringBackend, symbol::SymbolU32, StringInterner};

#[derive(Hash, Eq, PartialEq, Debug)]
struct RawEtymology {
    templates: Box<[RawEtyTemplate]>,
}

impl From<Vec<RawEtyTemplate>> for RawEtymology {
    fn from(templates: Vec<RawEtyTemplate>) -> Self {
        Self {
            templates: templates.into_boxed_slice(),
        }
    }
}

// models the basic info from a wiktionary etymology template
#[derive(Hash, Eq, PartialEq, Debug)]
struct RawEtyTemplate {
    langs: Box<[usize]>,     // e.g. "en", "en"
    terms: Box<[SymbolU32]>, // e.g. "re-", "do"
    mode: EtyMode,           // e.g. Prefix
    head: u8,                // e.g. 1 (the index of "do")
}

impl RawEtyTemplate {
    fn new(lang: usize, term: SymbolU32, mode: EtyMode) -> Self {
        Self {
            langs: Box::new([lang]),
            terms: Box::new([term]),
            mode,
            head: 0,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct RawRoot {
    lang: usize,
    term: SymbolU32,
    sense_id: Option<SymbolU32>,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct RawDescendants {
    lines: Box<[RawDescLine]>,
}

impl From<Vec<RawDescLine>> for RawDescendants {
    fn from(descendants: Vec<RawDescLine>) -> Self {
        Self {
            lines: descendants.into_boxed_slice(),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct RawDescLine {
    depth: u8,
    kind: RawDescLineKind,
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum RawDescLineKind {
    Desc { desc: RawDesc },
    // e.g. {{desc|osp|-}}, {{desc|itc-pro|}},
    BareLang { lang: usize },
    // i.e. line with no templates e.g. "Unsorted Formations", "with prefix -a"
    BareText { text: SymbolU32 },
    // e.g. a line with {{PIE root see}} or some other unhandled template(s)
    // or unexpected form of above line kinds
    Other,
    // stretch goal: https://en.wiktionary.org/wiki/Template:CJKV
}

// some combination of desc, l, desctree templates that together provide one or
// more descendant lang, term, mode combos
#[derive(Hash, Eq, PartialEq, Debug)]
struct RawDesc {
    lang: usize,
    terms: Box<[SymbolU32]>,
    modes: Box<[EtyMode]>,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct Item {
    line: Option<usize>, // the line-th ok line in the wiktextract file, if it was in the file
    is_imputed: bool,
    is_reconstructed: bool, // i.e. Reconstruction: namespace page, or an imputed item of form *term
    i: usize,               // the i-th item seen, used as id for RDF
    lang: usize,            // e.g "en", i.e. the wiktextract lang_code
    term: SymbolU32,        // e.g. "bank"
    page_title: Option<SymbolU32>, // i.e. the term stripped of diacritics etc. at the top of the page
    ety_num: Option<u8>,           // the nth numbered ety for this term-lang combo (1,2,...)
    pos: Option<usize>,            // e.g. "noun"
    gloss: Option<SymbolU32>,      // e.g. "An institution where one can place and borrow money...
    gloss_num: u8,                 // the nth gloss encountered for this term-lang-ety-pos combo
    raw_etymology: Option<RawEtymology>,
    raw_root: Option<RawRoot>,
    raw_descendants: Option<RawDescendants>,
}

impl Item {
    fn new_imputed(i: usize, lang: usize, term: SymbolU32, pos: Option<usize>) -> Self {
        Self {
            line: None,
            is_imputed: true,
            // $$ This will not catch all reconstructed terms, since some terms
            // in attested languages are reconstructed. Some better inference
            // should be done based on "*" prefix for terms.
            is_reconstructed: is_reconstructed_lang(lang),
            i,
            lang,
            term,
            pos,
            page_title: None,
            ety_num: None,
            gloss_num: 0,
            gloss: None,
            raw_etymology: None,
            raw_root: None,
            raw_descendants: None,
        }
    }
}

type GlossMap = HashMap<Option<SymbolU32>, Rc<Item>>;
type PosMap = HashMap<Option<usize>, GlossMap>;
type EtyMap = HashMap<Option<u8>, PosMap>;
type LangMap = HashMap<usize, EtyMap>;
type TermMap = HashMap<SymbolU32, LangMap>;

#[derive(Default)]
struct Items {
    term_map: TermMap,
    n: usize,
    redirects: Redirects,
    line_map: HashMap<usize, Rc<Item>>,
}

impl Items {
    fn add_to_term_map(&mut self, mut item: Item) -> Result<Option<Rc<Item>>> {
        // check if the item's term has been seen before
        if !self.term_map.contains_key(&item.term) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let mut lang_map = LangMap::new();
            let (pos, ety_num, lang, term) = (item.pos, item.ety_num, item.lang, item.term);
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_num, pos_map);
            lang_map.insert(lang, ety_map);
            self.term_map.insert(term, lang_map);
            self.n += 1;
            return Ok(Some(Rc::clone(&item)));
        }
        // since term has been seen before, there must be at least one lang for it
        // check if item's lang has been seen before
        let lang_map: &mut LangMap = self.term_map.get_mut(&item.term).unwrap();
        if !lang_map.contains_key(&item.lang) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let mut ety_map = EtyMap::new();
            let (pos, ety_num, lang) = (item.pos, item.ety_num, item.lang);
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_num, pos_map);
            lang_map.insert(lang, ety_map);
            self.n += 1;
            return Ok(Some(Rc::clone(&item)));
        }
        // since lang has been seen before, there must be at least one ety (possibly None)
        // check if this ety has been seen in this lang before
        let ety_map: &mut EtyMap = lang_map.get_mut(&item.lang).unwrap();
        if !ety_map.contains_key(&item.ety_num) {
            let mut gloss_map = GlossMap::new();
            let mut pos_map = PosMap::new();
            let (pos, ety_num) = (item.pos, item.ety_num);
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(pos, gloss_map);
            ety_map.insert(ety_num, pos_map);
            self.n += 1;
            return Ok(Some(Rc::clone(&item)));
        }
        // since ety has been seen before, there must be at least one pos
        // check if this pos has been seen for this ety before
        let pos_map: &mut PosMap = ety_map.get_mut(&item.ety_num).unwrap();
        if !pos_map.contains_key(&item.pos) {
            let mut gloss_map = GlossMap::new();
            let pos = item.pos;
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            pos_map.insert(pos, gloss_map);
            self.n += 1;
            return Ok(Some(Rc::clone(&item)));
        }
        // since pos has been seen before, there must be at least one gloss (possibly None)
        let gloss_map: &mut GlossMap = pos_map.get_mut(&item.pos).unwrap();
        if !gloss_map.contains_key(&item.gloss) {
            item.gloss_num = u8::try_from(gloss_map.len())?;
            let item = Rc::from(item);
            gloss_map.insert(item.gloss, Rc::clone(&item));
            self.n += 1;
            return Ok(Some(Rc::clone(&item)));
        }
        Ok(None)
    }

    fn rectify_lang_term(&self, lang: usize, term: SymbolU32) -> (usize, SymbolU32) {
        // If lang is an etymology-only language, we will not find any entries
        // for it in Items lang map, since such a language definitionally does
        // not have any entries itself. So we look for the actual lang that the
        // ety lang is associated with.
        let lang = etylang2lang(lang);
        // Then we also check if there is a redirect for this lang term combo.
        self.redirects.get(lang, term)
    }

    fn contains(&self, lang: usize, term: SymbolU32) -> bool {
        let (lang, term) = self.rectify_lang_term(lang, term);
        self.term_map
            .get(&term)
            .map_or(false, |lang_map| lang_map.contains_key(&lang))
    }

    fn get_disambiguated_item(
        &self,
        string_pool: &StringPool,
        sense: &Sense,
        lang: usize,
        term: SymbolU32,
    ) -> Option<&Rc<Item>> {
        let (lang, term) = self.rectify_lang_term(lang, term);
        self.term_map
            .get(&term)
            .and_then(|lang_map| lang_map.get(&lang))
            // If an ety_map is found, that means there is at least one item to
            // collect after this nested iteration. See logic in Items::add()
            // for why. Therefore, this function will always return either a
            // non-empty Vec or None.
            .and_then(|ety_map| {
                ety_map
                    .values()
                    .flat_map(|pos_map| pos_map.values().flat_map(|gloss_map| gloss_map.values()))
                    .max_by_key(|candidate| {
                        let candidate_sense = Sense::new(string_pool, candidate);
                        sense.lesk_score(&candidate_sense)
                    })
            })
    }

    fn get_item_lang_term_duplicates(&self, item: &Rc<Item>) -> Option<Vec<Rc<Item>>> {
        let lang_map = self.term_map.get(&item.term)?;
        let ety_map = lang_map.get(&item.lang)?;
        let mut duplicates = vec![];
        for pos_map in ety_map.values() {
            for gloss_map in pos_map.values() {
                for duplicate in gloss_map.values() {
                    if duplicate != item {
                        duplicates.push(Rc::clone(duplicate));
                    }
                }
            }
        }
        (!duplicates.is_empty()).then_some(duplicates)
    }

    fn item_has_duplicates(&self, item: &Rc<Item>) -> bool {
        self.get_item_lang_term_duplicates(item).is_some()
    }

    // For now we'll just take the first template. But cf. notes.md.
    // Only to be called once all json items have been processed into items.
    fn process_item_raw_etymology(
        &self,
        string_pool: &StringPool,
        ety_graph: &mut EtyGraph,
        item: &Rc<Item>,
    ) {
        if item.raw_etymology.is_none() {
            return; // don't add anything to ety_graph if no valid raw ety templates
        }
        let mut current_item = Rc::clone(item); // for tracking possibly imputed items
        let mut next_item = Rc::clone(item); // for tracking possibly imputed items
        for template in item.raw_etymology.as_ref().unwrap().templates.iter() {
            let sense = Sense::new(string_pool, &current_item);
            let mut ety_items = Vec::with_capacity(template.terms.len());
            let mut has_new_imputation = false;
            for (&ety_lang, &ety_term) in template.langs.iter().zip(template.terms.iter()) {
                if let Some(ety_item) =
                    self.get_disambiguated_item(string_pool, &sense, ety_lang, ety_term)
                {
                    // There exists at least one item for this lang term combo.
                    // We have to do a word sense disambiguation in case there
                    // are multiple items.
                    ety_items.push(Rc::clone(ety_item));
                } else if let Some(imputed_ety_item) =
                    ety_graph.imputed_items.get(ety_lang, ety_term)
                {
                    // We have already imputed an item that corresponds to this term.
                    ety_items.push(Rc::clone(imputed_ety_item));
                } else if template.terms.len() == 1 {
                    // This is an unseen term, and it is in a non-compound-kind template.
                    // We will impute an item for this term, and use this new imputed
                    // item as the item for the next template in the outer loop.
                    has_new_imputation = true;
                    let n = self.n + ety_graph.imputed_items.n;
                    // We previously assumed the imputed item has the same pos as the current_item.
                    // How often is this not the case?
                    let imputed_ety_item = Rc::from(Item::new_imputed(n, ety_lang, ety_term, None));
                    ety_graph.add_imputed(&imputed_ety_item);
                    ety_items.push(Rc::clone(&imputed_ety_item));
                    next_item = Rc::clone(&imputed_ety_item);
                } else {
                    // This is a term of a compound-kind template without a
                    // link, and for which a corresponding imputed item has not
                    // yet been created. We won't bother trying to do convoluted
                    // imputations for such cases at the moment. So we stop
                    // processing templates here.
                    return;
                }
            }
            ety_graph.add_ety(&current_item, template.mode, template.head, &ety_items);
            // We keep processing templates until we hit the first one with no
            // imputation required.
            if !has_new_imputation {
                return;
            }
            current_item = Rc::clone(&next_item);
        }
    }

    fn process_item_raw_descendants(
        &self,
        string_pool: &StringPool,
        ety_graph: &mut EtyGraph,
        item: &Rc<Item>,
    ) {
        #[derive(Clone, PartialEq, Eq)]
        struct Ancestor {
            item: Rc<Item>,
            depth: u8,
        }
        impl Ancestor {
            fn new(item: &Rc<Item>, depth: u8) -> Self {
                Self {
                    item: Rc::clone(item),
                    depth,
                }
            }
        }
        struct Ancestors {
            ancestors: Vec<Ancestor>,
            progenitor: Ancestor,
        }
        impl Ancestors {
            fn new(item: &Rc<Item>) -> Self {
                Self {
                    ancestors: vec![],
                    progenitor: Ancestor::new(item, 0),
                }
            }
            fn prune(&mut self, depth: u8) {
                while let Some(ancestor) = self.ancestors.last().map(Ancestor::clone)
                    && depth <= ancestor.depth
                {
                    self.ancestors.pop();
                }
            }
            fn prune_and_get_parent(&mut self, depth: u8) -> Ancestor {
                self.prune(depth);
                self.ancestors
                    .last()
                    .map_or_else(|| self.progenitor.clone(), Ancestor::clone)
            }
            fn add(&mut self, item: &Rc<Item>, depth: u8) {
                let ancestor = Ancestor::new(item, depth);
                self.ancestors.push(ancestor);
            }
        }

        if item.raw_descendants.is_none() {
            return;
        }
        let mut ancestors = Ancestors::new(item);
        let mut ancestor_senses = HashMap::new();
        'outer: for line in item.raw_descendants.as_ref().unwrap().lines.iter() {
            let parent = ancestors.prune_and_get_parent(line.depth);
            match &line.kind {
                RawDescLineKind::Desc { desc } => {
                    if desc.terms.is_empty() || desc.terms.len() != desc.modes.len() {
                        continue;
                    }
                    let lang = desc.lang;
                    let parent_sense = ancestor_senses
                        .entry(parent.item.i)
                        .or_insert_with(|| Sense::new(string_pool, &parent.item));
                    let (mut desc_items, mut modes) = (vec![], vec![]);
                    for (i, (&term, &mode)) in desc.terms.iter().zip(desc.modes.iter()).enumerate()
                    {
                        let desc_item = self
                            .get_disambiguated_item(string_pool, parent_sense, lang, term)
                            .or_else(|| ety_graph.imputed_items.get(lang, term))
                            .map(Rc::clone);
                        // Borrow checker complains when I use map_or_else
                        // instead of map then unwrap_or_else. But if I
                        // chain these last two then clippy::pedantic
                        // complains...
                        let desc_item = desc_item.unwrap_or_else(|| {
                            let n = self.n + ety_graph.imputed_items.n;
                            let imputed_item = Rc::from(Item::new_imputed(n, lang, term, None));
                            ety_graph.add_imputed(&imputed_item);
                            imputed_item
                        });
                        // A root generally shouldn't be listed as a descendant
                        // of another term. If it really is an etymological
                        // child, we will rely on the etymology section of the
                        // root to get the relationship. In descendants trees,
                        // creating this link will probably more often than not
                        // be a mistake. See e.g. page for PIE men-, where
                        // compound of men- and dʰeh₁- is listed. If we didn't
                        // skip the template featuring dʰeh₁-, then we would
                        // erroneously add an ety link from men- to dʰeh₁-. In
                        // general, we may need to end up doing much smarter
                        // processing of descendants sections if there is more
                        // such variation I am unaware of (probable?).
                        if desc_item
                            .pos
                            .is_some_and(|pos| pos == POS.get_expected_index("root").unwrap())
                        {
                            continue 'outer;
                        }
                        if i == 0 {
                            ancestors.add(&desc_item, line.depth);
                        }
                        desc_items.push(desc_item);
                        modes.push(mode);
                    }
                    for (desc_item, mode) in desc_items.iter().zip(modes) {
                        ety_graph.add_ety(desc_item, mode, 0, &[Rc::clone(&parent.item)]);
                    }
                }
                // Might want to do something for the other cases in the future,
                // e.g. impute placeholder "items" that have no info, or only
                // lang info (perhaps by making Item an enum?), to indicate that
                // there is a known missing step in the ety chain. Right now, we
                // just skip them. So e.g. for a descendants snippet like on the
                // page for PIE ḱerh₂-:
                //
                // * Unsorted formations: [BareText]
                // ** {{desc|grk-pro|}} [BareLang]
                // *** {{desc|grc|κάρυον}} [Desc]
                //
                // our resultant ety chain would just be  κάρυον -> ḱerh₂-.
                _ => continue,
            }
        }
    }

    fn add_all_to_ety_graph(&self, ety_graph: &mut EtyGraph) -> Result<()> {
        let pb = ProgressBar::new(u64::try_from(self.n)?);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} Adding items to ety graph: [{elapsed}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({per_sec}, {eta})")?
            .progress_chars("#>-"));
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            ety_graph.add(&Rc::clone(item));
                            pb.inc(1);
                        }
                    }
                }
            }
        }

        pb.finish();
        Ok(())
    }

    fn impute_root_items(&self, ety_graph: &mut EtyGraph) -> Result<()> {
        let root_pos = Some(POS.get_expected_index("root")?);
        let pb = ProgressBar::new(u64::try_from(self.n)?);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} Imputing roots: [{elapsed}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({per_sec}, {eta})")?
            .progress_chars("#>-"));
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            if let Some(raw_root) = &item.raw_root
                                && !self.contains(raw_root.lang, raw_root.term)
                            {
                                let i = self.n + ety_graph.imputed_items.n;
                                let root = Rc::from(Item::new_imputed(
                                    i,
                                    raw_root.lang,
                                    raw_root.term,
                                    root_pos,
                                ));
                                ety_graph.add_imputed(&root);
                            }
                            pb.inc(1);
                        }
                    }
                }
            }
        }
        pb.finish();
        Ok(())
    }

    fn process_raw_etymologies(
        &self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
    ) -> Result<()> {
        let pb = ProgressBar::new(u64::try_from(self.n)?);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} Processing etymologies: [{elapsed}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({per_sec}, {eta})")?
            .progress_chars("#>-"));
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            self.process_item_raw_etymology(string_pool, ety_graph, item);
                            pb.inc(1);
                        }
                    }
                }
            }
        }
        pb.finish();
        Ok(())
    }

    fn process_raw_descendants(
        &self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
    ) -> Result<()> {
        let pb = ProgressBar::new(u64::try_from(self.n)?);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} Processing descendants: [{elapsed}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({per_sec}, {eta})")?
            .progress_chars("#>-"));
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            self.process_item_raw_descendants(string_pool, ety_graph, item);
                            pb.inc(1);
                        }
                    }
                }
            }
        }
        pb.finish();
        Ok(())
    }

    fn impute_item_root_ety(
        &self,
        string_pool: &StringPool,
        ety_graph: &mut EtyGraph,
        sense: &Sense,
        item: &Rc<Item>,
    ) {
        if let Some(raw_root) = &item.raw_root
            && let Some(root_item) = self
                .get_disambiguated_item(string_pool, sense, raw_root.lang, raw_root.term)
                .or_else(|| ety_graph.imputed_items.get(raw_root.lang, raw_root.term))
        {
            let mut visited_items: HashSet<Rc<Item>> =
                HashSet::from([Rc::clone(item), Rc::clone(root_item)]);
            let mut current_item = Rc::clone(item);
            while let Some(immediate_ety) = ety_graph.get_immediate_ety(&current_item) {
                // If the root or any previously visited item is encountered
                // again, don't impute anything, so we don't create or get
                // caught in a cycle.
                if visited_items.contains(&current_item) {
                    return;
                }
                // If there are multiple ety items but one has the same root, pick this.
                let ety_item = immediate_ety.items.iter().find(|ety_item| {
                    ety_item.raw_root.as_ref()
                        .and_then(|r| {
                            let ety_item_sense = Sense::new(string_pool, ety_item);
                            self
                                .get_disambiguated_item(string_pool, &ety_item_sense, r.lang, r.term)
                                .or_else(|| ety_graph.imputed_items.get(r.lang, r.term))})
                        .is_some_and(|ety_root_item| ety_root_item == root_item)
                });
                // Otherwise, return so we don't guess wrong.
                if ety_item.is_none() && immediate_ety.items.len() > 1 {
                    return
                }
                // There is no chance of guessing wrong if only 1 ety item.
                let ety_item = ety_item.unwrap_or(&immediate_ety.items[0]);
                current_item = Rc::clone(ety_item);
                visited_items.insert(Rc::clone(&current_item));
            }
            if &current_item != root_item {
                ety_graph.add_ety(&current_item, EtyMode::Root, 0u8, &[Rc::clone(root_item)]);
            }
        }
    }

    fn impute_root_etys(
        &self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
    ) -> Result<()> {
        let pb = ProgressBar::new(u64::try_from(self.n)?);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} Imputing root etys: [{elapsed}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({per_sec}, {eta})")?
            .progress_chars("#>-"));
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            let sense = Sense::new(string_pool, item);
                            self.impute_item_root_ety(string_pool, ety_graph, &sense, item);
                            pb.inc(1);
                        }
                    }
                }
            }
        }
        pb.finish();
        Ok(())
    }

    // We go through the wiktextract file again, generating embeddings for all
    // ambiguous terms we found the first time.
    fn generate_embeddings(&self, path: &Path) -> Result<Embeddings> {
        let mut embeddings = Embeddings::new()?;
        for (line_number, mut line) in wiktextract_lines(path)?.enumerate() {
            // Items were only inserted into the line map if they were added to
            // the term_map in process_json_item.
            if let Some(item) = self.line_map.get(&line_number) {
                let json_item = to_borrowed_value(&mut line)?;
                if self.item_has_duplicates(item) {
                    embeddings.add(&json_item, item.i)?;
                }
            }
        }
        Ok(embeddings)
    }

    fn generate_ety_graph(
        &self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
    ) -> Result<EtyGraph> {
        let mut ety_graph = EtyGraph::default();
        self.add_all_to_ety_graph(&mut ety_graph)?;
        self.impute_root_items(&mut ety_graph)?;
        self.process_raw_descendants(string_pool, embeddings, &mut ety_graph)?;
        ety_graph.remove_cycles(string_pool, 1)?;
        self.process_raw_etymologies(string_pool, embeddings, &mut ety_graph)?;
        ety_graph.remove_cycles(string_pool, 2)?;
        self.impute_root_etys(string_pool, embeddings, &mut ety_graph)?;
        ety_graph.remove_cycles(string_pool, 3)?;
        Ok(ety_graph)
    }
}

type ImputedLangMap = HashMap<usize, Rc<Item>>;
type ImputedTermMap = HashMap<SymbolU32, ImputedLangMap>;

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
struct ImputedItems {
    term_map: ImputedTermMap,
    n: usize,
}

impl ImputedItems {
    fn add(&mut self, item: &Rc<Item>) {
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
    fn get(&self, lang: usize, term: SymbolU32) -> Option<&Rc<Item>> {
        self.term_map
            .get(&term)
            .and_then(|lang_map| lang_map.get(&lang))
    }
}

struct EtyLink {
    mode: EtyMode,
    order: u8,
    head: bool,
}

#[derive(Default)]
struct EtyGraph {
    graph: StableDiGraph<Rc<Item>, EtyLink>,
    imputed_items: ImputedItems,
    index: HashMap<Rc<Item>, NodeIndex>,
}

struct ImmediateEty {
    items: Vec<Rc<Item>>,
    head: u8,
    mode: EtyMode,
}

impl ImmediateEty {
    fn head(&self) -> &Rc<Item> {
        &self.items[self.head as usize]
    }
}

struct Progenitors {
    items: HashSet<Rc<Item>>,
    head: Rc<Item>,
}

impl EtyGraph {
    fn get_index(&self, item: &Rc<Item>) -> NodeIndex {
        *self.index.get(item).expect("index previously added item")
    }
    fn get_immediate_ety(&self, item: &Rc<Item>) -> Option<ImmediateEty> {
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
    fn get_progenitors(&self, item: &Rc<Item>) -> Option<Progenitors> {
        struct Tracker {
            unexpanded: Vec<Rc<Item>>,
            progenitors: HashSet<Rc<Item>>,
            head: Rc<Item>,
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
    fn add(&mut self, item: &Rc<Item>) {
        let node_index = self.graph.add_node(Rc::clone(item));
        self.index.insert(Rc::clone(item), node_index);
    }
    fn add_imputed(&mut self, item: &Rc<Item>) {
        self.imputed_items.add(&Rc::clone(item));
        self.add(&Rc::clone(item));
    }
    fn add_ety(&mut self, item: &Rc<Item>, mode: EtyMode, head: u8, ety_items: &[Rc<Item>]) {
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
    fn remove_cycles(&mut self, string_pool: &StringPool, pass: u8) -> Result<()> {
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

#[derive(Default)]
struct StringPool {
    pool: StringInterner<StringBackend<SymbolU32>>,
}

impl StringPool {
    fn resolve(&self, symbol: SymbolU32) -> &str {
        self.pool
            .resolve(symbol)
            .expect("Resolve interned string from symbol")
    }
    fn get_or_intern(&mut self, s: &str) -> SymbolU32 {
        self.pool.get_or_intern(s)
    }
}

// Always short-lived struct used for sense disambiguation.
struct Sense {
    gloss: HashSet<String>,
}

impl Sense {
    fn new(string_pool: &StringPool, item: &Rc<Item>) -> Sense {
        let mut gloss = HashSet::new();
        let gloss_str = item.gloss.map_or("", |g| string_pool.resolve(g));
        for word in remove_punctuation(gloss_str).split_whitespace() {
            gloss.insert(word.to_string());
        }
        Sense { gloss }
    }
    // https://en.wikipedia.org/wiki/Lesk_algorithm
    fn lesk_score(&self, other: &Sense) -> usize {
        self.gloss.intersection(&other.gloss).count()
    }
}

// convenience extension trait for reading from json
trait ValueExt {
    fn get_valid_str(&self, key: &str) -> Option<&str>;
}

impl ValueExt for Value<'_> {
    // return a cleaned version of the str if it exists
    fn get_valid_str(&self, key: &str) -> Option<&str> {
        self.get_str(key)
            // even though get_valid_str is called on other bits of wiktextract
            // json such as template lang args, clean_ety_term should never
            // effect them unless they're degenerate anyway, so we always call
            // this
            .map(clean_ety_term)
            .and_then(|s| (!s.is_empty() && s != "-").then_some(s))
    }
}

// convenience extension trait methods for dealing with ordered maps and sets
trait OrderedMapExt {
    fn get_index_key(&self, index: usize) -> Option<&str>;
    fn get_expected_index_key(&self, index: usize) -> Result<&str>;
    fn get_index_value(&self, index: usize) -> Option<&str>;
    fn get_expected_index_value(&self, index: usize) -> Result<&str>;
    fn get_expected_index(&self, key: &str) -> Result<usize>;
}

impl OrderedMapExt for OrderedMap<&str, &str> {
    fn get_index_key(&self, index: usize) -> Option<&str> {
        self.index(index).map(|(&key, _)| key)
    }
    fn get_expected_index_key(&self, index: usize) -> Result<&str> {
        self.get_index_key(index)
            .ok_or_else(|| anyhow!("The index {index} does not exist."))
    }
    fn get_index_value(&self, index: usize) -> Option<&str> {
        self.index(index).map(|(_, &value)| value)
    }
    fn get_expected_index_value(&self, index: usize) -> Result<&str> {
        self.get_index_value(index)
            .ok_or_else(|| anyhow!("The index {index} does not exist."))
    }
    fn get_expected_index(&self, key: &str) -> Result<usize> {
        self.get_index(key)
            .ok_or_else(|| anyhow!("The key '{key}' does not exist."))
    }
}

trait OrderedSetExt {
    fn get_expected_index_key(&self, index: usize) -> Result<&str>;
    fn get_expected_index(&self, key: &str) -> Result<usize>;
}

impl OrderedSetExt for OrderedSet<&str> {
    fn get_expected_index_key(&self, index: usize) -> Result<&str> {
        self.index(index)
            .copied()
            .ok_or_else(|| anyhow!("The index {index} does not exist."))
    }
    fn get_expected_index(&self, key: &str) -> Result<usize> {
        self.get_index(key)
            .ok_or_else(|| anyhow!("The key '{key}' does not exist."))
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct ReconstructionTitle {
    language: usize,
    term: SymbolU32,
}

#[derive(Default)]
struct Redirects {
    reconstruction: HashMap<ReconstructionTitle, ReconstructionTitle>,
    regular: HashMap<SymbolU32, SymbolU32>,
}

impl Redirects {
    // If a redirect page exists for given lang + term combo, get the redirect.
    // If not, just return back the original lang + term.
    fn get(&self, lang: usize, term: SymbolU32) -> (usize, SymbolU32) {
        if let Some(language) = LANG_CODE2NAME.get_index_value(lang)
            && let Some(language_index) = LANG_NAME2CODE.get_index(language)
            && let Some(redirect) = self.reconstruction.get(&ReconstructionTitle {
                language: language_index,
                term,
            })
            && let Some(redirect_lang) = LANG_NAME2CODE.get_index_value(redirect.language)
            && let Some(redirect_lang_index) = LANG_CODE2NAME.get_index(redirect_lang)
        {
            return (redirect_lang_index, redirect.term);
        } else if let Some(&redirect_term) = self.regular.get(&term) {
                return (lang, redirect_term);
        }
        (lang, term)
    }
}

struct RawDataProcessor {
    string_pool: StringPool,
}

impl RawDataProcessor {
    fn new() -> Result<Self> {
        Ok(Self {
            string_pool: StringPool::default(),
        })
    }
    fn process_derived_kind_json_template(
        &mut self,
        args: &Value,
        mode: EtyMode,
        lang: &str,
    ) -> Option<RawEtyTemplate> {
        let term_lang = args.get_valid_str("1")?;
        (term_lang == lang).then_some(())?;
        let ety_lang = args.get_valid_str("2")?;
        let ety_lang_index = LANG_CODE2NAME.get_index(ety_lang)?;
        let ety_term = args.get_valid_str("3")?;

        let ety_term = self.string_pool.get_or_intern(ety_term);
        Some(RawEtyTemplate::new(ety_lang_index, ety_term, mode))
    }

    fn process_abbrev_kind_json_template(
        &mut self,
        args: &Value,
        mode: EtyMode,
        lang: &str,
    ) -> Option<RawEtyTemplate> {
        let term_lang = args.get_valid_str("1")?;
        (term_lang == lang).then_some(())?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let ety_term = args.get_valid_str("2")?;

        let ety_term = self.string_pool.get_or_intern(ety_term);
        Some(RawEtyTemplate::new(lang_index, ety_term, mode))
    }

    fn process_prefix_json_template(&mut self, args: &Value, lang: &str) -> Option<RawEtyTemplate> {
        let term_lang = args.get_valid_str("1")?;
        (term_lang == lang).then_some(())?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let ety_prefix = args.get_valid_str("2")?;
        let ety_term = args.get_valid_str("3")?;

        let ety_term = self.string_pool.get_or_intern(ety_term);
        let ety_prefix = format!("{ety_prefix}-");
        let ety_prefix = ety_prefix.as_str();
        let ety_prefix = self.string_pool.get_or_intern(ety_prefix);
        Some(RawEtyTemplate {
            langs: Box::new([lang_index; 2]),
            terms: Box::new([ety_prefix, ety_term]),
            mode: EtyMode::Prefix,
            head: 1,
        })
    }

    fn process_suffix_json_template(&mut self, args: &Value, lang: &str) -> Option<RawEtyTemplate> {
        let term_lang = args.get_valid_str("1")?;
        (term_lang == lang).then_some(())?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let ety_term = args.get_valid_str("2")?;
        let ety_suffix = args.get_valid_str("3")?;

        let ety_term = self.string_pool.get_or_intern(ety_term);
        let ety_suffix = format!("-{ety_suffix}");
        let ety_suffix = ety_suffix.as_str();
        let ety_suffix = self.string_pool.get_or_intern(ety_suffix);
        Some(RawEtyTemplate {
            terms: Box::new([ety_term, ety_suffix]),
            langs: Box::new([lang_index; 2]),
            mode: EtyMode::Suffix,
            head: 0,
        })
    }

    fn process_circumfix_json_template(
        &mut self,
        args: &Value,
        lang: &str,
    ) -> Option<RawEtyTemplate> {
        let term_lang = args.get_valid_str("1")?;
        (term_lang == lang).then_some(())?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let ety_prefix = args.get_valid_str("2")?;
        let ety_term = args.get_valid_str("3")?;
        let ety_suffix = args.get_valid_str("4")?;

        let ety_term = self.string_pool.get_or_intern(ety_term);
        let ety_circumfix = format!("{ety_prefix}- -{ety_suffix}");
        let ety_circumfix = ety_circumfix.as_str();
        let ety_circumfix = self.string_pool.get_or_intern(ety_circumfix);
        Some(RawEtyTemplate {
            terms: Box::new([ety_term, ety_circumfix]),
            langs: Box::new([lang_index; 2]),
            mode: EtyMode::Circumfix,
            head: 0,
        })
    }

    fn process_infix_json_template(&mut self, args: &Value, lang: &str) -> Option<RawEtyTemplate> {
        let term_lang = args.get_valid_str("1")?;
        (term_lang == lang).then_some(())?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let ety_term = args.get_valid_str("2")?;
        let ety_infix = args.get_valid_str("3")?;

        let ety_term = self.string_pool.get_or_intern(ety_term);
        let ety_infix = format!("-{ety_infix}-");
        let ety_infix = ety_infix.as_str();
        let ety_infix = self.string_pool.get_or_intern(ety_infix);
        Some(RawEtyTemplate {
            terms: Box::new([ety_term, ety_infix]),
            langs: Box::new([lang_index; 2]),
            mode: EtyMode::Infix,
            head: 0,
        })
    }

    fn process_confix_json_template(&mut self, args: &Value, lang: &str) -> Option<RawEtyTemplate> {
        let term_lang = args.get_valid_str("1")?;
        (term_lang == lang).then_some(())?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let ety_prefix = args.get_valid_str("2")?;
        let ety2 = args.get_valid_str("3")?;

        let ety_prefix = format!("{ety_prefix}-");
        let ety_prefix = ety_prefix.as_str();
        let ety_prefix = self.string_pool.get_or_intern(ety_prefix);
        if let Some(ety3) = args.get_valid_str("4") {
            let ety_term = self.string_pool.get_or_intern(ety2);
            let ety_suffix = format!("-{ety3}");
            let ety_suffix = ety_suffix.as_str();
            let ety_suffix = self.string_pool.get_or_intern(ety_suffix);
            return Some(RawEtyTemplate {
                terms: Box::new([ety_prefix, ety_term, ety_suffix]),
                langs: Box::new([lang_index; 3]),
                mode: EtyMode::Confix,
                head: 1,
            });
        }
        let ety_suffix = format!("-{ety2}");
        let ety_suffix = ety_suffix.as_str();
        let ety_suffix = self.string_pool.get_or_intern(ety_suffix);
        Some(RawEtyTemplate {
            terms: Box::new([ety_prefix, ety_suffix]),
            langs: Box::new([lang_index; 2]),
            mode: EtyMode::Confix,
            head: 0, // no true head here, arbitrarily take first
        })
    }

    fn process_compound_kind_json_template(
        &mut self,
        args: &Value,
        mode: EtyMode,
        lang: &str,
    ) -> Option<RawEtyTemplate> {
        let term_lang = args.get_valid_str("1")?;
        (term_lang == lang).then_some(())?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;

        let mut n = 2;
        let mut ety_terms = vec![];
        let mut ety_langs = vec![];
        while let Some(ety_term) = args.get_valid_str(n.to_string().as_str()) {
            if let Some(ety_lang) = args.get_valid_str(format!("lang{n}").as_str()) {
                let ety_lang_index = LANG_CODE2NAME.get_index(ety_lang)?;
                let ety_term = self.string_pool.get_or_intern(ety_term);
                ety_terms.push(ety_term);
                ety_langs.push(ety_lang_index);
            } else {
                let ety_term = self.string_pool.get_or_intern(ety_term);
                ety_terms.push(ety_term);
                ety_langs.push(lang_index);
            }
            n += 1;
        }
        if !ety_terms.is_empty() {
            return Some(RawEtyTemplate {
                terms: ety_terms.into_boxed_slice(),
                langs: ety_langs.into_boxed_slice(),
                mode,
                head: 0, // no true head here, arbitrarily take first
            });
        }
        None
    }

    fn process_json_ety_template(
        &mut self,
        template: &Value,
        lang: &str,
    ) -> Option<RawEtyTemplate> {
        let name = template.get_valid_str("name")?;
        let ety_mode = EtyMode::from_str(name).ok()?;
        let args = template.get("args")?;

        match ety_mode.template_kind() {
            TemplateKind::Derived => self.process_derived_kind_json_template(args, ety_mode, lang),
            TemplateKind::Abbreviation => {
                self.process_abbrev_kind_json_template(args, ety_mode, lang)
            }
            TemplateKind::Compound => match ety_mode {
                EtyMode::Prefix => self.process_prefix_json_template(args, lang),
                EtyMode::Suffix => self.process_suffix_json_template(args, lang),
                EtyMode::Circumfix => self.process_circumfix_json_template(args, lang),
                EtyMode::Infix => self.process_infix_json_template(args, lang),
                EtyMode::Confix => self.process_confix_json_template(args, lang),
                _ => self.process_compound_kind_json_template(args, ety_mode, lang),
            },
            _ => None,
        }
    }

    fn process_json_ety(&mut self, json_item: &Value, lang: &str) -> Option<RawEtymology> {
        let mut raw_ety_templates = vec![];
        if let Some(templates) = json_item.get_array("etymology_templates") {
            raw_ety_templates.reserve(templates.len());
            for template in templates {
                if let Some(raw_ety_template) = self.process_json_ety_template(template, lang) {
                    raw_ety_templates.push(raw_ety_template);
                }
            }
        }

        if !raw_ety_templates.is_empty() {
            return Some(raw_ety_templates.into());
        }

        // if no ety section or no templates, as a fallback we see if term
        // is listed as a "form_of" (item.senses[0].form_of[0].word)
        // or "alt_of" (item.senses[0].alt_of[0].word) another term.
        // e.g. "happenin'" is listed as an alt_of of "happening".
        let alt_term = json_item
            .get_array("senses")
            .and_then(|senses| senses.get(0))
            .and_then(|sense| {
                sense
                    .get_array("alt_of")
                    .or_else(|| sense.get_array("form_of"))
            })
            .and_then(|alt_list| alt_list.get(0))
            .and_then(|alt_obj| alt_obj.get_str("word"))
            .map(|alt_term| self.string_pool.get_or_intern(alt_term))?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let raw_ety_template = RawEtyTemplate::new(lang_index, alt_term, EtyMode::Form);
        raw_ety_templates.push(raw_ety_template);
        Some(raw_ety_templates.into())
    }

    // cf. https://en.wiktionary.org/wiki/Template:root. For now we skip
    // attempting to deal with multiple roots listed in a root template or
    // multiple root templates being listed. In both cases we just take the
    // first root term seen. If we discover it is common, we will handle it.
    fn process_json_root(&mut self, json_item: &Value, lang: &str) -> Option<RawRoot> {
        if let Some(templates) = json_item.get_array("etymology_templates") {
            for template in templates {
                if let Some(name) = template.get_valid_str("name")
                    && name == "root"
                    && let Some(args) = template.get("args")
                    && let Some(raw_root) = self.process_json_root_template(args, lang)
                {
                   return Some(raw_root);
                }
            }
        }

        // if no {root} found in ety section, look for a category of the form
        // e.g. "English terms derived from the Proto-Indo-European root *dʰeh₁-"
        // or "English terms derived from the Proto-Indo-European root *bʰel- (shiny)"
        if let Some(categories) = json_item.get_array("categories") {
            for category in categories.iter().filter_map(|c| c.as_str()) {
                if let Some(raw_root) = self.process_json_root_category(category, lang) {
                    return Some(raw_root);
                }
            }
        }

        None
    }

    fn process_json_root_template(&mut self, args: &Value, lang: &str) -> Option<RawRoot> {
        let term_lang = args.get_valid_str("1")?;
        (term_lang == lang).then_some(())?;
        let root_lang = args.get_valid_str("2")?;
        let root_lang_index = LANG_CODE2NAME.get_index(root_lang)?;
        let mut root_term = args.get_valid_str("3")?;
        // we don't deal with multi-roots for now:
        args.get_valid_str("4").is_none().then_some(())?;

        let mut root_sense_id = "";
        // Sometimes a root's senseid is given in parentheses after the term in
        // the 3 arg slot, see e.g. https://en.wiktionary.org/wiki/blaze.
        if let Some(right_paren_idx) = root_term.rfind(')')
            && let Some(left_paren_idx) = root_term.rfind(" (")
        {
            root_sense_id = &root_term[left_paren_idx + 2..right_paren_idx];
            root_term = &root_term[..left_paren_idx];
        } else if let Some(sense_id) = args.get_valid_str("id") {
            root_sense_id = sense_id;
        }
        let root_sense_id =
            (!root_sense_id.is_empty()).then_some(self.string_pool.get_or_intern(root_sense_id));
        Some(RawRoot {
            lang: root_lang_index,
            term: self.string_pool.get_or_intern(root_term),
            sense_id: root_sense_id,
        })
    }

    fn process_json_root_category(&mut self, category: &str, lang: &str) -> Option<RawRoot> {
        lazy_static! {
            static ref ROOT_CAT: Regex =
                Regex::new(r"^(.+) terms derived from the (.+) root \*([^ ]+)(?: \((.+)\))?$")
                    .unwrap();
        }
        let caps = ROOT_CAT.captures(category)?;
        let cat_term_lang_name = caps.get(1).map(|m| m.as_str())?;
        let &cat_term_lang = LANG_NAME2CODE.get(cat_term_lang_name)?;
        (cat_term_lang == lang).then_some(())?;
        let cat_root_lang_name = caps.get(2).map(|m| m.as_str())?;
        let &cat_root_lang = LANG_NAME2CODE.get(cat_root_lang_name)?;
        let cat_root_lang_index = LANG_CODE2NAME.get_index(cat_root_lang)?;
        let cat_root_term = caps.get(3).map(|m| m.as_str())?;

        let cat_root_sense_id = caps
            .get(4)
            .map(|cap| self.string_pool.get_or_intern(cap.as_str()));
        Some(RawRoot {
            lang: cat_root_lang_index,
            term: self.string_pool.get_or_intern(cat_root_term),
            sense_id: cat_root_sense_id,
        })
    }

    fn process_json_descendants(&mut self, json_item: &Value) -> Option<RawDescendants> {
        let json_descendants = json_item.get_array("descendants")?;
        let mut descendants: Vec<RawDescLine> = vec![];
        for desc_line in json_descendants {
            let raw_desc_line = self.process_json_desc_line(desc_line)?;
            descendants.push(raw_desc_line);
        }
        (!descendants.is_empty()).then_some(())?;
        Some(descendants.into())
    }

    fn process_json_desc_line(&mut self, desc_line: &Value) -> Option<RawDescLine> {
        let depth = desc_line.get_u8("depth")?;
        let templates = desc_line.get_array("templates")?;

        if templates.is_empty()
            && let Some(text) = desc_line.get_valid_str("text") 
        {
            let text = self.string_pool.get_or_intern(text);
            let kind = RawDescLineKind::BareText { text };
            return Some(RawDescLine { depth, kind });
        }
        if templates.len() == 1
            && let Some(template) = templates.get(0)
            && let Some(name) = template.get_valid_str("name")
            && matches!(name, "desc" | "descendant")
            && let Some(args) = template.get("args")
            && let Some(lang) = args.get_valid_str("1")
            && let Some(lang_index) = LANG_CODE2NAME.get_index(lang)
            && args.get_valid_str("2").is_none()
            && args.get_valid_str("alt").is_none()
        {
            let kind = RawDescLineKind::BareLang { lang: lang_index };
            return Some(RawDescLine{ depth, kind });
        }
        let is_derivation = desc_line.get_array("tags").map_or(false, |tags| {
            tags.iter().any(|tag| tag.as_str() == Some("derived"))
        });
        let mut lang = 0;
        let (mut langs, mut terms, mut modes) = (HashSet::new(), vec![], vec![]);
        for template in templates {
            if let Some((template_lang, template_terms, template_modes)) =
                self.process_json_desc_line_template(template, is_derivation)
            {
                lang = template_lang;
                langs.insert(template_lang);
                terms.extend(template_terms);
                modes.extend(template_modes);
            }
        }
        if langs.len() == 1 && !terms.is_empty() && terms.len() == modes.len() {
            let terms = terms.into_boxed_slice();
            let modes = modes.into_boxed_slice();
            let desc = RawDesc { lang, terms, modes };
            let kind = RawDescLineKind::Desc { desc };
            return Some(RawDescLine { depth, kind });
        }
        Some(RawDescLine {
            depth,
            kind: RawDescLineKind::Other,
        })
    }

    fn process_json_desc_line_template(
        &mut self,
        template: &Value,
        is_derivation: bool,
    ) -> Option<(usize, Vec<SymbolU32>, Vec<EtyMode>)> {
        let name = template.get_valid_str("name")?;
        let args = template.get("args")?;
        match name {
            "desc" | "descendant" => self.process_json_desc_line_desc_template(args),
            "l" | "link" => self.process_json_desc_line_l_template(args, is_derivation),
            "desctree" | "descendants tree" => self.process_json_desc_line_desctree_template(args),
            _ => None,
        }
    }

    // cf. https://en.wiktionary.org/wiki/Template:descendant
    fn process_json_desc_line_desc_template(
        &mut self,
        args: &Value,
    ) -> Option<(usize, Vec<SymbolU32>, Vec<EtyMode>)> {
        let lang = args.get_valid_str("1")?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;

        let (mut terms, mut modes) = (vec![], vec![]);
        // Confusingly, "2" corresponds to the first term and "alt" to its alt,
        // while "3" corresponds to the second term, and "alt2" to its alt, etc.
        let mut n = 1;
        let mut n_str = String::from("2");
        let mut n_alt_str = String::from("alt");
        while let Some(term) = args
            .get_valid_str(n_str.as_str())
            .or_else(|| args.get_valid_str(n_alt_str.as_str()))
            .map(|term| self.string_pool.get_or_intern(term))
        {
            terms.push(term);
            let mode = get_desc_mode(args, n);
            modes.push(mode);
            n += 1;
            n_str = (n + 1).to_string();
            n_alt_str = format!("alt{n}");
        }
        Some((lang_index, terms, modes))
    }

    // cf. https://en.wiktionary.org/wiki/Template:link
    fn process_json_desc_line_l_template(
        &mut self,
        args: &Value,
        is_derivation: bool,
    ) -> Option<(usize, Vec<SymbolU32>, Vec<EtyMode>)> {
        let lang = args.get_valid_str("1")?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let term = args
            .get_valid_str("2")
            .or_else(|| args.get_valid_str("3"))
            .map(|term| self.string_pool.get_or_intern(term))?;
        // There is a bit of confusion here in the nominal similarity of these
        // two modes. It is wiktionary's fault for defaulting to "derived" for
        // "unspecified etymological relationship". We are merely following this
        // tradition in this case, although some finer-grained inference could
        // be implemented in the future (probably most {{l}} templates in
        // descendants sections actually are indicating inheritance, unless they
        // are preceded by a {{desc}} on the same line that indicates some other
        // relationship). For wiktionary ety sections, there is ongoing effort
        // to replace most {{der}} templates with {{inh}} or {{bor}}.
        let mode = if is_derivation {
            EtyMode::MorphologicalDerivation
        } else {
            EtyMode::Derived
        };
        Some((lang_index, vec![term], vec![mode]))
    }

    // cf. https://en.wiktionary.org/wiki/Template:descendants_tree While
    // {{desctree}} docs say it supports all {{desc}} args, I've never seen one
    // that's more than just e.g. {{desctree|gmw-pro|*fuhs}}. (Importantly, both
    // "1" AND "2" are required here.) So we just handle this simple case of one
    // descendant generating the tree, until we find that listing multiple has
    // any currency (how would that even work?).
    fn process_json_desc_line_desctree_template(
        &mut self,
        args: &Value,
    ) -> Option<(usize, Vec<SymbolU32>, Vec<EtyMode>)> {
        let lang = args.get_valid_str("1")?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let term = args
            .get_valid_str("2")
            .map(|term| self.string_pool.get_or_intern(term))?;
        // It's conceivable that another mode could be specified by template arg
        let mode = get_desc_mode(args, 1);
        Some((lang_index, vec![term], vec![mode]))
    }

    fn process_redirect(&mut self, items: &mut Items, json_item: &Value) {
        // cf. https://github.com/tatuylonen/wiktextract/blob/master/wiktwords
        static IGNORED_REDIRECTS: Set<&'static str> = phf_set! {
            "Index", "Help", "MediaWiki", "Citations", "Concordance", "Rhymes",
            "Thread", "Summary", "File", "Transwiki", "Category", "Appendix",
            "Wiktionary", "Thesaurus", "Module", "Template"
        };
        if let Some(from_title) = json_item.get_valid_str("title")
            && let Some(to_title) = json_item.get_valid_str("redirect")
        {
            for title in [from_title, to_title] {
                if let Some(colon) = title.find(':')
                    && let Some(namespace) = title.get(..colon)
                    && IGNORED_REDIRECTS.contains(namespace)
                {
                    return;
                }
            }
            // e.g. Reconstruction:Proto-Germanic/pīpǭ
            if let Some(from_title) = self.process_reconstruction_title(from_title) {
                // e.g. "Reconstruction:Proto-West Germanic/pīpā"
                if let Some(to_title) = self.process_reconstruction_title(to_title) {
                    items.redirects.reconstruction.insert(from_title, to_title);
                }
                return;
            }
            // otherwise, this is a simple term-to-term redirect
            let from_title = self.string_pool.get_or_intern(from_title);
            let to_title = self.string_pool.get_or_intern(to_title);
            items.redirects.regular.insert(from_title, to_title);
        }
    }

    fn process_reconstruction_title(&mut self, title: &str) -> Option<ReconstructionTitle> {
        // e.g. Reconstruction:Proto-Germanic/pīpǭ
        let title = title.strip_prefix("Reconstruction:")?;
        let slash = title.find('/')?;
        let language = &title.get(..slash)?;
        let term = title.get(slash + 1..)?;
        let language_index = LANG_NAME2CODE.get_index(language)?;

        Some(ReconstructionTitle {
            language: language_index,
            term: self.string_pool.get_or_intern(term),
        })
    }

    fn process_json_item(
        &mut self,
        items: &mut Items,
        json_item: &Value,
        line_number: usize,
    ) -> Result<()> {
        // Some wiktionary pages are redirects. These are actually used somewhat
        // heavily, so we need to take them into account
        // https://github.com/tatuylonen/wiktextract#format-of-extracted-redirects
        if json_item.contains_key("redirect") {
            self.process_redirect(items, json_item);
            return Ok(());
        }
        if let Some(page_title) = json_item.get_valid_str("word")
            && let Some(pos) = json_item.get_valid_str("pos")
            && let Some(pos_index) = POS.get_index(pos)
            && !should_ignore_term(page_title, pos)
            && let Some(lang) = json_item.get_valid_str("lang_code")
            && let Some(lang_index) = LANG_CODE2NAME.get_index(lang)
        {
            let term = get_term_canonical_form(json_item).unwrap_or(page_title);
            let term = self.string_pool.get_or_intern(term);
            let page_title = Some(self.string_pool.get_or_intern(page_title));
            // if term-lang combo has multiple ety's, then 'etymology_number' is
            // present with range 1,2,... Otherwise, this key is missing.
            let ety_num = json_item.get_u8("etymology_number");
            // 'senses' key should always be present with non-empty value, but glosses
            // may be missing or empty.
            let gloss = json_item
                .get_array("senses")
                .and_then(|senses| senses.get(0))
                .and_then(|sense| sense.get_array("glosses"))
                .and_then(|glosses| glosses.get(0))
                .and_then(|gloss| gloss.as_str())
                .and_then(|s| (!s.is_empty()).then(|| self.string_pool.get_or_intern(s)));

            let raw_root = self.process_json_root(json_item, lang);
            let raw_etymology = self.process_json_ety(json_item, lang);
            let raw_descendants = self.process_json_descendants(json_item);

            let item = Item {
                line: Some(line_number),
                is_imputed: false,
                // $$ This will not catch all reconstructed terms, since some terms
                // in attested languages are reconstructed. Some better inference
                // should be done based on "*" prefix for terms. 
                is_reconstructed: is_reconstructed_lang(lang_index),
                i: items.n,
                lang: lang_index,
                term,
                page_title,
                ety_num,
                pos: Some(pos_index),
                gloss,
                gloss_num: 0, // temp value to be changed if need be in add()
                raw_etymology,
                raw_root,
                raw_descendants,
            };
            if let Some(item) = items.add_to_term_map(item)? {
                items.line_map.insert(line_number, item);
            }
        }
        Ok(())
    }

    fn process_json_items(&mut self, path: &Path) -> Result<Items> {
        let mut items = Items::default();
        for (line_number, mut line) in wiktextract_lines(path)?.enumerate() {
            let json_item = to_borrowed_value(&mut line)?;
            self.process_json_item(&mut items, &json_item, line_number)?;
        }
        Ok(items)
    }
}

fn wiktextract_lines(path: &Path) -> Result<impl Iterator<Item = Vec<u8>>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let is_gz_compressed = path.extension().is_some_and(|ext| ext == "gz");
    let uncompressed: Box<dyn Read> = if is_gz_compressed {
        Box::new(GzDecoder::new(reader))
    } else {
        Box::new(reader)
    };
    let reader = BufReader::new(uncompressed);
    let lines = ByteLines::new(reader);
    // We use into_iter() here and thereby allocate a Vec<u8> for each line, so
    // that we have the convenience of returning an iterator. These allocations
    // are not particularly a bottleneck relative to other things so it's fine.
    Ok(lines.into_iter().filter_map(Result::ok))
}

fn clean_ety_term(term: &str) -> &str {
    // Reconstructed terms (e.g. PIE) are supposed to start with "*" when cited
    // in etymologies but their entry titles (and hence wiktextract "word"
    // field) do not. This is done by
    // https://en.wiktionary.org/wiki/Module:links. Sometimes reconstructed
    // terms are missing this *, and sometimes non-reconstructed terms start
    // with * incorrectly. So we strip the * in every case. This will break
    // terms that actually start with *, but there are almost none of these, and
    // none of them are particularly relevant for our purposes AFAIK.
    term.strip_prefix('*').unwrap_or(term)
}

fn remove_punctuation(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_ascii_punctuation())
        .collect::<String>()
}

fn should_ignore_term(term: &str, pos: &str) -> bool {
    // This function needs revisiting depending on results.

    // We would generally like to ignore phrases, and potentially other things.
    //  Barring all phrases may be both too strict and not strict enough. Too
    // strict because certain phrases may be relevant for etymologies (i.e. a
    // phrase became one word in a daughter language). Not strict enough because
    // many phrases are categorized as other pos. See e.g.
    // https://en.wiktionary.org/wiki/this,_that,_or_the_other. Ignoring terms
    // that contain any ascii punctuation is too strict, as this would ingore
    // e.g. affixes with -. Ignoring terms with any ascii whitespace is too
    // strict as well, as this would ignore e.g. circumfixes (e.g. "ver- -en").
    if pos.contains("phrase") || term.contains(|c: char| c == ',') {
        return true;
    }
    false
}

// We look for a canonical form, otherwise we take the "word" field.
// See notes.md for motivation.
fn get_term_canonical_form<'a>(json_item: &'a Value) -> Option<&'a str> {
    let forms = json_item.get_array("forms")?;
    let mut f = 0;
    while let Some(form) = forms.get(f) {
        if let Some(tags) = form.get_array("tags") {
            let mut t = 0;
            while let Some(tag) = tags.get(t).as_str() {
                if tag == "canonical" {
                    // There are some
                    if let Some(term) = form.get_valid_str("form") {
                        return Some(term);
                    }
                }
                t += 1;
            }
        }
        f += 1;
    }
    None
}

fn etylang2lang(lang: usize) -> usize {
    // If lang is an etymology-only language, we will not find any entries
    // for it in Items lang map, since such a language definitionally does
    // not have any entries itself. So we look for the actual lang that the
    // ety lang is associated with.
    LANG_CODE2NAME
        .get_index_key(lang)
        .and_then(|code| {
            LANG_ETYCODE2CODE
                .get(code)
                .and_then(|code| LANG_CODE2NAME.get_index(code))
        })
        .unwrap_or(lang)
}

fn is_reconstructed_lang(lang: usize) -> bool {
    LANG_CODE2NAME
        .get_index_key(lang)
        .map_or(false, |code| LANG_RECONSTRUCTED.contains(code))
}

fn get_desc_mode(args: &Value, n: usize) -> EtyMode {
    // what about "der"?
    const MODES: [&str; 7] = ["bor", "lbor", "slb", "clq", "pclq", "sml", "translit"];
    const DEFAULT: EtyMode = EtyMode::Inherited;
    for mode in MODES {
        let mode_n = format!("{mode}{n}");
        if args.contains_key(mode) || args.contains_key(mode_n.as_str()) {
            return EtyMode::from_str(mode).ok().unwrap_or(DEFAULT);
        }
    }
    DEFAULT
}

pub(crate) struct ProcessedData {
    string_pool: StringPool,
    items: Items,
    ety_graph: EtyGraph,
}

/// # Errors
///
/// Will return `Err` if any unexpected issue arises parsing the wiktextract
/// data or writing to Turtle file.
pub fn wiktextract_to_turtle(wiktextract_path: &Path, turtle_path: &Path) -> Result<Instant> {
    let mut t = Instant::now();
    println!(
        "Processing raw wiktextract data from {}...",
        wiktextract_path.display()
    );
    let mut processor = RawDataProcessor::new()?;
    let items = processor.process_json_items(wiktextract_path)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    t = Instant::now();
    println!("Generating embeddings for ambiguous terms...");
    let embeddings = items.generate_embeddings(wiktextract_path)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    t = Instant::now();
    println!("Generating ety graph...");
    let ety_graph = items.generate_ety_graph(&processor.string_pool, &embeddings)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    println!("Writing RDF to Turtle file {}...", turtle_path.display());
    t = Instant::now();
    let data = ProcessedData {
        string_pool: processor.string_pool,
        items,
        ety_graph,
    };
    write_turtle_file(&data, turtle_path)?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    t = Instant::now();
    println!("Dropping all processed data...");
    Ok(t)
}

/// # Errors
///
/// Will return `Err` if any unexpected issue arises building the Oxigraph store.
pub fn build_store(turtle_path: &Path, store_path: &Path, skip_optimizing: bool) -> Result<()> {
    let mut t = Instant::now();
    println!("Building oxigraph store {}...", store_path.display());
    let turtle = BufReader::new(File::open(turtle_path)?);
    // delete any previous oxigraph db
    if store_path.is_dir() {
        remove_dir_all(store_path)?;
    }
    let store = Store::open(store_path)?;
    store
        .bulk_loader()
        .load_graph(turtle, Turtle, DefaultGraph, None)?;
    store.flush()?;
    println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    if !skip_optimizing {
        t = Instant::now();
        println!("Optimizing oxigraph store {}...", store_path.display());
        store.optimize()?;
        store.flush()?;
        println!("Finished. Took {}.", HumanDuration(t.elapsed()));
    }
    Ok(())
}
