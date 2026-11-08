use crate::{
    HashSet,
    embeddings::Embeddings,
    entries::{EntryPos, Json, JsonStr, clean_template_term},
    items::Items,
    progress_bar,
    resolve::{Citation, Relation, Resolution},
};
use wety_core::{
    assertions::Asserted,
    ety_graph::EdgeBasis,
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
impl EntryPos<'_> {
    /// The Descendants section under this part of speech.
    pub(crate) fn get_descendants(&self, string_pool: &mut StringPool) -> Option<RawDescendants> {
        lines_of(string_pool, self.pos.get_array("descendants")?)
    }

    /// The Descendants section beside the parts of speech, for the whole entry.
    pub(crate) fn get_entry_descendants(
        &self,
        string_pool: &mut StringPool,
    ) -> Option<RawDescendants> {
        lines_of(string_pool, self.entry.get_array("descendants")?)
    }
}

// The entries give a Descendants section as a tree of list items, each with its
// text, its template calls and its links.
fn lines_of(string_pool: &mut StringPool, nodes: &[Json]) -> Option<RawDescendants> {
    let mut lines = vec![];
    flatten(string_pool, nodes, 1, &mut lines);
    (!lines.is_empty()).then(|| lines.into())
}

fn flatten(string_pool: &mut StringPool, nodes: &[Json], depth: u8, lines: &mut Vec<RawDescLine>) {
    for node in nodes {
        lines.push(RawDescLine {
            depth,
            kind: desc_line(string_pool, node),
        });
        if let Some(children) = node.get_array("children") {
            flatten(string_pool, children, depth.saturating_add(1), lines);
        }
    }
}

const DESC_TEMPLATES: [&str; 4] = ["desc", "descendant", "desctree", "descendants tree"];
const LINK_TEMPLATES: [&str; 3] = ["l", "link", "ll"];

/// Templates whose list gives a word's readings in other languages, not its
/// descendants: {{CJKV}} lists the Sino-Xenic readings of a Chinese word, written
/// with its characters, which may be Japanese coinages Chinese borrowed, or native
/// words written with the same characters.
const READING_LISTS: [&str; 1] = ["cjkv"];

fn desc_line(string_pool: &mut StringPool, node: &Json) -> RawDescLineKind {
    if node
        .get_str("via")
        .is_some_and(|via| READING_LISTS.contains(&via.to_lowercase().as_str()))
    {
        return RawDescLineKind::Other;
    }
    let calls = node
        .get_array("calls")
        .map(Vec::as_slice)
        .unwrap_or_default();
    // {{desc|enm|lam|lame|bor=1}}: a language, its terms, each with its mode
    if let Some(call) = calls.iter().find(|c| {
        c.get_str("name")
            .is_some_and(|n| DESC_TEMPLATES.contains(&n))
    }) && let Some(args) = call.get("args")
        && let Some(lang) = args
            .get_valid_str("1")
            .and_then(|code| Lang::from_str(code).ok())
    {
        let (mut terms, mut modes): (Vec<_>, Vec<_>) = (2..)
            .map_while(|n| args.get_str(n.to_string().as_str()).map(|t| (n - 1, t)))
            .filter_map(|(i, term)| {
                let term = clean_template_term(term.trim());
                (!term.is_empty() && term != "-")
                    .then(|| (Term::new(string_pool, term), desc_mode(args, i)))
            })
            .unzip();
        // further forms the line links in the same language:
        // "{{desc|be|обро́ць|абро́ць}}; {{l|be|абро́т|g=m}}", with the line's flags
        for link in calls
            .iter()
            .filter(|c| {
                c.get_str("name")
                    .is_some_and(|n| LINK_TEMPLATES.contains(&n))
            })
            .filter_map(|c| c.get("args"))
            .filter(|a| a.get_valid_str("1") == Some(lang.code()))
        {
            if let Some(term) = link.get_valid_term("2") {
                let term = Term::new(string_pool, term);
                if !terms.contains(&term) {
                    terms.push(term);
                    modes.push(desc_mode(args, 0));
                }
            }
        }
        if terms.is_empty() {
            return RawDescLineKind::BareLang { lang };
        }
        let desc = RawDesc {
            lang,
            terms: terms.into_boxed_slice(),
            modes: modes.into_boxed_slice(),
        };
        return RawDescLineKind::Desc { desc };
    }
    // * {{l|en|foo}}
    if let Some(call) = calls.iter().find(|c| {
        c.get_str("name")
            .is_some_and(|n| LINK_TEMPLATES.contains(&n))
    }) && let Some(args) = call.get("args")
        && let Some(lang) = args
            .get_valid_str("1")
            .and_then(|code| Lang::from_str(code).ok())
        && let Some(term) = args.get_valid_term("2")
    {
        let desc = RawDesc {
            lang,
            terms: vec![Term::new(string_pool, term)].into_boxed_slice(),
            modes: vec![EtyMode::Inherited].into_boxed_slice(),
        };
        return RawDescLineKind::Desc { desc };
    }
    let text = node.get_str("text").unwrap_or_default().trim();
    // An item another page's {{desctree}} drew: its links name the terms (the
    // pages Wiktionary computed for them), the anchors their language; "→" marks
    // a borrowing.
    if calls.is_empty()
        && let Some(links) = node.get_array("links")
        && let Some(anchor) = links.first().and_then(|l| l.get_str("anchor"))
        && let Some(lang) = Lang::from_name(anchor).ok()
    {
        let mode = if text.starts_with('→') {
            EtyMode::Borrowed
        } else {
            EtyMode::Inherited
        };
        let terms: Vec<Term> = links
            .iter()
            .filter(|l| l.get_str("anchor") == Some(anchor))
            .filter_map(|l| l.get_valid_term("page"))
            .map(|page| Term::new(string_pool, page))
            .collect();
        let modes = vec![mode; terms.len()];
        let desc = RawDesc {
            lang,
            terms: terms.into_boxed_slice(),
            modes: modes.into_boxed_slice(),
        };
        return RawDescLineKind::Desc { desc };
    }
    // headings: a language or family ("Old French:"), or text ("Unsorted
    // formations")
    let heading = text.trim_end_matches(':').trim();
    if calls.is_empty() && !heading.is_empty() {
        return match Lang::from_name(heading).ok() {
            Some(lang) => RawDescLineKind::BareLang { lang },
            None => RawDescLineKind::BareText {
                text: Gloss::new(string_pool, heading),
            },
        };
    }
    RawDescLineKind::Other
}

