use crate::{
    embeddings::Embeddings,
    etymology_templates::{EtyMode, TemplateKind},
    items::{ItemId, Items, Retrieval},
    langterm::LangTerm,
    languages::Lang,
    progress_bar,
    string_pool::StringPool,
    wiktextract_json::{Affix, WiktextractJson, WiktextractJsonItem, WiktextractJsonValidStr},
    HashSet,
};

use std::{mem, str::FromStr};

use anyhow::{anyhow, ensure, Ok, Result};
use simd_json::ValueAccess;

// models the basic info from a wiktionary etymology template
#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawEtyTemplate {
    pub(crate) langterms: Box<[LangTerm]>, // e.g. "en" "re-", "en" "do"
    pub(crate) mode: EtyMode,              // e.g. Prefix
    pub(crate) head: Option<u8>,           // e.g. 1 (the index of "do")
}

impl RawEtyTemplate {
    fn new(langterm: LangTerm, mode: EtyMode) -> Self {
        Self {
            langterms: Box::from([langterm]),
            mode,
            head: Some(0),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) enum ParsedRawEtyTemplate {
    Parsed(RawEtyTemplate),
    Skipped,
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawEtymology {
    pub(crate) templates: Box<[ParsedRawEtyTemplate]>,
}

impl From<Vec<ParsedRawEtyTemplate>> for RawEtymology {
    fn from(templates: Vec<ParsedRawEtyTemplate>) -> Self {
        Self {
            templates: templates.into_boxed_slice(),
        }
    }
}

fn process_derived_kind_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    mode: EtyMode,
) -> Option<RawEtyTemplate> {
    let mut ety_lang = args.get_valid_str("2")?;
    // Template::borrowed and its siblings allow specifying multiple ety langs
    // in a comma-separated list, e.g. {{bor|lv|sv,da,no|Gunnar}}. We take the
    // the first one in this case.
    ety_lang = ety_lang.split_once(',').map_or(ety_lang, |(el, _)| el);
    let ety_lang = Lang::from_str(ety_lang).ok()?;
    let ety_term = args.get_valid_term("3")?;
    let ety_langterm = ety_lang.new_langterm(string_pool, ety_term);
    Some(RawEtyTemplate::new(ety_langterm, mode))
}

fn process_abbrev_kind_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    mode: EtyMode,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_term("2")?;
    let ety_langterm = lang.new_langterm(string_pool, ety_term);
    Some(RawEtyTemplate::new(ety_langterm, mode))
}

fn process_prefix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_prefix = args.get_affix_term("2", &Affix::Prefix)?;
    let ety_prefix = lang.new_langterm(string_pool, &ety_prefix);
    let ety_term = args.get_valid_term("3")?;
    let ety_term = lang.new_langterm(string_pool, ety_term);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_prefix, ety_term]),
        mode: EtyMode::Prefix,
        head: Some(1),
    })
}

fn process_suffix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_term("2")?;
    let ety_term = lang.new_langterm(string_pool, ety_term);
    let ety_suffix = args.get_affix_term("3", &Affix::Suffix)?;
    let ety_suffix = lang.new_langterm(string_pool, &ety_suffix);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_term, ety_suffix]),
        mode: EtyMode::Suffix,
        head: Some(0),
    })
}

fn process_circumfix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_prefix = args.get_affix_term("2", &Affix::Prefix)?;
    let ety_term = args.get_valid_term("3")?;
    let ety_suffix = args.get_affix_term("4", &Affix::Suffix)?;

    let ety_term = lang.new_langterm(string_pool, ety_term);
    let ety_circumfix = format!("{ety_prefix} {ety_suffix}");
    let ety_circumfix = lang.new_langterm(string_pool, &ety_circumfix);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_term, ety_circumfix]),
        mode: EtyMode::Circumfix,
        head: Some(0),
    })
}

fn process_infix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_term("2")?;
    let ety_infix = args.get_affix_term("3", &Affix::Infix)?;

    let ety_term = lang.new_langterm(string_pool, ety_term);
    let ety_infix = lang.new_langterm(string_pool, &ety_infix);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_term, ety_infix]),
        mode: EtyMode::Infix,
        head: Some(0),
    })
}

