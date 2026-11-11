use std::{collections::BTreeMap, mem, str::FromStr};

use crate::{
    HashMap, HashSet,
    embeddings::{self, Embeddings},
    entries::{EntryPos, Json, JsonStr, clean_template_term, modifier, split_modifiers},
    etymology::validate_ety_template_lang,
    items::Items,
    progress_bar,
};
use wety_core::{
    assertions::Asserted,
    ety_graph::{Certainty, EdgeBasis, EtyEdgeAccess, NewEtymology},
    etymology_templates::EtyMode,
    items::ItemId,
    langterm::{LangTerm, Term},
    languages::Lang,
    string_pool::StringPool,
};

use anyhow::{Ok, Result};
use std::sync::LazyLock;

use regex::Regex;
use simd_json::ValueAccess;

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawRoot {
    pub(crate) langterm: LangTerm,
    // The root's sense, e.g. "carry" in {{root|en|ine-pro|*bʰer- (carry)}} or
    // the category "... root *bʰer- (carry)". It is the {{etymon}} id of the
    // root's own entry, which distinguishes homograph roots.
    pub(crate) sense_id: Option<Box<str>>,
}

enum RootKind {
    Root,
    Word,
}

impl EntryPos<'_> {
    // The roots an entry descends from, per its first {{root}}-like template
    // (https://en.wiktionary.org/wiki/Template:root, and the similar
    // https://en.wiktionary.org/wiki/Template:word and
    // https://en.wiktionary.org/wiki/Template:PIE_word), else per its root
    // categories, which {{etymon}} and other templates also generate. These
    // indicate ultimate descent from a proto-language term, which we record as
    // "root" etys. An entry may name several roots (e.g. a compound), or one
    // root under several senses (see ambiguous_roots).
    pub(crate) fn get_roots(&self, string_pool: &mut StringPool, lang: Lang) -> Vec<RawRoot> {
        self.get_named_roots(string_pool, lang)
    }

    fn get_named_roots(&self, string_pool: &mut StringPool, lang: Lang) -> Vec<RawRoot> {
        {
            for &template in self.templates {
                if let Some(name) = template.get_valid_str("name")
                    && let Some(args) = template.get("args")
                {
                    let roots = match name {
                        "root" => process_root_template(string_pool, args, lang, &RootKind::Root),
                        "word" => process_root_template(string_pool, args, lang, &RootKind::Word),
                        "PIE word" => process_pie_word_template(string_pool, args, lang),
                        _ => continue,
                    };
                    if !roots.is_empty() {
                        return roots;
                    }
                }
            }
        }

        // e.g. "English terms derived from the Proto-Indo-European root *dʰeh₁-"
        // or "English terms derived from the Proto-Indo-European root *bʰel- (shiny)"
        let mut roots: Vec<RawRoot> = vec![];
        for category in self.categories() {
            if let Some(root) = process_json_root_category(string_pool, category, lang)
                && !roots.contains(&root)
            {
                roots.push(root);
            }
        }
        roots
    }
}

// https://en.wiktionary.org/wiki/Template:root
// https://en.wiktionary.org/wiki/Template:word
// This handles both {{root}} and {{word}}. The templates work the same, with
// the only exception being that the arg for the proto-language uses the
// standard language code in the case of {{root}}, while "-pro" is clipped off
// in the case of {{word}}. The roots are args "3", "4", ...
fn process_root_template(
    string_pool: &mut StringPool,
    args: &Json,
    lang: Lang,
    kind: &RootKind,
) -> Vec<RawRoot> {
    let mut roots = vec![];
    if validate_ety_template_lang(args, lang).is_err() {
        return roots;
    }
    let Some(root_lang) = args.get_valid_str("2").and_then(|code| match kind {
        RootKind::Root => Lang::from_str(code).ok(),
        RootKind::Word => Lang::from_str(&format!("{code}-pro")).ok(),
    }) else {
        return roots;
    };
    for n in 3.. {
        let Some(raw_root_term) = args.get_valid_str(n.to_string().as_str()) else {
            break;
        };
        let (term, modifiers) = split_modifiers(raw_root_term);
        let root_term = clean_template_term(term);
        if root_term.is_empty() {
            continue;
        }
        // The sense is given in parentheses after the term (see e.g.
        // https://en.wiktionary.org/wiki/blaze), as <id:...>, or for a single
        // root as id=.
        let sense_id = term
            .rfind(" (")
            .and_then(|l| Some(&term[l + 2..term.rfind(')')?]))
            .or_else(|| modifier(&modifiers, "id"))
            .or_else(|| (n == 3).then(|| args.get_valid_str("id")).flatten())
            .filter(|id| !id.is_empty())
            .map(Box::from);
        let langterm = root_lang.new_langterm(string_pool, root_term);
        roots.push(RawRoot { langterm, sense_id });
    }
    roots
}

