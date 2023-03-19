use crate::{
    embeddings::{Embeddings, ItemEmbedding},
    ety_graph::EtyGraph,
    etymology_templates::EtyMode,
    lang_phf::LANG_CODE2NAME,
    phf_ext::OrderedSetExt,
    pos_phf::POS,
    progress_bar,
    raw_items::{RawItem, RawItems},
    string_pool::Symbol,
    wiktextract_json::{WiktextractJson, WiktextractJsonAccess},
    RawDataProcessor,
};

use std::{rc::Rc, str::FromStr};

use anyhow::{Ok, Result};
use hashbrown::HashSet;
use simd_json::ValueAccess;

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawDescendants {
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
    BareText { text: Symbol },
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
    terms: Box<[Symbol]>,
    modes: Box<[EtyMode]>,
}

impl RawDataProcessor {
    pub(crate) fn process_json_descendants(
        &mut self,
        json_item: &WiktextractJson,
    ) -> Option<RawDescendants> {
        let json_descendants = json_item.get_array("descendants")?;
        let mut descendants: Vec<RawDescLine> = vec![];
        for desc_line in json_descendants {
            let raw_desc_line = self.process_json_desc_line(desc_line)?;
            descendants.push(raw_desc_line);
        }
        (!descendants.is_empty()).then_some(())?;
        Some(descendants.into())
    }

    fn process_json_desc_line(&mut self, desc_line: &WiktextractJson) -> Option<RawDescLine> {
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
        template: &WiktextractJson,
        is_derivation: bool,
    ) -> Option<(usize, Vec<Symbol>, Vec<EtyMode>)> {
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
        args: &WiktextractJson,
    ) -> Option<(usize, Vec<Symbol>, Vec<EtyMode>)> {
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
        args: &WiktextractJson,
        is_derivation: bool,
    ) -> Option<(usize, Vec<Symbol>, Vec<EtyMode>)> {
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
        args: &WiktextractJson,
    ) -> Option<(usize, Vec<Symbol>, Vec<EtyMode>)> {
        let lang = args.get_valid_str("1")?;
        let lang_index = LANG_CODE2NAME.get_index(lang)?;
        let term = args
            .get_valid_str("2")
            .map(|term| self.string_pool.get_or_intern(term))?;
        // It's conceivable that another mode could be specified by template arg
        let mode = get_desc_mode(args, 1);
        Some((lang_index, vec![term], vec![mode]))
    }
}

fn get_desc_mode(args: &WiktextractJson, n: usize) -> EtyMode {
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

struct Ancestors<T: Clone> {
    ancestors: Vec<T>,
    depths: Vec<u8>,
    // progenitor: Ancestor<T>,
}
impl<T: Clone> Ancestors<T> {
    fn new(item: &T) -> Self {
        Self {
            ancestors: vec![item.clone()],
            depths: vec![0],
        }
    }
    // fn progenitor(&self) -> T {
    //     self.ancestors
    //         .get(0)
    //         .map(T::clone)
    //         .expect("ancestors always contains at least the progenitor")
    // }
    fn remove_last(&mut self) {
        self.ancestors.pop();
        self.depths.pop();
    }
    fn prune(&mut self, depth: u8) {
        while let Some(&ancestor_depth) = self.depths.last()
            && depth <= ancestor_depth
            && self.depths.len() > 1 // ensure at least progenitor remains
        {
            self.remove_last();
        }
    }
    fn prune_and_get_parent(&mut self, depth: u8) -> T {
        self.prune(depth);
        self.ancestors
            .last()
            .map(T::clone)
            .expect("ancestors always contains at least the progenitor")
    }
    fn add(&mut self, item: &T, depth: u8) {
        self.ancestors.push(item.clone());
        self.depths.push(depth);
    }
}

impl Ancestors<Rc<RawItem>> {
    fn embeddings<'a>(&self, embeddings: &'a Embeddings) -> Vec<ItemEmbedding<'a>> {
        self.ancestors.iter().map(|a| embeddings.get(a)).collect()
    }
}

impl RawItems {
    pub(crate) fn get_desc_items_needing_embedding(
        &self,
        item: &Rc<RawItem>,
        raw_descendants: &RawDescendants,
    ) -> HashSet<Rc<RawItem>> {
        let mut items_needing_embedding = HashSet::new();
        let mut possible_ancestors = Ancestors::new(&vec![item.clone()]);
        for line in raw_descendants.lines.iter() {
            let possible_parents = possible_ancestors.prune_and_get_parent(line.depth);
            let mut has_ambiguous_child = false;
            let mut has_imputed_child = false;
            if let RawDescLineKind::Desc { desc } = &line.kind {
                for (i, &term) in desc.terms.iter().enumerate() {
                    if let Some(desc_items) = self.get_all_lang_term_items(desc.lang, term) {
                        if i == 0 {
                            possible_ancestors.add(&desc_items, line.depth);
                        }
                        if desc_items.len() > 1 {
                            // i.e. (lang, term) is ambiguous
                            has_ambiguous_child = true;
                            for desc_item in &desc_items {
                                items_needing_embedding.insert(Rc::clone(desc_item));
                            }
                        }
                    } else {
                        has_imputed_child = true;
                    }
                }
                if has_ambiguous_child || has_imputed_child {
                    for possible_parent in possible_parents {
                        items_needing_embedding.insert(possible_parent.clone());
                    }
                }
            }
        }
        items_needing_embedding
    }

    pub(crate) fn process_raw_descendants(
        &self,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
    ) -> Result<()> {
        let pb = progress_bar(self.n, "Processing descendants")?;
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            self.process_item_raw_descendants(embeddings, ety_graph, item);
                            pb.inc(1);
                        }
                    }
                }
            }
        }
        pb.finish();
        Ok(())
    }

    pub(crate) fn process_item_raw_descendants(
        &self,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
        item: &Rc<RawItem>,
    ) {
        if item.raw_descendants.is_none() {
            return;
        }
        let mut ancestors = Ancestors::new(item);
        'outer: for line in item.raw_descendants.as_ref().unwrap().lines.iter() {
            let parent = ancestors.prune_and_get_parent(line.depth);
            match &line.kind {
                RawDescLineKind::Desc { desc } => {
                    if desc.terms.is_empty() || desc.terms.len() != desc.modes.len() {
                        continue;
                    }
                    let lang = desc.lang;
                    let (mut desc_items, mut modes) = (vec![], vec![]);
                    for (i, (&term, &mode)) in desc.terms.iter().zip(desc.modes.iter()).enumerate()
                    {
                        let desc_item = self
                            .get_disambiguated_item(
                                embeddings,
                                &ancestors.embeddings(embeddings),
                                lang,
                                term,
                            )
                            .or_else(|| ety_graph.imputed_items.get(lang, term));
                        // Borrow checker complains when I use map_or_else
                        // instead of map then unwrap_or_else. But if I
                        // chain these last two then clippy::pedantic
                        // complains...
                        let desc_item = desc_item.unwrap_or_else(|| {
                            let n = self.n + ety_graph.imputed_items.n;
                            let imputed_item = Rc::from(RawItem::new_imputed(n, lang, term, None));
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
                        // Only use the first term in a multi-term desc line as
                        // the ancestor for any deeper-nested lines below it.
                        if i == 0 {
                            ancestors.add(&desc_item, line.depth);
                        }
                        desc_items.push(desc_item);
                        modes.push(mode);
                    }
                    for (desc_item, mode) in desc_items.iter().zip(modes) {
                        ety_graph.add_ety(desc_item, mode, 0, &[Rc::clone(&parent)]);
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
}