fn process_confix_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_prefix = args.get_affix_term("2", &Affix::Prefix)?;
    let ety2 = args.get_valid_term("3")?;

    let ety_prefix = lang.new_langterm(string_pool, &ety_prefix);
    if let Some(ety_suffix) = args.get_affix_term("4", &Affix::Suffix) {
        let ety_term = lang.new_langterm(string_pool, ety2);
        let ety_suffix = lang.new_langterm(string_pool, &ety_suffix);
        return Some(RawEtyTemplate {
            langterms: Box::new([ety_prefix, ety_term, ety_suffix]),
            mode: EtyMode::Confix,
            head: Some(1),
        });
    }
    let ety_suffix = format!("-{ety2}");
    let ety_suffix = lang.new_langterm(string_pool, &ety_suffix);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_prefix, ety_suffix]),
        mode: EtyMode::Confix,
        head: None, // no true head here
    })
}

/// {{affix}} is a very general template. We try to narrow down to the specific
/// type of affixation.
fn affixation_kind(affixes: &[Affix], n_base_terms: usize) -> EtyMode {
    let n = affixes.len();
    if n_base_terms == n {
        // This is to handle the case of e.g. {{af|en|volley|ball}}.
        // This will allow us to record that there were multiple
        // ambiguous heads, whereas if we had recorded Affix we
        // wouldn't know without looking at the terms whether this
        // template included no affix terms or not. This is
        // important for EtyMode::has_ambiguous_head().
        return EtyMode::Compound;
    }

    if n == 3
        && affixes[0] == Affix::Base
        && affixes[1] == Affix::Infix
        && affixes[2] == Affix::Base
    {
        return EtyMode::Infix;
    }

    if n != 2 {
        return EtyMode::Affix;
    }

    match affixes[0] {
        Affix::Prefix => match affixes[1] {
            Affix::Suffix => EtyMode::Confix,
            Affix::Base => EtyMode::Prefix,
            _ => EtyMode::Affix,
        },
        Affix::Base => match affixes[1] {
            Affix::Suffix => EtyMode::Suffix,
            _ => EtyMode::Affix,
        },
        _ => EtyMode::Affix,
    }
}

fn process_compound_kind_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    mode: EtyMode,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let mut n = 2;
    let mut ety_langterms = vec![];
    let mut affixes = vec![];
    let mut head = 0;
    let mut n_base_terms = 0; // terms that aren't x-, -x, etc.
    while let Some(ety_term) = args.get_valid_term(n.to_string().as_str()) {
        // These compound-kind templates often have no true head (affix is the
        // most common of these templates, see that). We will take a head only
        // in the case where there is a single base (non-affix) term. So e.g.
        // {{af|en|pre-|date}} will have date as head, but
        // {{af|en|volley|ball}} will not have a head.
        affixes.push(if ety_term.starts_with('-') {
            if ety_term.ends_with('-') {
                Affix::Infix
            } else {
                Affix::Suffix
            }
        } else if ety_term.ends_with('-') {
            Affix::Prefix
        } else {
            n_base_terms += 1;
            head = n - 2;
            Affix::Base
        });
        if let Some(ety_lang) = args.get_valid_str(format!("lang{n}").as_str()) {
            let ety_lang = Lang::from_str(ety_lang).ok()?;
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
            mode: if mode == EtyMode::Affix {
                affixation_kind(&affixes, n_base_terms)
            } else {
                mode
            },
            head: (n_base_terms == 1).then_some(head), // see above
        });
    }
    None
}

