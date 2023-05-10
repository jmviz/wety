use crate::{
    embeddings::{Embeddings, ItemEmbedding},
    etymology_templates::EtyMode,
    gloss::Gloss,
    items::{ItemId, Items, Retrieval},
    langterm::{LangTerm, Term},
    languages::Lang,
    progress_bar,
    string_pool::StringPool,
    wiktextract_json::{WiktextractJson, WiktextractJsonItem, WiktextractJsonValidStr},
    HashSet,
};

use std::{mem, str::FromStr};

use anyhow::{Ok, Result};
use itertools::izip;
use simd_json::ValueAccess;

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawDescendants {
    pub(crate) lines: Box<[RawDescLine]>,
}

impl From<Vec<RawDescLine>> for RawDescendants {
    fn from(descendants: Vec<RawDescLine>) -> Self {
        Self {
            lines: descendants.into_boxed_slice(),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawDescLine {
    depth: u8,
    kind: RawDescLineKind,
}

#[derive(Hash, Eq, PartialEq, Debug)]
enum RawDescLineKind {
    Desc { desc: RawDesc },
    // e.g. {{desc|osp|-}}, {{desc|itc-pro|}},
    BareLang { lang: Lang },
    // i.e. line with no templates e.g. "Unsorted Formations", "with prefix -a"
    BareText { text: Gloss },
    // e.g. a line with {{PIE root see}} or some other unhandled template(s)
    // or unexpected form of above line kinds
    Other,
    // stretch goal: https://en.wiktionary.org/wiki/Template:CJKV
}

// some combination of desc, l, desctree templates that together provide one or
// more descendant lang, term, mode combos
#[derive(Hash, Eq, PartialEq, Debug)]
struct RawDesc {
    lang: Lang,
    terms: Box<[Term]>,
    modes: Box<[EtyMode]>,
}
impl WiktextractJsonItem<'_> {
    pub(crate) fn get_descendants(&self, string_pool: &mut StringPool) -> Option<RawDescendants> {
        let json_descendants = self.json.get_array("descendants")?;
        let mut descendants: Vec<RawDescLine> = vec![];
        for desc_line in json_descendants {
            let raw_desc_line = process_json_desc_line(string_pool, desc_line)?;
            descendants.push(raw_desc_line);
        }
        (!descendants.is_empty()).then_some(())?;
        Some(descendants.into())
    }
}

fn process_json_desc_line(
    string_pool: &mut StringPool,
    desc_line: &WiktextractJson,
) -> Option<RawDescLine> {
    let depth = desc_line.get_u8("depth")?;
    let templates = desc_line.get_array("templates")?;

    if templates.is_empty()
            && let Some(text) = desc_line.get_valid_str("text") 
        {
            let text = Gloss::new(string_pool, text);
            let kind = RawDescLineKind::BareText { text };
            return Some(RawDescLine { depth, kind });
        }
    if templates.len() == 1
            && let Some(template) = templates.get(0)
            && let Some(name) = template.get_valid_str("name")
            && matches!(name, "desc" | "descendant")
            && let Some(args) = template.get("args")
            && let Some(lang) = args.get_valid_str("1")
            && let Some(lang) = Lang::from_str(lang).ok()
            && args.get_valid_term("2").is_none()
            && args.get_valid_term("alt").is_none()
        {
            let kind = RawDescLineKind::BareLang { lang };
            return Some(RawDescLine{ depth, kind });
        }
    let is_derivation = desc_line.get_array("tags").map_or(false, |tags| {
        tags.iter().any(|tag| tag.as_str() == Some("derived"))
    });
    let mut lang = Lang::from_str("en").unwrap(); // dummy assignment
    let (mut langs, mut terms, mut modes) = (HashSet::default(), vec![], vec![]);
    for template in templates {
        if let Some((template_lang, template_terms, template_modes)) =
            process_json_desc_line_template(string_pool, template, is_derivation)
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
    string_pool: &mut StringPool,
    template: &WiktextractJson,
    is_derivation: bool,
) -> Option<(Lang, Vec<Term>, Vec<EtyMode>)> {
    let name = template.get_valid_str("name")?;
    let args = template.get("args")?;
    match name {
        "desc" | "descendant" => process_json_desc_line_desc_template(string_pool, args),
        "l" | "link" => process_json_desc_line_l_template(string_pool, args, is_derivation),
        "desctree" | "descendants tree" => {
            process_json_desc_line_desctree_template(string_pool, args)
        }
        _ => None,
    }
}

// cf. https://en.wiktionary.org/wiki/Template:descendant
fn process_json_desc_line_desc_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
) -> Option<(Lang, Vec<Term>, Vec<EtyMode>)> {
    let lang = args.get_valid_str("1")?;
    let lang = Lang::from_str(lang).ok()?;

    let (mut terms, mut modes) = (vec![], vec![]);
    // Confusingly, "2" corresponds to the first term and "alt" to its alt,
    // while "3" corresponds to the second term, and "alt2" to its alt, etc.
    let mut n = 1;
    let mut n_str = String::from("2");
    let mut n_alt_str = String::from("alt");
    while let Some(term) = args
        .get_valid_term(&n_str)
        .or_else(|| args.get_valid_term(&n_alt_str))
        .map(|term| Term::new(string_pool, term))
    {
        terms.push(term);
        let mode = get_desc_mode(args, n);
        modes.push(mode);
        n += 1;
        n_str = (n + 1).to_string();
        n_alt_str = format!("alt{n}");
    }
    Some((lang, terms, modes))
}

// cf. https://en.wiktionary.org/wiki/Template:link
fn process_json_desc_line_l_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    is_derivation: bool,
) -> Option<(Lang, Vec<Term>, Vec<EtyMode>)> {
    let lang = args.get_valid_str("1")?;
    let lang = Lang::from_str(lang).ok()?;
    let term = args
        .get_valid_term("2")
        .or_else(|| args.get_valid_term("3"))
        .map(|term| Term::new(string_pool, term))?;
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
    Some((lang, vec![term], vec![mode]))
}

// cf. https://en.wiktionary.org/wiki/Template:descendants_tree While
// {{desctree}} docs say it supports all {{desc}} args, I've never seen one
// that's more than just e.g. {{desctree|gmw-pro|*fuhs}}. (Importantly, both
// "1" AND "2" are required here.) So we just handle this simple case of one
// descendant generating the tree, until we find that listing multiple has
// any currency (how would that even work?).
fn process_json_desc_line_desctree_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
) -> Option<(Lang, Vec<Term>, Vec<EtyMode>)> {
    let lang = args.get_valid_str("1")?;
    let lang = Lang::from_str(lang).ok()?;
    let term = args
        .get_valid_term("2")
        .map(|term| Term::new(string_pool, term))?;
    // It's conceivable that another mode could be specified by template arg
    let mode = get_desc_mode(args, 1);
    Some((lang, vec![term], vec![mode]))
}