// https://en.wiktionary.org/wiki/Template:PIE_word
fn process_pie_word_template(
    string_pool: &mut StringPool,
    args: &Json,
    lang: Lang,
) -> Vec<RawRoot> {
    if validate_ety_template_lang(args, lang).is_err() {
        return vec![];
    }
    let (Some(pie_lang), Some(pie_word)) =
        (Lang::from_str("ine-pro").ok(), args.get_valid_term("2"))
    else {
        return vec![];
    };
    vec![RawRoot {
        langterm: pie_lang.new_langterm(string_pool, pie_word),
        sense_id: None,
    }]
}

fn process_json_root_category(
    string_pool: &mut StringPool,
    category: &str,
    lang: Lang,
) -> Option<RawRoot> {
    static ROOT_CAT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(.+) terms derived from the (.+) root \*([^ ]+)(?: \((.+)\))?$").unwrap()
    });
    let caps = ROOT_CAT.captures(category)?;
    let cat_term_lang_name = caps.get(1).map(|m| m.as_str())?;
    let cat_term_lang = Lang::from_name(cat_term_lang_name).ok()?;
    (cat_term_lang == lang).then_some(())?;
    let cat_root_lang_name = caps.get(2).map(|m| m.as_str())?;
    let cat_root_lang = Lang::from_name(cat_root_lang_name).ok()?;
    let cat_root_term = caps.get(3).map(|m| m.as_str())?;
    let cat_root_term = Term::new(string_pool, cat_root_term);
    Some(RawRoot {
        langterm: LangTerm::new(cat_root_lang, cat_root_term),
        sense_id: caps.get(4).map(|cap| Box::from(cap.as_str())),
    })
}

// The roots an entry names under several senses, in the order it names them.
// Such an entry does not say which it means: pro-, per- and prae- words name
// both *per- (before) and *per- (fare), since Wiktionary derives *pér,
// uncertainly, from either.
fn ambiguous_roots(roots: &[RawRoot]) -> Vec<LangTerm> {
    let mut ambiguous = vec![];
    for root in roots {
        let senses = roots
            .iter()
            .filter(|r| r.langterm == root.langterm)
            .filter_map(|r| r.sense_id.as_deref())
            .collect::<HashSet<_>>()
            .len();
        if senses > 1 && !ambiguous.contains(&root.langterm) {
            ambiguous.push(root.langterm);
        }
    }
    ambiguous
}

// The sets of roots an entry may mean: one per choice of sense for each root it
// names under several, each with every root it names under one. Too many
// choices give the unambiguous roots alone.
const MAX_ROOT_ALTERNATIVES: usize = 4;

fn root_alternatives(roots: &[RawRoot]) -> Vec<Vec<&RawRoot>> {
    let ambiguous = ambiguous_roots(roots);
    let plain: Vec<&RawRoot> = roots
        .iter()
        .filter(|r| !ambiguous.contains(&r.langterm))
        .collect();
    let mut alternatives = vec![plain.clone()];
    for langterm in ambiguous {
        let mut senses: Vec<&RawRoot> = vec![];
        for root in roots.iter().filter(|r| r.langterm == langterm) {
            if !senses.iter().any(|s| s.sense_id == root.sense_id) {
                senses.push(root);
            }
        }
        alternatives = alternatives
            .iter()
            .flat_map(|alt| {
                senses.iter().map(move |&sense| {
                    let mut alt = alt.clone();
                    alt.push(sense);
                    alt
                })
            })
            .collect();
    }
    if alternatives.len() > MAX_ROOT_ALTERNATIVES {
        return vec![plain];
    }
    alternatives
}

