use crate::{
    HashSet,
    embeddings::{self, Embeddings, ItemEmbedding},
    items::Items,
    resolve::{Citation, Relation, Resolution},
    progress_bar,
    wiktextract_json::{WiktextractJson, WiktextractJsonItem, WiktextractJsonValidStr},
};
use wety_core::{
    etymology_templates::EtyMode,
    gloss::Gloss,
    items::ItemId,
    langterm::{LangTerm, Term},
    languages::Lang,
    string_pool::StringPool,
};

use std::{mem, str::FromStr};

use anyhow::{Ok, Result};
use itertools::izip;
use simd_json::ValueAccess;

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawDescendants {
    pub(crate) lines: Box<[RawDescLine]>,
}

impl RawDescendants {
    // the langterms the section lists
    pub(crate) fn langterms(&self) -> impl Iterator<Item = LangTerm> + '_ {
        self.lines.iter().flat_map(|line| match &line.kind {
            RawDescLineKind::Desc { desc } => desc
                .terms
                .iter()
                .map(|&term| LangTerm::new(desc.lang, term))
                .collect::<Vec<_>>(),
            _ => vec![],
        })
    }
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
        flatten_desc_nodes(string_pool, json_descendants, 1, &mut descendants);
        (!descendants.is_empty()).then_some(())?;
        Some(descendants.into())
    }
}

// Wiktextract gives a Descendants section as a tree with one node per term.
// A line naming several terms, e.g. "{{desc|enm|lam|lame}}", becomes sibling
// nodes that each carry a copy of the line's sub-lines. We flatten the tree
// back into lines with depths, regrouping such siblings into one multi-term
// line so that the shared sub-lines hang only from its first term.
fn flatten_desc_nodes(
    string_pool: &mut StringPool,
    nodes: &[WiktextractJson],
    depth: u8,
    lines: &mut Vec<RawDescLine>,
) {
    let mut i = 0;
    while i < nodes.len() {
        let mut j = i + 1;
        while j < nodes.len() && same_desc_line(&nodes[i], &nodes[j]) {
            j += 1;
        }
        lines.push(process_json_desc_line(string_pool, &nodes[i..j], depth));
        if let Some(children) = nodes[i].get_array("descendants") {
            flatten_desc_nodes(string_pool, children, depth.saturating_add(1), lines);
        }
        i = j;
    }
}

// Only siblings with identical, non-empty sub-lines are taken to be one line:
// childless siblings in the same language are as often separate lines (e.g.
// the list of derivations on a PIE root page).
fn same_desc_line(a: &WiktextractJson, b: &WiktextractJson) -> bool {
    a.get_valid_term("word").is_some()
        && b.get_valid_term("word").is_some()
        && a.get_str("lang_code") == b.get_str("lang_code")
        && a.get_array("descendants").is_some_and(|c| !c.is_empty())
        && a.get("descendants") == b.get("descendants")
}

fn process_json_desc_line(
    string_pool: &mut StringPool,
    nodes: &[WiktextractJson],
    depth: u8,
) -> RawDescLine {
    let first = &nodes[0];
    let lang = first
        .get_valid_str("lang_code")
        .and_then(|code| Lang::from_str(code).ok());
    let kind = match (lang, first.get_valid_term("word")) {
        (Some(lang), Some(_)) => {
            let (terms, modes): (Vec<_>, Vec<_>) = nodes
                .iter()
                .filter_map(|node| {
                    let term = Term::new(string_pool, node.get_valid_term("word")?);
                    Some((term, get_desc_mode(node)))
                })
                .unzip();
            let desc = RawDesc {
                lang,
                terms: terms.into_boxed_slice(),
                modes: modes.into_boxed_slice(),
            };
            RawDescLineKind::Desc { desc }
        }
        // e.g. {{desc|osp|-}}, or a family heading like "East Slavic"
        (Some(lang), None) => RawDescLineKind::BareLang { lang },
        // headings without a language, e.g. "Unsorted formations",
        // "Borrowings"; Wiktextract gives these lang_code "unknown"
        (None, None) => match first.get_valid_str("lang") {
            Some(text) => RawDescLineKind::BareText {
                text: Gloss::new(string_pool, text),
            },
            None => RawDescLineKind::Other,
        },
        (None, Some(_)) => RawDescLineKind::Other,
    };
    RawDescLine { depth, kind }
}