fn get_desc_mode(args: &WiktextractJson, n: usize) -> EtyMode {
    // what about "der"?
    const MODES: [&str; 7] = ["bor", "lbor", "slb", "clq", "pclq", "sml", "translit"];
    const DEFAULT: EtyMode = EtyMode::Inherited;
    for mode in MODES {
        let mode_n = format!("{mode}{n}");
        if args.contains_key(mode) || args.contains_key(mode_n.as_str()) {
            return mode.parse().ok().unwrap_or(DEFAULT);
        }
    }
    DEFAULT
}

struct Ancestors<T: Clone> {
    ancestors: Vec<T>,
    depths: Vec<u8>,
}
impl<T: Clone> Ancestors<T> {
    fn new(item: &T) -> Self {
        Self {
            ancestors: vec![item.clone()],
            depths: vec![0],
        }
    }

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

impl Ancestors<ItemId> {
    fn embeddings(&self, items: &Items, embeddings: &Embeddings) -> Result<Vec<ItemEmbedding>> {
        let mut item_embeddings = Vec::with_capacity(self.ancestors.len());
        for &ancestor in &self.ancestors {
            item_embeddings.push(embeddings.get(items.get(ancestor), ancestor)?);
        }
        Ok(item_embeddings)
    }
}

impl Items {
    pub(crate) fn get_desc_items_needing_embedding(
        &self,
        item: ItemId,
        raw_descendants: &RawDescendants,
    ) -> HashSet<ItemId> {
        let mut items_needing_embedding = HashSet::default();
        let mut possible_ancestors = Ancestors::new(&vec![item]);
        for line in raw_descendants.lines.iter() {
            let possible_parents = possible_ancestors.prune_and_get_parent(line.depth);
            let mut has_ambiguous_child = false;
            let mut has_imputed_child = false;
            if let RawDescLineKind::Desc { desc } = &line.kind {
                for (i, &term) in desc.terms.iter().enumerate() {
                    let desc_langterm = LangTerm::new(desc.lang, term);
                    if let Some(desc_items) = self.get_dupes(desc_langterm) {
                        if i == 0 {
                            possible_ancestors.add(desc_items, line.depth);
                        }
                        if desc_items.len() > 1 {
                            // i.e. langterm is ambiguous
                            has_ambiguous_child = true;
                            for &desc_item in desc_items {
                                items_needing_embedding.insert(desc_item);
                            }
                        }
                    } else {
                        has_imputed_child = true;
                    }
                }
                if has_ambiguous_child || has_imputed_child {
                    for possible_parent in possible_parents {
                        items_needing_embedding.insert(possible_parent);
                    }
                }
            }
        }
        items_needing_embedding
    }

