use crate::{
    embeddings::Embeddings,
    ety_graph::EtyGraph,
    etymology_templates::{EtyMode, TemplateKind},
    items::{ItemId, RawItems, Retrieval},
    langterm::{Lang, LangTerm},
    progress_bar,
    string_pool::StringPool,
    wiktextract_json::{WiktextractJson, WiktextractJsonItem, WiktextractJsonValidStr},
};

use std::{mem, str::FromStr};

use anyhow::{anyhow, ensure, Ok, Result};
use simd_json::ValueAccess;
use std::collections::HashSet;

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
    pub(crate) langterms: Box<[LangTerm]>, // e.g. "en" "re-", "en" "do"
    pub(crate) mode: EtyMode,              // e.g. Prefix
    pub(crate) head: u8,                   // e.g. 1 (the index of "do")
}

impl RawEtyTemplate {
    fn new(langterm: LangTerm, mode: EtyMode) -> Self {
        Self {
            langterms: Box::from([langterm]),
            mode,
            head: 0,
        }
    }
}

fn process_derived_kind_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    mode: EtyMode,
) -> Option<RawEtyTemplate> {
    let ety_lang = args.get_valid_str("2")?;
    let ety_lang = Lang::try_from(ety_lang).ok()?;
    let ety_term = args.get_valid_str("3")?;
    let ety_langterm = ety_lang.new_langterm(string_pool, ety_term);
    Some(RawEtyTemplate::new(ety_langterm, mode))
}

fn process_abbrev_kind_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    mode: EtyMode,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_str("2")?;
    let ety_langterm = lang.new_langterm(string_pool, ety_term);
    Some(RawEtyTemplate::new(ety_langterm, mode))
}

fn process_prefix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_prefix = args.get_valid_str("2")?;
    let ety_prefix = format!("{ety_prefix}-");
    let ety_prefix = lang.new_langterm(string_pool, &ety_prefix);
    let ety_term = args.get_valid_str("3")?;
    let ety_term = lang.new_langterm(string_pool, ety_term);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_prefix, ety_term]),
        mode: EtyMode::Prefix,
        head: 1,
    })
}

fn process_suffix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_str("2")?;
    let ety_term = lang.new_langterm(string_pool, ety_term);
    let ety_suffix = args.get_valid_str("3")?;
    let ety_suffix = format!("-{ety_suffix}");
    let ety_suffix = lang.new_langterm(string_pool, &ety_suffix);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_term, ety_suffix]),
        mode: EtyMode::Suffix,
        head: 0,
    })
}

fn process_circumfix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_prefix = args.get_valid_str("2")?;
    let ety_term = args.get_valid_str("3")?;
    let ety_suffix = args.get_valid_str("4")?;

    let ety_term = lang.new_langterm(string_pool, ety_term);
    let ety_circumfix = format!("{ety_prefix}- -{ety_suffix}");
    let ety_circumfix = lang.new_langterm(string_pool, &ety_circumfix);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_term, ety_circumfix]),
        mode: EtyMode::Circumfix,
        head: 0,
    })
}

fn process_infix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_str("2")?;
    let ety_infix = args.get_valid_str("3")?;

    let ety_term = lang.new_langterm(string_pool, ety_term);
    let ety_infix = format!("-{ety_infix}-");
    let ety_infix = lang.new_langterm(string_pool, &ety_infix);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_term, ety_infix]),
        mode: EtyMode::Infix,
        head: 0,
    })
}

fn process_confix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_prefix = args.get_valid_str("2")?;
    let ety2 = args.get_valid_str("3")?;

    let ety_prefix = format!("{ety_prefix}-");
    let ety_prefix = lang.new_langterm(string_pool, &ety_prefix);
    if let Some(ety3) = args.get_valid_str("4") {
        let ety_term = lang.new_langterm(string_pool, ety2);
        let ety_suffix = format!("-{ety3}");
        let ety_suffix = lang.new_langterm(string_pool, &ety_suffix);
        return Some(RawEtyTemplate {
            langterms: Box::new([ety_prefix, ety_term, ety_suffix]),
            mode: EtyMode::Confix,
            head: 1,
        });
    }
    let ety_suffix = format!("-{ety2}");
    let ety_suffix = lang.new_langterm(string_pool, &ety_suffix);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_prefix, ety_suffix]),
        mode: EtyMode::Confix,
        head: 0, // no true head here, arbitrarily take first
    })
}

fn process_compound_kind_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    mode: EtyMode,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let mut n = 2;
    let mut ety_langterms = vec![];
    while let Some(ety_term) = args.get_valid_str(n.to_string().as_str()) {
        if let Some(ety_lang) = args.get_valid_str(format!("lang{n}").as_str()) {
            let ety_lang = Lang::try_from(ety_lang).ok()?;
            let ety_langterm = ety_lang.new_langterm(string_pool, ety_term);
            ety_langterms.push(ety_langterm);
        } else {
            let ety_langterm = lang.new_langterm(string_pool, ety_term);
            ety_langterms.push(ety_langterm);
        }
        n += 1;
    }
    if !ety_langterms.is_empty() {
        return Some(RawEtyTemplate {
            langterms: ety_langterms.into_boxed_slice(),
            mode,
            head: 0, // no true head here, arbitrarily take first
        });
    }
    None
}

pub(crate) fn validate_ety_template_lang(args: &WiktextractJson, lang: Lang) -> Result<()> {
    let item_lang = lang.code();
    let template_lang = args.get_valid_str("1").ok_or_else(|| {
        anyhow!("ety template does not contain valid \"1\" lang arg: it has args:\n{args}")
    })?;
    ensure!(template_lang == item_lang, "ety template \"1\" lang arg was {template_lang}, should have matched item lang {item_lang}");
    Ok(())
}