// The {{desc}} flags (bor=1, clq=1, der=1, ...) survive only as tags.
fn get_desc_mode(node: &WiktextractJson) -> EtyMode {
    // most specific first
    const MODES: [(&str, EtyMode); 9] = [
        ("partial calque", EtyMode::PartialCalque),
        ("calque", EtyMode::Calque),
        ("semantic loan", EtyMode::SemanticLoan),
        ("semi-learned borrowing", EtyMode::SemiLearnedBorrowing),
        ("learned borrowing", EtyMode::LearnedBorrowing),
        ("borrowed", EtyMode::Borrowed),
        ("transliteration", EtyMode::Transliteration),
        ("derived", EtyMode::MorphologicalDerivation),
        ("reshaped by analogy or addition of morphemes", EtyMode::Derived),
    ];
    let tags: Vec<&str> = ["tags", "raw_tags"]
        .into_iter()
        .filter_map(|key| node.get_array(key))
        .flatten()
        .filter_map(|tag| tag.as_str())
        .collect();
    MODES
        .iter()
        .find(|(tag, _)| tags.contains(tag))
        .map_or(EtyMode::Inherited, |&(_, mode)| mode)
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
            && self.depths.len() > 1
        // ensure at least progenitor remains
        {
            self.remove_last();
        }
    }

    fn prune_and_get_parent(&mut self, depth: u8) -> T {
        self.prune(depth);
        self.ancestors
            .last()
            .cloned()
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
        for line in &*raw_descendants.lines {
            let possible_parents = possible_ancestors.prune_and_get_parent(line.depth);
            let mut has_ambiguous_child = false;
            let mut has_imputed_child = false;
            if let RawDescLineKind::Desc { desc } = &line.kind {
                for (i, &term) in desc.terms.iter().enumerate() {
                    let desc_langterm = LangTerm::new(desc.lang, term);
                    if let Some(desc_items) = self.candidates(desc_langterm) {
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

    pub(crate) fn process_raw_descendants(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
    ) -> Result<()> {
        let n = self.raw_templates.desc.len();
        let pb = progress_bar(n, "Processing descendants")?;
        // in id order, so that the result does not depend on hash-map order
        let mut raw_templates_desc: Vec<_> =
            mem::take(&mut self.raw_templates.desc).into_iter().collect();
        raw_templates_desc.sort_unstable_by_key(|(item_id, _)| *item_id);
        for (item_id, desc) in raw_templates_desc {
            self.process_item_raw_descendants(string_pool, embeddings, item_id, &desc)?;
            pb.inc(1);
        }

        pb.finish();
        Ok(())
    }

    pub(crate) fn process_item_raw_descendants(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        item: ItemId,
        raw_descendants: &RawDescendants,
    ) -> Result<()> {
        let item_lang = self.get(item).lang();
        let mut ancestors = Ancestors::new(&item);
        'lines: for line in &*raw_descendants.lines {
            let parent = ancestors.prune_and_get_parent(line.depth);
            // Might want to do something for the other line kinds in the
            // future, e.g. impute placeholder "items" that have no info, or
            // only lang info (perhaps by making Item an enum?), to indicate
            // that there is a known missing step in the ety chain. Right now,
            // we just skip them. So e.g. for a descendants snippet like on the
            // page for PIE ḱerh₂-:
            //
            // * Unsorted formations: [BareText]
            // ** {{desc|grk-pro|}} [BareLang]
            // *** {{desc|grc|κάρυον}} [Desc]
            //
            // our resultant ety chain would just be  κάρυον -> ḱerh₂-.
            if let RawDescLineKind::Desc { desc } = &line.kind {
                if desc.terms.is_empty() || desc.terms.len() != desc.modes.len() {
                    continue;
                }
                let (mut desc_items, mut confidences, mut modes) = (vec![], vec![], vec![]);
                for (i, (&term, &mode)) in desc.terms.iter().zip(desc.modes.iter()).enumerate() {
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
                    let citation = Citation {
                        langterm,
                        id: None,
                        gloss: None,
                        citer: parent,
                        relation: Relation::Child,
                    };
                    let comp = ancestors.embeddings(self, embeddings)?;
                    let (desc_item, confidence) =
                        match self.resolve(string_pool, embeddings, &comp, &citation)? {
                            Resolution::Found { item, confidence } => (item, confidence),
                            Resolution::Missing => (
                                self.impute_item(langterm, item),
                                embeddings::SIMILARITY_THRESHOLD,
                            ),
                        };
                    // Only use the first term in a multi-term desc line as
                    // the ancestor for any deeper-nested lines below it.
                    if i == 0 {
                        ancestors.add(&desc_item, line.depth);
                    }
                    desc_items.push(desc_item);
                    confidences.push(confidence);
                    modes.push(mode);
                }
                for (desc_item, confidence, mut mode) in izip!(desc_items, confidences, modes) {
                    // Inheritance is from another language: an untagged
                    // same-language line is a derivation (e.g. a derived
                    // term listed under its base word).
                    if mode == EtyMode::Inherited
                        && self.get(desc_item).lang() == self.get(parent).lang()
                    {
                        mode = EtyMode::MorphologicalDerivation;
                    }
                    if confidence >= embeddings::SIMILARITY_THRESHOLD {
                        self.graph
                            .add_ety(desc_item, mode, Some(0), &[parent], &[confidence]);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::to_borrowed_value;

    fn lines(json: &str) -> (StringPool, Vec<RawDescLine>) {
        let mut bytes = json.as_bytes().to_vec();
        let value = to_borrowed_value(&mut bytes).unwrap();
        let mut string_pool = StringPool::new();
        let item = WiktextractJsonItem { json: value };
        let lines = item
            .get_descendants(&mut string_pool)
            .map_or_else(Vec::new, |d| Vec::from(d.lines));
        (string_pool, lines)
    }

    fn terms(string_pool: &StringPool, line: &RawDescLine) -> Vec<String> {
        match &line.kind {
            RawDescLineKind::Desc { desc } => desc
                .terms
                .iter()
                .map(|t| t.resolve(string_pool).to_owned())
                .collect(),
            _ => vec![],
        }
    }

    #[test]
    fn multi_term_line_is_regrouped() {
        let (sp, lines) = lines(
            r#"{"descendants": [{"lang_code": "ang", "word": "lām", "descendants": [
                {"lang_code": "enm", "word": "lam", "descendants": [{"lang_code": "en", "word": "loam"}]},
                {"lang_code": "enm", "word": "lame", "descendants": [{"lang_code": "en", "word": "loam"}]}
            ]}]}"#,
        );
        let got: Vec<_> = lines.iter().map(|l| (l.depth, terms(&sp, l))).collect();
        assert_eq!(
            got,
            [
                (1, vec!["lām".to_owned()]),
                (2, vec!["lam".to_owned(), "lame".to_owned()]),
                (3, vec!["loam".to_owned()]),
            ]
        );
    }

    #[test]
    fn childless_siblings_stay_separate_lines() {
        let (sp, lines) = lines(
            r#"{"descendants": [
                {"lang_code": "ine-pro", "word": "*mneh₂-", "tags": ["derived"]},
                {"lang_code": "ine-pro", "word": "*mn̥-tó", "tags": ["derived"]}
            ]}"#,
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(terms(&sp, &lines[0]), ["mneh₂-"]);
        match &lines[0].kind {
            RawDescLineKind::Desc { desc } => {
                assert_eq!(&*desc.modes, [EtyMode::MorphologicalDerivation]);
            }
            _ => panic!("expected a desc line"),
        }
    }

    #[test]
    fn headings_and_modes() {
        let (_, lines) = lines(
            r#"{"descendants": [
                {"lang_code": "unknown", "lang": "Borrowings", "descendants": [
                    {"lang_code": "fi", "word": "kuningas", "raw_tags": ["borrowed"]},
                    {"lang_code": "de", "word": "Halbinsel", "tags": ["calque"]},
                    {"lang_code": "ru", "word": "вода́", "raw_tags": ["learned borrowing", "borrowed"]}
                ]}
            ]}"#,
        );
        assert!(matches!(lines[0].kind, RawDescLineKind::BareText { .. }));
        let modes: Vec<_> = lines[1..]
            .iter()
            .map(|l| match &l.kind {
                RawDescLineKind::Desc { desc } => (l.depth, desc.modes[0]),
                _ => panic!("expected a desc line"),
            })
            .collect();
        assert_eq!(
            modes,
            [
                (2, EtyMode::Borrowed),
                (2, EtyMode::Calque),
                (2, EtyMode::LearnedBorrowing),
            ]
        );
    }
}