    pub(crate) fn process_raw_descendants(&mut self, embeddings: &Embeddings) -> Result<()> {
        let n = self.raw_templates.desc.len();
        let pb = progress_bar(n, "Processing descendants")?;
        let raw_templates_desc = mem::take(&mut self.raw_templates.desc);
        for (item_id, desc) in raw_templates_desc {
            self.process_item_raw_descendants(embeddings, item_id, &desc)?;
            pb.inc(1);
        }

        pb.finish();
        Ok(())
    }

    pub(crate) fn process_item_raw_descendants(
        &mut self,
        embeddings: &Embeddings,
        item: ItemId,
        raw_descendants: &RawDescendants,
    ) -> Result<()> {
        let item_lang = self.get(item).lang();
        let mut ancestors = Ancestors::new(&item);
        'lines: for line in raw_descendants.lines.iter() {
            let parent = ancestors.prune_and_get_parent(line.depth);
            match &line.kind {
                RawDescLineKind::Desc { desc } => {
                    if desc.terms.is_empty() || desc.terms.len() != desc.modes.len() {
                        continue;
                    }
                    let (mut desc_items, mut confidences, mut modes) = (vec![], vec![], vec![]);
                    for (i, (&term, &mode)) in desc.terms.iter().zip(desc.modes.iter()).enumerate()
                    {
                        // Sometimes a within-language compound is listed as a
                        // descendant. See e.g. PIE men- page, where compound of
                        // men- and dʰeh₁- is listed, or PIE bʰer- page, where
                        // compound of h₂ed and bʰer- is listed. We try to skip
                        // these lines, as otherwise we would e.g. end up making
                        // a connection from bʰer- to h₂éd, which will
                        // completely screw up both of their total descendants
                        // trees. $$ In general, we may need to end up doing
                        // much smarter processing of descendants sections if
                        // there is more such variation I am unaware of
                        // (probable?).
                        if desc.terms.len() > 1 && desc.lang == item_lang {
                            continue 'lines;
                        }
                        let langterm = LangTerm::new(desc.lang, term);
                        let Retrieval {
                            item_id: desc_item,
                            confidence,
                        } = self.get_or_impute_item(
                            embeddings,
                            &ancestors.embeddings(self, embeddings)?,
                            item,
                            langterm,
                        )?;
                        // Only use the first term in a multi-term desc line as
                        // the ancestor for any deeper-nested lines below it.
                        if i == 0 {
                            ancestors.add(&desc_item, line.depth);
                        }
                        desc_items.push(desc_item);
                        confidences.push(confidence);
                        modes.push(mode);
                    }
                    for (desc_item, confidence, mode) in izip!(desc_items, confidences, modes) {
                        self.graph
                            .add_ety(desc_item, mode, Some(0), &[parent], &[confidence]);
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
        Ok(())
    }
}
