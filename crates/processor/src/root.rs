use std::{collections::BTreeMap, mem, str::FromStr};

use crate::{
    HashMap,
    embeddings::{self, Embeddings},
    etymology::validate_ety_template_lang,
    items::Items,
    progress_bar,
    wiktextract_json::{
        WiktextractJson, WiktextractJsonItem, WiktextractJsonValidStr, clean_template_term,
        modifier, split_modifiers,
    },
};
use wety_core::{
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

impl WiktextractJsonItem<'_> {
    // The roots an entry descends from, per its first {{root}}-like template
    // (https://en.wiktionary.org/wiki/Template:root, and the similar
    // https://en.wiktionary.org/wiki/Template:word and
    // https://en.wiktionary.org/wiki/Template:PIE_word), else per its root
    // categories, which {{etymon}} and other templates also generate. These
    // indicate ultimate descent from a proto-language term, which we record as
    // "root" etys. An entry may name several roots (e.g. a compound).
    pub(crate) fn get_roots(&self, string_pool: &mut StringPool, lang: Lang) -> Vec<RawRoot> {
        if let Some(templates) = self.json.get_array("etymology_templates") {
            for template in templates {
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
        for category in self
            .json
            .get_array("categories")
            .into_iter()
            .flatten()
            .filter_map(|c| c.as_str())
        {
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
    args: &WiktextractJson,
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
    args: &WiktextractJson,
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
        let embedding = embeddings.get(self.get(item), item)?;
        let mut best: Option<(ItemId, f32)> = None;
        for &candidate in candidates {
            let candidate_embedding = embeddings.get(self.get(candidate), candidate)?;
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
    fn may_descend_from_root(&self, string_pool: &StringPool, item: ItemId, root_item: ItemId) -> bool {
        let item = self.get(item);
        let root_lang = self.get(root_item).lang();
        item.lang().strictly_descends_from(root_lang)
            || (item.is_imputed()
                && item.lang().descends_from(root_lang)
                && !item.term().resolve(string_pool).ends_with('-'))
    }

    // Root etys are imputed in two passes, each over items in id order so that
    // the result does not depend on hash-map order.
    //
    // 1. An item with no ety of its own gets its roots as its ety (imputing a
    //    root missing from the data).
    // 2. An item with an ety chain that does not reach one of its roots says
    //    that the top of its head chain (its head progenitor, which has no ety)
    //    descends from that root. Many items may speak for the same progenitor,
    //    not always alike (their categories also cover other parts of their
    //    ancestry), so each progenitor gets the root most of them name, an item
    //    naming n unreached roots giving each 1/n of a vote.
    pub(crate) fn impute_root_etys(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
    ) -> Result<()> {
        let raw_roots: BTreeMap<ItemId, Box<[RawRoot]>> =
            mem::take(&mut self.raw_templates.root).into_iter().collect();
        let pb = progress_bar(raw_roots.len() * 2, "Imputing root etys")?;

        for (&item_id, roots) in &raw_roots {
            pb.inc(1);
            if self.graph.immediate_ety(item_id).is_some() {
                continue;
            }
            let (mut root_items, mut confidences) = (vec![], vec![]);
            for root in roots {
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
            if !root_items.is_empty() {
                let head = (root_items.len() == 1).then_some(0);
                self.graph
                    .add_ety(item_id, EtyMode::Root, head, &root_items, &confidences);
            }
        }

        // progenitor -> root -> (votes, voters)
        let mut votes: BTreeMap<ItemId, HashMap<ItemId, (f32, u32)>> = BTreeMap::new();
        for (&item_id, roots) in &raw_roots {
            pb.inc(1);
            let Some(progenitors) = self.graph.progenitors(item_id) else {
                continue;
            };
            let Some(head_progenitor) = progenitors.head else {
                continue;
            };
            let mut unreached = vec![];
            for root in roots {
                if let Some((root_item, _)) = self.find_root(embeddings, item_id, root)?
                    && !progenitors.items.contains(&root_item)
                    && head_progenitor != root_item
                    && self.may_descend_from_root(string_pool, head_progenitor, root_item)
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
            let Some((&root_item, _)) = roots.iter().max_by(|(a, va), (b, vb)| {
                va.0.total_cmp(&vb.0)
                    .then(va.1.cmp(&vb.1))
                    .then(b.cmp(a)) // the lower id on a tie
            }) else {
                continue;
            };
            // a confidence from the share of the votes
            let (votes, _) = roots[&root_item];
            let total: f32 = roots.values().map(|(v, _)| v).sum();
            self.graph.add_ety(
                progenitor,
                EtyMode::Root,
                Some(0u8),
                &[root_item],
                &[votes / total],
            );
        }
        pb.finish();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::to_borrowed_value;

    fn roots(json: &str) -> Vec<(String, Option<String>)> {
        let mut bytes = json.as_bytes().to_vec();
        let json = to_borrowed_value(&mut bytes).unwrap();
        let mut sp = StringPool::new();
        WiktextractJsonItem { json }
            .get_roots(&mut sp, Lang::from_str("en").unwrap())
            .into_iter()
            .map(|r| {
                (
                    format!("{}:{}", r.langterm.lang.code(), r.langterm.term.resolve(&sp)),
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
            roots(r#"{"etymology_templates": [{"name": "root",
                "args": {"1": "en", "2": "ine-pro", "3": "*bʰer- (carry)", "4": "*pleh₂k-<id:flat>"}}]}"#),
            [pie("bʰer-", Some("carry")), pie("pleh₂k-", Some("flat"))]
        );
        assert_eq!(
            roots(r#"{"etymology_templates": [{"name": "root",
                "args": {"1": "en", "2": "ine-pro", "3": "*bʰel-", "id": "shiny"}}]}"#),
            [pie("bʰel-", Some("shiny"))]
        );
        // all root categories, once each
        assert_eq!(
            roots(r#"{"categories": [
                "English terms derived from the Proto-Indo-European root *bʰer- (carry)",
                "English lemmas",
                "English terms derived from the Proto-Indo-European root *per- (before)",
                "English terms derived from the Proto-Indo-European root *bʰer- (carry)"]}"#),
            [pie("bʰer-", Some("carry")), pie("per-", Some("before"))]
        );
    }
}
