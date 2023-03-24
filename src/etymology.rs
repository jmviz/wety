use crate::{
    embeddings::Embeddings,
    ety_graph::EtyGraph,
    etymology_templates::{EtyMode, TemplateKind},
    lang_phf::LANG_CODE2NAME,
    progress_bar,
    raw_items::{RawItem, RawItems, Retrieval},
    string_pool::Symbol,
    wiktextract_json::{WiktextractJson, WiktextractJsonAccess},
    RawDataProcessor,
};

use std::{rc::Rc, str::FromStr};

use anyhow::{Ok, Result};
use hashbrown::HashSet;
use simd_json::ValueAccess;

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawEtymology {
    pub(crate) templates: Box<[RawEtyTemplate]>,
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
pub(crate) struct RawEtyTemplate {
    pub(crate) langs: Box<[usize]>,  // e.g. "en", "en"
    pub(crate) terms: Box<[Symbol]>, // e.g. "re-", "do"
    pub(crate) mode: EtyMode,        // e.g. Prefix
    pub(crate) head: u8,             // e.g. 1 (the index of "do")
}

impl RawEtyTemplate {
    fn new(lang: usize, term: Symbol, mode: EtyMode) -> Self {
        Self {
            langs: Box::new([lang]),
            terms: Box::new([term]),
            mode,
            head: 0,
        }
    }
}

impl RawDataProcessor {
    fn process_derived_kind_json_template(
        &mut self,
        args: &WiktextractJson,
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
        args: &WiktextractJson,
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

    fn process_prefix_json_template(
        &mut self,
        args: &WiktextractJson,
        lang: &str,
    ) -> Option<RawEtyTemplate> {
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

    fn process_suffix_json_template(
        &mut self,
        args: &WiktextractJson,
        lang: &str,
    ) -> Option<RawEtyTemplate> {
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
        args: &WiktextractJson,
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

    fn process_infix_json_template(
        &mut self,
        args: &WiktextractJson,
        lang: &str,
    ) -> Option<RawEtyTemplate> {
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

    fn process_confix_json_template(
        &mut self,
        args: &WiktextractJson,
        lang: &str,
    ) -> Option<RawEtyTemplate> {
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
        args: &WiktextractJson,
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
        template: &WiktextractJson,
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

    pub(crate) fn process_json_ety(
        &mut self,
        json_item: &WiktextractJson,
        lang: &str,
    ) -> Option<RawEtymology> {
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
}

impl RawItems {
    pub(crate) fn get_ety_items_needing_embedding(
        &self,
        item: &Rc<RawItem>,
        raw_etymology: &RawEtymology,
    ) -> HashSet<Rc<RawItem>> {
        let mut items_needing_embedding = HashSet::new();
        let mut parent_items = vec![Rc::clone(item)];

        for template in raw_etymology.templates.iter() {
            let mut has_ambiguous_child = false;
            let mut has_imputed_child = false;
            let mut next_parent_items = vec![];
            for (&lang, &term) in template.langs.iter().zip(template.terms.iter()) {
                if let Some(ety_items) = self.get_all_langterm_ids(lang, term) {
                    if ety_items.len() > 1 {
                        // i.e. (lang, term) is ambiguous
                        has_ambiguous_child = true;
                        for ety_item in &ety_items {
                            items_needing_embedding.insert(Rc::clone(ety_item));
                        }
                    }
                    for ety_item in &ety_items {
                        next_parent_items.push(Rc::clone(ety_item));
                    }
                } else {
                    has_imputed_child = true;
                }
            }
            if has_ambiguous_child || has_imputed_child {
                for parent_item in &parent_items {
                    items_needing_embedding.insert(Rc::clone(parent_item));
                }
            }
            parent_items = next_parent_items;
        }
        items_needing_embedding
    }

    // For now we'll just take the first template. But cf. notes.md.
    // Only to be called once all json items have been processed into items.
    fn process_item_raw_etymology(
        &self,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
        item: &Rc<RawItem>,
    ) {
        if item.raw_etymology.is_none() {
            return; // don't add anything to ety_graph if no valid raw ety templates
        }
        let mut current_item = Rc::clone(item); // for tracking possibly imputed items
        let mut next_item = Rc::clone(item); // for tracking possibly imputed items
        let mut item_embeddings = vec![];
        for template in item.raw_etymology.as_ref().unwrap().templates.iter() {
            item_embeddings.push(embeddings.get(&current_item));
            let mut ety_items = Vec::with_capacity(template.terms.len());
            let mut confidences = Vec::with_capacity(template.terms.len());
            let mut has_new_imputation = false;
            for (&ety_lang, &ety_term) in template.langs.iter().zip(template.terms.iter()) {
                let Retrieval {
                    item: ety_item,
                    confidence,
                    is_newly_imputed,
                } = self.get_or_impute_item(
                    ety_graph,
                    embeddings,
                    &item_embeddings,
                    ety_lang,
                    ety_term,
                );
                has_new_imputation = is_newly_imputed;
                if has_new_imputation {
                    if template.terms.len() == 1 {
                        // This is a newly imputed term in a non-compound-kind template.
                        // We will use this newly imputed item as the item for the next
                        // template in the outer loop.
                        next_item = Rc::clone(&ety_item);
                    } else {
                        // This is a newly imputed item for a term in a
                        // compound-kind template. We won't bother trying to do
                        // convoluted ety link imputations for such cases at the
                        // moment. So we stop processing templates here.
                        return;
                    }
                }
                ety_items.push(Rc::clone(&ety_item));
                confidences.push(confidence);
            }
            ety_graph.add_ety(
                &current_item,
                template.mode,
                template.head,
                &ety_items,
                &confidences,
            );
            // We keep processing templates until we hit the first one with no
            // imputation required.
            if !has_new_imputation {
                return;
            }
            current_item = Rc::clone(&next_item);
        }
    }

    pub(crate) fn process_raw_etymologies(
        &self,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
    ) -> Result<()> {
        let pb = progress_bar(self.n, "Processing etymologies")?;
        for lang_map in self.langterm_map.values() {
            for ety_map in lang_map.values() {
                for pos_map in ety_map.values() {
                    for gloss_map in pos_map.values() {
                        for item in gloss_map.values() {
                            self.process_item_raw_etymology(embeddings, ety_graph, item);
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