fn process_json_ety_template(
    string_pool: &mut StringPool,
    template: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let name = template.get_valid_str("name")?;
    let ety_mode = EtyMode::from_str(name).ok()?;
    let args = template.get("args")?;
    validate_ety_template_lang(args, lang).ok()?;
    match ety_mode.template_kind() {
        TemplateKind::Derived => process_derived_kind_json_template(string_pool, args, ety_mode),
        TemplateKind::Abbreviation => {
            process_abbrev_kind_json_template(string_pool, args, ety_mode, lang)
        }
        TemplateKind::Compound => match ety_mode {
            EtyMode::Prefix => process_prefix_json_template(string_pool, args, lang),
            EtyMode::Suffix => process_suffix_json_template(string_pool, args, lang),
            EtyMode::Circumfix => process_circumfix_json_template(string_pool, args, lang),
            EtyMode::Infix => process_infix_json_template(string_pool, args, lang),
            EtyMode::Confix => process_confix_json_template(string_pool, args, lang),
            _ => process_compound_kind_json_template(string_pool, args, ety_mode, lang),
        },
        _ => None,
    }
}

impl WiktextractJsonItem<'_> {
    pub(crate) fn get_etymology(
        &self,
        string_pool: &mut StringPool,
        lang: Lang,
    ) -> Option<RawEtymology> {
        let mut raw_ety_templates = vec![];
        if let Some(templates) = self.json.get_array("etymology_templates") {
            raw_ety_templates.reserve(templates.len());
            for template in templates {
                if let Some(raw_ety_template) =
                    process_json_ety_template(string_pool, template, lang)
                {
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
        let alt_term = self
            .json
            .get_array("senses")
            .and_then(|senses| senses.get(0))
            .and_then(|sense| {
                sense
                    .get_array("alt_of")
                    .or_else(|| sense.get_array("form_of"))
            })
            .and_then(|alt_list| alt_list.get(0))
            .and_then(|alt_obj| alt_obj.get_str("word"))?;
        let langterm = lang.new_langterm(string_pool, alt_term);
        let raw_ety_template = RawEtyTemplate::new(langterm, EtyMode::Form);
        raw_ety_templates.push(raw_ety_template);
        Some(raw_ety_templates.into())
    }
}

impl RawItems {
    pub(crate) fn get_ety_items_needing_embedding(
        &self,
        item: ItemId,
        raw_etymology: &RawEtymology,
    ) -> HashSet<ItemId> {
        let mut items_needing_embedding = HashSet::new();
        let mut parent_items = vec![item];

        for template in raw_etymology.templates.iter() {
            let mut has_ambiguous_child = false;
            let mut has_imputed_child = false;
            let mut next_parent_items = vec![];
            for &langterm in template.langterms.iter() {
                if let Some(ety_items) = self.get_dupes(langterm) {
                    if ety_items.len() > 1 {
                        // i.e. langterm is ambiguous
                        has_ambiguous_child = true;
                        for &ety_item in ety_items {
                            items_needing_embedding.insert(ety_item);
                        }
                    }
                    for &ety_item in ety_items {
                        next_parent_items.push(ety_item);
                    }
                } else {
                    has_imputed_child = true;
                }
            }
            if has_ambiguous_child || has_imputed_child {
                for &parent_item in &parent_items {
                    items_needing_embedding.insert(parent_item);
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
        item: ItemId,
        raw_etymology: &RawEtymology,
    ) -> Result<()> {
        let mut current_item = item; // for tracking possibly imputed items
        let mut next_item = item; // for tracking possibly imputed items
        let mut item_embeddings = vec![];
        for template in raw_etymology.templates.iter() {
            item_embeddings.push(embeddings.get(current_item)?);
            let mut ety_items = Vec::with_capacity(template.langterms.len());
            let mut confidences = Vec::with_capacity(template.langterms.len());
            let mut has_new_imputation = false;
            for &ety_langterm in template.langterms.iter() {
                let Retrieval {
                    item_id: ety_item,
                    confidence,
                    is_newly_imputed,
                    ..
                } =
                    self.get_or_impute_item(ety_graph, embeddings, &item_embeddings, ety_langterm)?;
                has_new_imputation = is_newly_imputed;
                if has_new_imputation {
                    if template.langterms.len() == 1 {
                        // This is a newly imputed term in a non-compound-kind template.
                        // We will use this newly imputed item as the item for the next
                        // template in the outer loop.
                        next_item = ety_item;
                    } else {
                        // This is a newly imputed item for a term in a
                        // compound-kind template. We won't bother trying to do
                        // convoluted ety link imputations for such cases at the
                        // moment. So we stop processing templates here.
                        return Ok(());
                    }
                }
                ety_items.push(ety_item);
                confidences.push(confidence);
            }
            ety_graph.add_ety(
                current_item,
                template.mode,
                template.head,
                &ety_items,
                &confidences,
            );
            // We keep processing templates until we hit the first one with no
            // imputation required.
            if !has_new_imputation {
                return Ok(());
            }
            current_item = next_item;
        }
        Ok(())
    }

    pub(crate) fn process_raw_etymologies(
        &mut self,
        embeddings: &Embeddings,
        ety_graph: &mut EtyGraph,
    ) -> Result<()> {
        let n = self.raw_templates.ety.len();
        let pb = progress_bar(n, "Processing etymologies")?;
        let raw_templates_ety = mem::take(&mut self.raw_templates.ety);
        for (item_id, ety) in raw_templates_ety {
            self.process_item_raw_etymology(embeddings, ety_graph, item_id, &ety)?;
            pb.inc(1);
        }
        pb.finish();
        Ok(())
    }
}