// A {{desc}} term's mode, from its flags: per term (bor2=1) or for all (bor=1).
fn desc_mode(args: &Json, term: usize) -> EtyMode {
    // most specific first
    const FLAGS: [(&str, EtyMode); 11] = [
        ("pclq", EtyMode::PartialCalque),
        ("pcal", EtyMode::PartialCalque),
        ("clq", EtyMode::Calque),
        ("cal", EtyMode::Calque),
        ("sml", EtyMode::SemanticLoan),
        ("slb", EtyMode::SemiLearnedBorrowing),
        ("lbor", EtyMode::LearnedBorrowing),
        ("bor", EtyMode::Borrowed),
        ("translit", EtyMode::Transliteration),
        // shown as "⇒", "reshaped by analogy or addition of morphemes"
        ("der", EtyMode::MorphologicalDerivation),
        ("inh", EtyMode::Inherited),
    ];
    let set = |key: &str| {
        args.get_valid_str(key)
            .is_some_and(|v| v != "0" && v != "no")
    };
    FLAGS
        .iter()
        .find(|(flag, _)| set(&format!("{flag}{term}")) || set(flag))
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
                    }
                }
                if has_ambiguous_child {
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
        let mut raw_templates_desc: Vec<_> = mem::take(&mut self.raw_templates.desc)
            .into_iter()
            .collect();
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
                        pos: None,
                        citer: parent,
                        relation: Relation::Child,
                    };
                    let (desc_item, confidence) =
                        match self.resolve(string_pool, embeddings, &citation)? {
                            Resolution::Found { item, confidence } => (item, confidence),
                            Resolution::Missing => (self.impute_item(langterm, item), 0.0),
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
                    // stated in `item`'s Descendants section
                    if self.graph.add_ety(
                        desc_item,
                        mode,
                        Some(0),
                        EdgeBasis::Descendants,
                        &[parent],
                        &[confidence],
                    ) {
                        let asserted = Asserted {
                            page: Some(item),
                            ..Asserted::default()
                        };
                        self.graph.set_asserted(desc_item, vec![asserted]);
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
    use crate::entries::test_entry;

    fn lines(json: &str) -> (StringPool, Vec<RawDescLine>) {
        let mut string_pool = StringPool::new();
        let lines = test_entry::with(json, |item| item.get_descendants(&mut string_pool))
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

    fn modes(line: &RawDescLine) -> Vec<EtyMode> {
        match &line.kind {
            RawDescLineKind::Desc { desc } => desc.modes.to_vec(),
            _ => panic!("expected a desc line"),
        }
    }

    #[test]
    fn a_line_with_several_terms_is_one_line() {
        let (sp, lines) = lines(
            r#"{"pos": [{"descendants": [
                {"text": "Old English: lām", "calls": [{"name": "desc", "args": {"1": "ang", "2": "lām"}}], "children": [
                    {"text": "Middle English: lam, lame", "calls": [{"name": "desc", "args": {"1": "enm", "2": "lam", "3": "lame"}}], "children": [
                        {"text": "English: loam", "calls": [{"name": "desc", "args": {"1": "en", "2": "loam"}}]}
                    ]}
                ]}
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
    fn flags_give_modes_per_term_or_for_all() {
        let (_, lines) = lines(
            r#"{"pos": [{"descendants": [
                {"text": "Borrowings:", "calls": [], "children": [
                    {"text": "Finnish: kuningas", "calls": [{"name": "desc", "args": {"1": "fi", "2": "kuningas", "bor": "1"}}]},
                    {"text": "German: Halbinsel", "calls": [{"name": "desc", "args": {"1": "de", "2": "Halbinsel", "clq": "1"}}]},
                    {"text": "Russian: вода́, вода", "calls": [{"name": "desc", "args": {"1": "ru", "2": "вода́", "3": "вода", "lbor2": "1"}}]},
                    {"text": "Proto-Indo-European: *mn̥-tó", "calls": [{"name": "desc", "args": {"1": "ine-pro", "2": "*mn̥-tó", "der": "1"}}]}
                ]}
            ]}]}"#,
        );
        assert!(matches!(lines[0].kind, RawDescLineKind::BareText { .. }));
        assert_eq!(modes(&lines[1]), [EtyMode::Borrowed]);
        assert_eq!(modes(&lines[2]), [EtyMode::Calque]);
        assert_eq!(
            modes(&lines[3]),
            [EtyMode::Inherited, EtyMode::LearnedBorrowing]
        );
        assert_eq!(modes(&lines[4]), [EtyMode::MorphologicalDerivation]);
    }

    #[test]
    fn sino_xenic_readings_are_not_descendants() {
        let (_, lines) = lines(
            r#"{"pos": [{"descendants": [
                {"text": "→ Japanese: 孝敬(こうけい) (kōkei)", "calls": [], "via": "cJKV",
                    "links": [{"page": "孝敬", "anchor": "Japanese"}]},
                {"text": "→ Vietnamese: căn", "calls": [{"name": "desc", "args": {"1": "vi", "2": "căn", "bor": "1"}}]}
            ]}]}"#,
        );
        assert!(matches!(lines[0].kind, RawDescLineKind::Other));
        assert_eq!(modes(&lines[1]), [EtyMode::Borrowed]);
    }

    #[test]
    fn forms_a_desc_line_links_are_its_terms_too() {
        let (sp, lines) = lines(
            r#"{"pos": [{"descendants": [
                {"text": "Belarusian: обро́ць, абро́ць; абро́т", "calls": [
                    {"name": "desc", "args": {"1": "be", "2": "обро́ць", "3": "абро́ць"}},
                    {"name": "l", "args": {"1": "be", "2": "абро́т", "g": "m"}},
                    {"name": "l", "args": {"1": "ru", "2": "оброть"}}]}
            ]}]}"#,
        );
        assert_eq!(terms(&sp, &lines[0]), ["обро́ць", "абро́ць", "абро́т"]);
    }

    #[test]
    fn items_another_page_drew_are_read_from_their_links() {
        let (sp, lines) = lines(
            r#"{"pos": [{"descendants": [
                {"text": "Sicilian: lupu", "calls": [{"name": "desctree", "args": {"1": "scn", "2": "lupu"}}], "children": [
                    {"text": "→ Maltese: lupu", "calls": [], "links": [{"page": "lupu", "anchor": "Maltese"}]}
                ]},
                {"text": "Old French:", "calls": []},
                {"text": "Balkan Romance:", "calls": []}
            ]}]}"#,
        );
        assert_eq!(terms(&sp, &lines[1]), ["lupu"]);
        assert_eq!(modes(&lines[1]), [EtyMode::Borrowed]);
        assert!(matches!(lines[2].kind, RawDescLineKind::BareLang { .. }));
        assert!(matches!(lines[3].kind, RawDescLineKind::BareText { .. }));
    }
}