fn process_vrddhi_kind_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    ety_mode: EtyMode,
) -> Option<RawEtyTemplate> {
    let ety_lang = args.get_valid_str("1")?;
    let ety_lang = Lang::from_str(ety_lang).ok()?;
    let ety_term = args.get_valid_term("2")?;
    let ety_langterm = ety_lang.new_langterm(string_pool, ety_term);
    Some(RawEtyTemplate::new(ety_langterm, ety_mode))
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
    let template_kind = ety_mode.template_kind();
    // vrddhi-kind templates are unusual in that their "1" arg is not the lang
    // of the term whose ety is being described. Therefore we avoid calling
    // validate_ety_template_lang() on them.
    if template_kind == Some(TemplateKind::Vrddhi) {
        return process_vrddhi_kind_json_template(string_pool, args, ety_mode);
    }
    validate_ety_template_lang(args, lang).ok()?;
    match template_kind {
        Some(TemplateKind::Derived) => {
            process_derived_kind_json_template(string_pool, args, ety_mode)
        }
        Some(TemplateKind::Abbreviation) => {
            process_abbrev_kind_json_template(string_pool, args, ety_mode, lang)
        }
        Some(TemplateKind::Compound) => match ety_mode {
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
    // Many ety sections contain a single {{m}} template and no others, and
    // consist only of "From {{m..." (or similar). This is to handle this case.
    fn get_single_mention_ety(&self, string_pool: &mut StringPool) -> Option<RawEtymology> {
        // Since we handle {{root}} separately, we still want to get etys where
        // there is say one {{root}} then one "From {{m..."
        let mut templates = self
            .json
            .get_array("etymology_templates")?
            .iter()
            .filter(|t| t.get_valid_str("name").is_some_and(|n| n != "root"));
        // i.e. we want exactly 1 non-root template
        let template = templates.next()?;
        templates.next().is_none().then_some(())?;
        let name = template.get_valid_str("name")?;
        matches!(name, "mention" | "m").then_some(())?;
        // $$ Previously we used this check to only allow "From {{m..." etys.
        // $$ But cf. eg. la penicillum, which has "Diminutive of {{m|la|pēniculus}}".
        // $$ So we'll just allow any single {{m}} template for now. Need to monitor
        // $$ this to see if it is too permissive. If it is, we will check for matches
        // $$ against a list of "From ", "Diminutive of ", etc. here.
        // self.json
        //     .get_valid_str("etymology_text")
        //     .is_some_and(|et| et.starts_with("From "))
        //     .then_some(())?;
        let args = template.get("args")?;
        let mention_lang = args.get_valid_str("1")?;
        let mention_term = args.get_valid_term("2")?;
        let mention_lang = Lang::from_str(mention_lang).ok()?;
        let mention_langterm = mention_lang.new_langterm(string_pool, mention_term);
        let ety = RawEtyTemplate::new(mention_langterm, EtyMode::Mention);
        Some(vec![ParsedRawEtyTemplate::Parsed(ety)].into())
    }

    fn get_standard_ety(&self, string_pool: &mut StringPool, lang: Lang) -> Option<RawEtymology> {
        let templates = self.json.get_array("etymology_templates")?;
        let mut raw_ety_templates = Vec::with_capacity(templates.len());
        for template in templates {
            if let Some(raw_ety_template) = process_json_ety_template(string_pool, template, lang) {
                raw_ety_templates.push(ParsedRawEtyTemplate::Parsed(raw_ety_template));
            } else {
                raw_ety_templates.push(ParsedRawEtyTemplate::Skipped);
            }
        }
        (!raw_ety_templates.is_empty()).then(|| raw_ety_templates.into())
    }

    // if no ety section or no templates, as a fallback we see if term
    // is listed as a "form_of" (item.senses[0].form_of[0].word)
    // or "alt_of" (item.senses[0].alt_of[0].word) another term.
    // e.g. "happenin'" is listed as an alt_of of "happening".
    fn get_form_ety(&self, string_pool: &mut StringPool, lang: Lang) -> Option<RawEtymology> {
        let alt_term = self
            .json
            .get_array("senses")
            .and_then(|senses| senses.first())
            .and_then(|sense| {
                sense
                    .get_array("alt_of")
                    .or_else(|| sense.get_array("form_of"))
            })
            .and_then(|alt_list| alt_list.first())
            .and_then(|alt_obj| alt_obj.get_str("word"))?;
        let langterm = lang.new_langterm(string_pool, alt_term);
        let ety = RawEtyTemplate::new(langterm, EtyMode::Form);
        Some(vec![ParsedRawEtyTemplate::Parsed(ety)].into())
    }

    pub(crate) fn get_etymology(
        &self,
        string_pool: &mut StringPool,
        lang: Lang,
    ) -> Option<RawEtymology> {
        self.get_single_mention_ety(string_pool)
            .or_else(|| self.get_standard_ety(string_pool, lang))
            .or_else(|| self.get_form_ety(string_pool, lang))
    }
}

impl Items {
    pub(crate) fn get_ety_items_needing_embedding(
        &self,
        item: ItemId,
        raw_etymology: &RawEtymology,
    ) -> HashSet<ItemId> {
        let mut items_needing_embedding = HashSet::default();
        let mut parent_items = vec![item];

        for template in raw_etymology.templates.iter().filter_map(|t| match t {
            ParsedRawEtyTemplate::Parsed(template) => Some(template),
            ParsedRawEtyTemplate::Skipped => None,
        }) {
            let mut has_ambiguous_child = false;
            let mut has_imputed_child = false;
            let mut next_parent_items = vec![];
            for &langterm in &*template.langterms {
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

    // For now we'll just take the first template, unless imputation is requred,
    // in which case we impute a chain of relations until the first known item
    // is hit. But cf. notes.md. Only to be called once all json items have been
    // processed into items.
    fn process_item_raw_etymology(
        &mut self,
        embeddings: &Embeddings,
        item: ItemId,
        raw_etymology: &RawEtymology,
    ) -> Result<()> {
        let mut current_item = item; // for tracking possibly imputed items
        let mut next_item = item; // for tracking possibly imputed items
        let mut item_embeddings = vec![];
        let mut imputation_chain_in_progress = false;
        for template in &*raw_etymology.templates {
            match template {
                ParsedRawEtyTemplate::Parsed(template) => {
                    item_embeddings.push(embeddings.get(self.get(current_item), current_item)?);
                    let mut ety_items = Vec::with_capacity(template.langterms.len());
                    let mut confidences = Vec::with_capacity(template.langterms.len());
                    for &ety_langterm in &*template.langterms {
                        let Retrieval {
                            item_id: ety_item,
                            confidence,
                        } = self.get_or_impute_item(
                            embeddings,
                            &item_embeddings,
                            item,
                            ety_langterm,
                        )?;
                        if self.get(ety_item).is_imputed() {
                            if template.langterms.len() == 1
                            // $$$ It would be better to have language timespan data and
                            // only impute connection if parent timespan precedes child
                            // timespan. Going based on genetic descent makes us miss
                            // out on common connections like e.g. Middle English <
                            // Latin.
                            && self
                                .get(current_item)
                                .lang()
                                .descends_from(self.get(ety_item).lang())
                            {
                                // This is an imputed term in a non-compound-kind template.
                                // We will use this imputed item as the item for the next
                                // template in the outer loop.
                                imputation_chain_in_progress = true;
                                next_item = ety_item;
                            } else {
                                // This is an imputed item for a term in a
                                // compound-kind template. We won't bother trying to do
                                // convoluted ety link imputations for such cases at the
                                // moment. So we stop processing templates here.
                                return Ok(());
                            }
                        } else {
                            imputation_chain_in_progress = false;
                        }
                        ety_items.push(ety_item);
                        confidences.push(confidence);
                    }

                    self.graph.add_ety(
                        current_item,
                        template.mode,
                        template.head,
                        &ety_items,
                        &confidences,
                    );

                    if !imputation_chain_in_progress {
                        return Ok(());
                    }
                    current_item = next_item;
                }
                ParsedRawEtyTemplate::Skipped => {
                    if imputation_chain_in_progress {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn process_raw_etymologies(&mut self, embeddings: &Embeddings) -> Result<()> {
        let n = self.raw_templates.ety.len();
        let pb = progress_bar(n, "Processing etymologies")?;
        let raw_templates_ety = mem::take(&mut self.raw_templates.ety);
        for (item_id, ety) in raw_templates_ety {
            self.process_item_raw_etymology(embeddings, item_id, &ety)?;
            pb.inc(1);
        }
        pb.finish();
        Ok(())
    }
}
