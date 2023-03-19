use crate::{
    embeddings::{Embeddings, ItemEmbedding},
    ety_graph::EtyGraph,
    etymology_templates::EtyMode,
    lang_phf::{LANG_CODE2NAME, LANG_NAME2CODE},
    phf_ext::OrderedSetExt,
    pos_phf::POS,
    progress_bar,
    raw_items::{RawItem, RawItems},
    string_pool::Symbol,
    wiktextract_json::{WiktextractJson, WiktextractJsonAccess},
    RawDataProcessor,
};

use std::rc::Rc;

use anyhow::{Ok, Result};
use hashbrown::HashSet;
use lazy_static::lazy_static;
use regex::Regex;
use simd_json::ValueAccess;

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawRoot {
    pub(crate) lang: usize,
    pub(crate) term: Symbol,
    pub(crate) sense_id: Option<Symbol>,
}

impl RawDataProcessor {
    // cf. https://en.wiktionary.org/wiki/Template:root. For now we skip
    // attempting to deal with multiple roots listed in a root template or
    // multiple root templates being listed. In both cases we just take the
    // first root term seen. If we discover it is common, we will handle it.
    pub(crate) fn process_json_root(
        &mut self,
        json_item: &WiktextractJson,
        lang: &str,
    ) -> Option<RawRoot> {
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

    fn process_json_root_template(
        &mut self,
        args: &WiktextractJson,
        lang: &str,
    ) -> Option<RawRoot> {
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
}

impl RawItems {
    pub(crate) fn impute_root_items(&self, ety_graph: &mut EtyGraph) -> Result<()> {
        let pb = progress_bar(self.n, "Imputing roots")?;
        let root_pos = Some(POS.get_expected_index("root")?);
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            if let Some(raw_root) = &item.raw_root
                                && !self.contains(raw_root.lang, raw_root.term)
                            {
                                let i = self.n + ety_graph.imputed_items.n;
                                let root = Rc::from(RawItem::new_imputed(
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

    fn impute_item_root_ety(
        &self,
        ety_graph: &mut EtyGraph,
        embeddings: &Embeddings,
        embedding: ItemEmbedding,
        item: &Rc<RawItem>,
    ) {
        if let Some(raw_root) = &item.raw_root
            && let Some(root_item) = self
                .get_disambiguated_item(embeddings, embedding, raw_root.lang, raw_root.term)
                .or_else(|| ety_graph.imputed_items.get(raw_root.lang, raw_root.term))
        {
            let mut visited_items: HashSet<Rc<RawItem>> =
                HashSet::from([Rc::clone(item), Rc::clone(&root_item)]);
            let mut current_item = Rc::clone(item);
            let mut item_embeddings = vec![embedding];
            while let Some(immediate_ety) = ety_graph.get_immediate_ety(&current_item) {
                // If there are multiple ety items but one has the same root, pick this.
                let ety_item = immediate_ety.items.iter().find(|ety_item| {
                    ety_item.raw_root.as_ref()
                        .and_then(|r| {
                            item_embeddings.push(embeddings.get(ety_item));
                            self
                                .get_disambiguated_item(embeddings, &item_embeddings, r.lang, r.term)
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
                // If the root or any previously visited item is encountered
                // again, don't impute anything, so we don't create or get
                // caught in a cycle.
                if visited_items.contains(&current_item) {
                    return;
                }
                visited_items.insert(Rc::clone(&current_item));
            }
            if current_item != root_item {
                ety_graph.add_ety(&current_item, EtyMode::Root, 0u8, &[Rc::clone(&root_item)]);
            }
        }
    }

    pub(crate) fn impute_root_etys(
        &self,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
    ) -> Result<()> {
        let pb = progress_bar(self.n, "Imputing root etys")?;
        for lang_map in self.term_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            let embedding = embeddings.get(item);
                            self.impute_item_root_ety(ety_graph, embeddings, embedding, item);
                            pb.inc(1);
                        }
                    }
                }
            }
        }
        pb.finish();
        Ok(())
    }
}