impl Items {
    // The item a root names: by its sense id where that is some root entry's
    // {{etymon}} id, else among homographs the one that best fits the item by
    // meaning (embeddings::citation_score).
    fn find_root(
        &self,
        embeddings: &Embeddings,
        item: ItemId,
        root: &RawRoot,
    ) -> Result<Option<(ItemId, f32)>> {
        if let Some(sense_id) = &root.sense_id
            && let Some(root_item) = self.get_id_item(root.langterm, sense_id)
        {
            return Ok(Some((root_item, 1.0)));
        }
        let Some(candidates) = self.candidates(root.langterm) else {
            return Ok(None);
        };
        if let [only] = candidates[..] {
            return Ok(Some((only, 0.5)));
        }
        let embedding = embeddings.get(self.get(item), item);
        let mut best: Option<(ItemId, f32)> = None;
        for &candidate in candidates {
            let candidate_embedding = embeddings.get(self.get(candidate), candidate);
            let score = embeddings::citation_score(&embedding, &candidate_embedding, true, None);
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((candidate, score));
            }
        }
        Ok(best)
    }

    // Whether a root may be imputed as an item's ety: the item's language must
    // descend from the root's (an imputed item's may be the root's own).
    // An imputed item may be a derivative in its root's own language (*bʰor-no-m
    // < *bʰer-), unless it is a root or affix itself (*bʰor-).
    fn may_descend_from_root(
        &self,
        string_pool: &StringPool,
        item: ItemId,
        root_item: ItemId,
    ) -> bool {
        let item = self.get(item);
        let root_lang = self.get(root_item).lang();
        item.lang().strictly_descends_from(root_lang)
            || (item.is_imputed()
                && item.lang().descends_from(root_lang)
                && !item.term().resolve(string_pool).ends_with('-'))
    }

    // The langterms of every item in an item's etymology: its parents, theirs
    // and so on, along every part, not only the head.
    fn ancestor_langterms(&self, item: ItemId) -> HashSet<LangTerm> {
        let mut seen = HashSet::default();
        let mut langterms = HashSet::default();
        let mut queue = vec![item];
        while let Some(item) = queue.pop() {
            for edge in self.graph.parent_edges(item) {
                let parent = edge.parent();
                if seen.insert(parent) {
                    let parent_item = self.get(parent);
                    langterms.insert(LangTerm::new(parent_item.lang(), parent_item.term()));
                    queue.push(parent);
                }
            }
        }
        langterms
    }

    // Root etys are imputed in two passes, each over items in id order so that
    // the result does not depend on hash-map order.
    //
    // 1. An item with no ety of its own gets its roots as its ety (imputing a
    //    root missing from the data). An item naming a root under several
    //    senses gets one etymology for each, alternatives with no primary.
    // 2. An item with an ety chain that does not reach one of its roots says
    //    that the top of its head chain (its head progenitor) descends from
    //    that root, if that has no ety; the chain also stops at an ety without
    //    a head, such as a compound of two bases. A root counts as reached when any part of the
    //    item's etymology, at any depth, is that root or a homograph of it: the
    //    categories of a derivative also cover its other parts (pl
    //    przetestować, prze- + testować, names *per- (fare) for prze-, whose
    //    chain reaches *per- (before)), and these must not speak for the base's
    //    progenitor (la testum). Nor do they when another part's line also
    //    ends without an etymology in a language that may descend from the
    //    root: the category may be that part's. Many items may speak for the
    //    same progenitor,
    //    not always alike, so each progenitor gets the root most of them name,
    //    an item naming n unreached roots giving each 1/n of a vote. Where the
    //    votes do not single out one root, the progenitor gets none. A root
    //    named under several senses is no vote for either.
    pub(crate) fn impute_root_etys(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
    ) -> Result<()> {
        let raw_roots: BTreeMap<ItemId, Box<[RawRoot]>> = mem::take(&mut self.raw_templates.root)
            .into_iter()
            .collect();
        let pb = progress_bar(raw_roots.len() * 2, "Imputing root etys")?;

        for (&item_id, roots) in &raw_roots {
            pb.inc(1);
            if self.graph.immediate_ety(item_id).is_some() {
                continue;
            }
            let mut alternatives: Vec<(Vec<ItemId>, Vec<f32>)> = vec![];
            for alternative in root_alternatives(roots) {
                let (mut root_items, mut confidences) = (vec![], vec![]);
                for root in alternative {
                    let (root_item, confidence) = match self.find_root(embeddings, item_id, root)? {
                        Some(found) => found,
                        None => (self.impute_item(root.langterm, item_id), 0.0),
                    };
                    if self.may_descend_from_root(string_pool, item_id, root_item)
                        && !root_items.contains(&root_item)
                    {
                        root_items.push(root_item);
                        confidences.push(confidence);
                    }
                }
                if !root_items.is_empty() && !alternatives.iter().any(|(r, _)| *r == root_items) {
                    alternatives.push((root_items, confidences));
                }
            }
            let certainties: Vec<Vec<Certainty>> = alternatives
                .iter()
                .map(|(r, _)| vec![Certainty::Stated; r.len()])
                .collect();
            let etys: Vec<NewEtymology> = alternatives
                .iter()
                .zip(&certainties)
                .map(|((root_items, confidences), certainties)| NewEtymology {
                    mode: EtyMode::Root,
                    head: (root_items.len() == 1).then_some(0),
                    basis: EdgeBasis::Root,
                    items: root_items,
                    confidences,
                    certainties,
                })
                .collect();
            if !etys.is_empty() {
                self.graph.add_etymologies(item_id, &etys);
            }
        }

        // progenitor -> root -> (votes, voters)
        let mut votes: BTreeMap<ItemId, HashMap<ItemId, (f32, u32)>> = BTreeMap::new();
        for (&item_id, roots) in &raw_roots {
            pb.inc(1);
            let Some(progenitors) = self.graph.progenitors(item_id) else {
                continue;
            };
            // The head chain stops at an etymology without a head (de
            // anderweit = ander + weit), which is no gap for a root to fill.
            let Some(head_progenitor) = progenitors
                .head
                .filter(|&p| self.graph.immediate_ety(p).is_none())
            else {
                continue;
            };
            let reached = self.ancestor_langterms(item_id);
            let ambiguous = ambiguous_roots(roots);
            let mut unreached = vec![];
            for root in roots.iter().filter(|r| !ambiguous.contains(&r.langterm)) {
                if let Some((root_item, _)) = self.find_root(embeddings, item_id, root)?
                    && !reached.contains(&LangTerm::new(
                        self.get(root_item).lang(),
                        self.get(root_item).term(),
                    ))
                    && self.may_descend_from_root(string_pool, head_progenitor, root_item)
                    // Another part's line also ends without an etymology
                    // where the root could be its source: the category may
                    // be that part's (Lycosa, λύκος + -ōsus, filed under
                    // *h₃ed- "smell" for the suffix), so it is no vote for
                    // the head's (*wl̥kʷós).
                    && !progenitors.items.iter().any(|&p| {
                        p != head_progenitor && self.may_descend_from_root(string_pool, p, root_item)
                    })
                    && !unreached.contains(&root_item)
                {
                    unreached.push(root_item);
                }
            }
            #[allow(clippy::cast_precision_loss)]
            let share = 1.0 / unreached.len() as f32;
            for root_item in unreached {
                let vote = votes
                    .entry(head_progenitor)
                    .or_default()
                    .entry(root_item)
                    .or_default();
                vote.0 += share;
                vote.1 += 1;
            }
        }
        for (progenitor, roots) in votes {
            let cmp = |a: &(f32, u32), b: &(f32, u32)| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1));
            let mut ranked: Vec<_> = roots.iter().collect();
            ranked.sort_unstable_by(|(a, va), (b, vb)| cmp(vb, va).then(a.cmp(b)));
            let [(root_item, best), rest @ ..] = &ranked[..] else {
                continue;
            };
            let root_item = **root_item;
            if rest
                .first()
                .is_some_and(|(_, next)| cmp(next, best).is_eq())
            {
                continue;
            }
            // a confidence from the share of the votes
            let (votes, voters) = roots[&root_item];
            let total: f32 = roots.values().map(|(v, _)| v).sum();
            if self.graph.add_ety(
                progenitor,
                EtyMode::Root,
                Some(0u8),
                EdgeBasis::RootVote,
                &[root_item],
                &[votes / total],
            ) {
                let asserted = Asserted {
                    inputs: voters,
                    ..Asserted::default()
                };
                self.graph.set_asserted(progenitor, vec![asserted]);
            }
        }
        pb.finish();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entries::test_entry;

    fn roots(json: &str) -> Vec<(String, Option<String>)> {
        let mut sp = StringPool::new();
        test_entry::with(json, |e| {
            e.get_roots(&mut sp, Lang::from_str("en").unwrap())
        })
        .into_iter()
        .map(|r| {
            (
                format!(
                    "{}:{}",
                    r.langterm.lang.code(),
                    r.langterm.term.resolve(&sp)
                ),
                r.sense_id.map(String::from),
            )
        })
        .collect()
    }

    #[test]
    fn root_templates_and_categories() {
        let pie = |t: &str, s: Option<&str>| (format!("ine-pro:{t}"), s.map(String::from));
        // several roots, with senses
        assert_eq!(
            roots(
                r#"{"etymology_templates": [{"name": "root",
                "args": {"1": "en", "2": "ine-pro", "3": "*bʰer- (carry)", "4": "*pleh₂k-<id:flat>"}}]}"#
            ),
            [pie("bʰer-", Some("carry")), pie("pleh₂k-", Some("flat"))]
        );
        assert_eq!(
            roots(
                r#"{"etymology_templates": [{"name": "root",
                "args": {"1": "en", "2": "ine-pro", "3": "*bʰel-", "id": "shiny"}}]}"#
            ),
            [pie("bʰel-", Some("shiny"))]
        );
        // all root categories, once each
        assert_eq!(
            roots(
                r#"{"categories": [
                "English terms derived from the Proto-Indo-European root *bʰer- (carry)",
                "English lemmas",
                "English terms derived from the Proto-Indo-European root *per- (before)",
                "English terms derived from the Proto-Indo-European root *bʰer- (carry)"]}"#
            ),
            [pie("bʰer-", Some("carry")), pie("per-", Some("before"))]
        );
    }

    #[test]
    fn a_root_named_under_several_senses_gives_alternatives() {
        // en product: pro- (from *pér, from either *per-) + dūcō (*dewk-)
        let json = r#"{"categories": [
            "English terms derived from the Proto-Indo-European root *per- (before)",
            "English terms derived from the Proto-Indo-European root *per- (fare)",
            "English terms derived from the Proto-Indo-European root *dewk-"]}"#;
        let mut sp = StringPool::new();
        let named = test_entry::with(json, |e| {
            e.get_roots(&mut sp, Lang::from_str("en").unwrap())
        });
        let alternatives: Vec<Vec<String>> = root_alternatives(&named)
            .into_iter()
            .map(|alt| {
                alt.into_iter()
                    .map(|r| {
                        format!(
                            "{}{}",
                            r.langterm.term.resolve(&sp),
                            r.sense_id
                                .as_deref()
                                .map(|s| format!(" ({s})"))
                                .unwrap_or_default()
                        )
                    })
                    .collect()
            })
            .collect();
        assert_eq!(
            alternatives,
            [["dewk-", "per- (before)"], ["dewk-", "per- (fare)"]]
        );
        // a {{root}} naming one root under two senses
        let pie = |t: &str, s: Option<&str>| (format!("ine-pro:{t}"), s.map(String::from));
        assert_eq!(
            roots(
                r#"{"etymology_templates": [{"name": "root",
                "args": {"1": "en", "2": "ine-pro", "3": "*per- (before)", "4": "*per-<id:fare>"}}]}"#
            ),
            [pie("per-", Some("before")), pie("per-", Some("fare"))]
        );
    }
}
