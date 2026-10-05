use crate::{
    embeddings::{self, Embeddings},
    etymon,
    items::Items,
    resolve::{Citation, Relation, Resolution},
    progress_bar,
    wiktextract_json::{
        Affix, WiktextractJson, WiktextractJsonItem, WiktextractJsonValidStr, affix_by_hyphens,
        affix_type, clean_template_term, modifier, split_lang_prefix, split_modifiers,
    },
    HashSet,
};
use wety_core::{
    pos::Pos,
    etymology_templates::{EtyMode, TemplateKind},
    items::ItemId,
    langterm::LangTerm,
    languages::Lang,
    string_pool::StringPool,
};

use std::{mem, str::FromStr};

use anyhow::{anyhow, ensure, Ok, Result};
use simd_json::ValueAccess;

// models the basic info from a wiktionary etymology template
#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawEtyTemplate {
    pub(crate) langterms: Box<[LangTerm]>, // e.g. "en" "re-", "en" "do"
    // The {{etymon}} ids naming each langterm's etymology, e.g. "mother" in
    // "enm:moder<id:mother>". Empty for the classic templates.
    pub(crate) ids: Box<[Option<Box<str>>]>,
    // The gloss given for each langterm (t=, <t:...>), where given: it glosses
    // the cited term itself, unlike the citing item's own glosses.
    pub(crate) glosses: Box<[Option<Box<str>>]>,
    // Whether this template is known to give the etymology of the previous
    // template's single parent (an {{etymon}} inline etymology), rather than
    // only presumed to (the next classic template).
    pub(crate) continues: bool,
    // For compound-kind templates, each part's affix type, and whether the
    // mode was inferred from them (as for {{af}}), so that both can be
    // revised once the parts are known items (see revise_affixes).
    pub(crate) affixes: Box<[Affix]>,
    pub(crate) refine_mode: bool,
    pub(crate) mode: EtyMode,              // e.g. Prefix
    pub(crate) head: Option<u8>,           // e.g. 1 (the index of "do")
}

impl RawEtyTemplate {
    fn with_id_and_gloss(mut self, id: Option<&str>, gloss: Option<&str>) -> Self {
        self.ids = Box::new([id.map(Box::from)]);
        self.glosses = Box::new([gloss.map(Box::from)]);
        self
    }

    fn new(langterm: LangTerm, mode: EtyMode) -> Self {
        Self {
            langterms: Box::from([langterm]),
            ids: Box::default(),
            glosses: Box::default(),
            continues: false,
            affixes: Box::default(),
            refine_mode: false,
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
    let (id, gloss) = term_id_and_gloss(args, "3", &["t", "gloss", "5"]);
    Some(RawEtyTemplate::new(ety_langterm, mode).with_id_and_gloss(id, gloss))
}

// The id (id=, <id:...>) and gloss (t= etc., <t:...>) a template gives for
// the term in arg `key`.
fn term_id_and_gloss<'a>(
    args: &'a WiktextractJson,
    key: &str,
    gloss_keys: &[&str],
) -> (Option<&'a str>, Option<&'a str>) {
    let modifiers = args.get_str(key).map(|arg| split_modifiers(arg).1).unwrap_or_default();
    let id = modifier(&modifiers, "id").or_else(|| args.get_valid_str("id"));
    let gloss = modifier(&modifiers, "t")
        .or_else(|| modifier(&modifiers, "gloss"))
        .or_else(|| gloss_keys.iter().find_map(|k| args.get_valid_str(k)));
    (id.filter(|s| !s.is_empty()), gloss.filter(|s| !s.is_empty()))
}

fn process_abbrev_kind_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    mode: EtyMode,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_term("2")?;
    let ety_langterm = lang.new_langterm(string_pool, ety_term);
    let (id, gloss) = term_id_and_gloss(args, "2", &["t", "gloss", "4"]);
    Some(RawEtyTemplate::new(ety_langterm, mode).with_id_and_gloss(id, gloss))
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
        ids: Box::default(),
        glosses: Box::default(),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
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
        ids: Box::default(),
        glosses: Box::default(),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
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
        ids: Box::default(),
        glosses: Box::default(),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
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
        ids: Box::default(),
        glosses: Box::default(),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
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
            ids: Box::default(),
            glosses: Box::default(),
            continues: false,
            affixes: Box::default(),
            refine_mode: false,
            mode: EtyMode::Confix,
            head: Some(1),
        });
    }
    let ety_suffix = format!("-{ety2}");
    let ety_suffix = lang.new_langterm(string_pool, &ety_suffix);
    Some(RawEtyTemplate {
        langterms: Box::new([ety_prefix, ety_suffix]),
        ids: Box::default(),
        glosses: Box::default(),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
        mode: EtyMode::Confix,
        head: None, // no true head here
    })
}

/// {{affix}} is a very general template. We try to narrow down to the specific
/// type of affixation.
pub(crate) fn affixation_kind(affixes: &[Affix], n_base_terms: usize) -> EtyMode {
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

// The parts of compound-kind templates take per-part params numbered from 1
// (lang1=, type1= for arg "2"), or the same as inline modifiers (<lang:>,
// <type:>), and may be prefixed with a language code (grc:χρῶμα). See
// https://en.wiktionary.org/wiki/Template:affix.
fn process_compound_kind_json_template(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    mode: EtyMode,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let mut ety_langterms = vec![];
    let mut ids = vec![];
    let mut glosses = vec![];
    let mut affixes = vec![];
    let mut head = 0;
    let mut n_base_terms = 0; // terms that aren't x-, -x, etc.
    for part in 1.. {
        let Some(arg) = args.get_valid_str((part + 1).to_string().as_str()) else {
            break;
        };
        let (arg, modifiers) = split_modifiers(arg);
        let (prefix_lang, arg) = match split_lang_prefix(arg)
            .and_then(|(code, term)| Some((Lang::from_str(code).ok()?, term)))
        {
            Some((prefix_lang, term)) => (Some(prefix_lang), term),
            None => (None, arg),
        };
        let ety_term = clean_template_term(arg);
        if ety_term.is_empty() || ety_term == "-" {
            break;
        }
        let part_lang = match modifier(&modifiers, "lang")
            .or_else(|| args.get_valid_str(format!("lang{part}").as_str()))
        {
            Some(code) => Lang::from_str(code).ok()?,
            None => prefix_lang.unwrap_or(lang),
        };
        // These compound-kind templates often have no true head (affix is the
        // most common of these templates, see that). We will take a head only
        // in the case where there is a single base (non-affix) term. So e.g.
        // {{af|en|pre-|date}} will have date as head, but
        // {{af|en|volley|ball}} will not have a head.
        let affix = modifier(&modifiers, "type")
            .or_else(|| args.get_valid_str(format!("type{part}").as_str()))
            .and_then(affix_type)
            .unwrap_or_else(|| affix_by_hyphens(ety_term));
        if affix == Affix::Base {
            n_base_terms += 1;
            head = ety_langterms.len();
        }
        affixes.push(affix);
        ety_langterms.push(part_lang.new_langterm(string_pool, ety_term));
        let part_param = |name: &str| {
            modifier(&modifiers, name)
                .or_else(|| args.get_valid_str(format!("{name}{part}").as_str()))
                .filter(|s| !s.is_empty())
                .map(Box::from)
        };
        ids.push(part_param("id"));
        glosses.push(part_param("t").or_else(|| part_param("gloss")));
    }
    if !ety_langterms.is_empty() {
        return Some(RawEtyTemplate {
            langterms: ety_langterms.into_boxed_slice(),
            ids: ids.into_boxed_slice(),
            glosses: glosses.into_boxed_slice(),
            continues: false,
            refine_mode: mode == EtyMode::Affix,
            affixes: affixes.clone().into_boxed_slice(),
            mode: if mode == EtyMode::Affix {
                affixation_kind(&affixes, n_base_terms)
            } else {
                mode
            },
            // see above
            head: (n_base_terms == 1).then(|| u8::try_from(head).ok()).flatten(),
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
    // The args of the entry's {{etymon}} (or {{ety}}) template for its own
    // language. A page section rarely has more than one; we take the first.
    fn get_etymon_args(&self, lang: Lang) -> Option<&WiktextractJson<'_>> {
        self.json
            .get_array("etymology_templates")?
            .iter()
            .filter(|t| t.get_valid_str("name").is_some_and(|n| matches!(n, "etymon" | "ety")))
            .filter_map(|t| t.get("args"))
            .find(|args| validate_ety_template_lang(args, lang).is_ok())
    }

    /// The ids this entry declares, by which references can name it: its
    /// {{etymon}} id=, its {{etymid}} ids and its senses' {{senseid}}s
    /// (which Wiktextract prefixes with the language code).
    pub(crate) fn get_ids(&self, lang: Lang) -> Vec<&str> {
        let mut ids: Vec<&str> = self
            .get_etymon_args(lang)
            .and_then(|args| args.get_valid_str("id"))
            .into_iter()
            .collect();
        for template in self.json.get_array("etymology_templates").into_iter().flatten() {
            if template.get_valid_str("name") == Some("etymid")
                && let Some(args) = template.get("args")
                && args.get_valid_str("1") == Some(lang.code())
                && let Some(id) = args.get_valid_str("2")
            {
                ids.push(id);
            }
        }
        for sense in self.json.get_array("senses").into_iter().flatten() {
            for id in sense.get_array("senseid").into_iter().flatten().filter_map(|id| id.as_str()) {
                let id = id
                    .split_once(':')
                    .filter(|(code, _)| *code == lang.code())
                    .map_or(id, |(_, id)| id);
                if !id.is_empty() {
                    ids.push(id);
                }
            }
        }
        ids
    }

    // An entry may give its etymology in {{etymon}}, in the classic templates
    // ({{inh}}, {{bor}}, {{af}}, ...) or in both, usually as an etymology tree
    // atop a prose etymology. {{etymon}} is preferred: it names the immediate
    // parent(s) explicitly, while the first classic template is only usually
    // the parent. When it names a single parent and gives no inline etymology
    // for it, the classic templates after the one naming that same parent
    // continue the chain (used if the parent has to be imputed). If {{etymon}}
    // has no usable parent (e.g. only an id, a tree or a :root), the classic
    // templates are used as before.
    fn get_etymon_ety(&self, string_pool: &mut StringPool, lang: Lang) -> Option<RawEtymology> {
        let args = self.get_etymon_args(lang)?;
        let mut templates = etymon::parse_etymon_args(string_pool, args, lang);
        if templates.is_empty() {
            return None;
        }
        if let [ParsedRawEtyTemplate::Parsed(parent)] = templates.as_slice()
            && parent.langterms.len() == 1
            && let Some(standard) = self.get_standard_ety(string_pool, lang)
        {
            let parent = parent.langterms[0];
            let continuation = Vec::from(standard.templates)
                .into_iter()
                .skip_while(|t| {
                    !matches!(t, ParsedRawEtyTemplate::Parsed(c) if *c.langterms == [parent])
                })
                .skip(1);
            templates.extend(continuation);
        }
        Some(templates.into())
    }

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
            // An etymology that opens as unknown or uncertain ({{unk}},
            // {{unc}}) goes on to conjecture or compare ("Perhaps from ...",
            // "Compare ..."): no template after it names the parent.
            if template
                .get_valid_str("name")
                .is_some_and(|name| matches!(name, "unk" | "unknown" | "unc" | "uncertain"))
                && !raw_ety_templates
                    .iter()
                    .any(|t| matches!(t, ParsedRawEtyTemplate::Parsed(_)))
            {
                break;
            }
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
        self.get_etymon_ety(string_pool, lang)
            .or_else(|| self.get_single_mention_ety(string_pool))
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
                if let Some(ety_items) = self.candidates(langterm) {
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

    // The first parsed template gives the item's parent(s). If that is a
    // single parent missing from the data (and so imputed), the next template
    // is taken to give the imputed parent's own parent, and so on, as long as
    // it is known to (an {{etymon}} inline etymology) or the chain stays within
    // one line of descent (e.g. en < enm < ang). $$ Language dates would allow
    // the latter for e.g. Middle English < Latin too. Leading templates that
    // could not be parsed are passed over.
    fn process_item_raw_etymology(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        item: ItemId,
        raw_etymology: &RawEtymology,
    ) -> Result<()> {
        let templates = &*raw_etymology.templates;
        let mut current_item = item;
        let mut item_embeddings = vec![];
        let mut first = templates
            .iter()
            .position(|t| matches!(t, ParsedRawEtyTemplate::Parsed(_)));
        while let Some(i) = first {
            let ParsedRawEtyTemplate::Parsed(template) = &templates[i] else {
                break;
            };
            item_embeddings.push(embeddings.get(self.get(current_item), current_item)?);
            // What the etymology gives as a single parent's own source: the next
            // template, if it is known to continue the chain or stays in the
            // parent's line of descent (not e.g. "from French X or Spanish Y").
            let next = match (&template.langterms[..], templates.get(i + 1)) {
                ([parent], Some(ParsedRawEtyTemplate::Parsed(next))) => match &next.langterms[..] {
                    [source]
                        if next.continues
                            || parent.lang.ety2non().descends_from(source.lang.ety2non()) =>
                    {
                        Some(*source)
                    }
                    _ => None,
                },
                _ => None,
            };
            let mut ety_items = Vec::with_capacity(template.langterms.len());
            let mut confidences = Vec::with_capacity(template.langterms.len());
            for (n, &ety_langterm) in template.langterms.iter().enumerate() {
                let citation = Citation {
                    langterm: ety_langterm,
                    id: template.ids.get(n).and_then(Option::as_deref),
                    gloss: template.glosses.get(n).and_then(Option::as_deref),
                    citer: current_item,
                    relation: Relation::Parent { next },
                };
                let (ety_item, confidence) =
                    match self.resolve(string_pool, embeddings, &item_embeddings, &citation)? {
                        Resolution::Found { item, confidence } => (item, confidence),
                        Resolution::Missing => (
                            self.impute_item(ety_langterm, item),
                            embeddings::SIMILARITY_THRESHOLD,
                        ),
                    };
                ety_items.push(ety_item);
                confidences.push(confidence);
            }
            let (mode, head) = self.revise_affixes(template, &ety_items);
            if !confidences.iter().all(|&c| c >= embeddings::SIMILARITY_THRESHOLD)
                || !self.set_own_ety(current_item, mode, head, &ety_items, &confidences)
            {
                return Ok(());
            }
            // Continue with an imputed single parent's own etymology?
            let [parent] = ety_items[..] else {
                return Ok(());
            };
            let next = i + 1;
            let continues = self.get(parent).is_imputed()
                && match templates.get(next) {
                    Some(ParsedRawEtyTemplate::Parsed(next)) => {
                        next.continues
                            || self
                                .get(current_item)
                                .lang()
                                .descends_from(self.get(parent).lang())
                    }
                    _ => false,
                };
            if !continues {
                return Ok(());
            }
            current_item = parent;
            first = Some(next);
        }
        Ok(())
    }

    // Set an item's ety from its own etymology section (or, for an imputed
    // item, from the section of an item citing it). A Descendants list
    // may already have given it a parent; the item's own section is preferred,
    // except where that parent is more specific (it descends from the parent
    // named here, e.g. Old French between Latin and English), where the item
    // would trade an entry for a term missing from the data, or where the
    // section gives only a mention or a form-of link. Returns whether the ety
    // was set.
    fn set_own_ety(
        &mut self,
        item: ItemId,
        mode: EtyMode,
        head: Option<u8>,
        ety_items: &[ItemId],
        confidences: &[f32],
    ) -> bool {
        if let Some(existing) = self.graph.immediate_ety(item) {
            // An imputed item is shared by all citations of its term; the first
            // to give it an ety keeps it.
            if self.get(item).is_imputed() {
                return false;
            }
            let more_specific = match (&existing.items[..], ety_items) {
                ([old], [new]) => old != new && self.is_ancestor(*new, *old),
                _ => false,
            };
            let loses_entry = ety_items.iter().any(|&e| self.get(e).is_imputed())
                && existing.items.iter().all(|&e| !self.get(e).is_imputed());
            // a lone {{m}} or a form-of sense, used when there is no
            // etymology proper, is weaker evidence than a Descendants list
            let weak = matches!(mode, EtyMode::Mention | EtyMode::Form);
            if more_specific || loses_entry || weak {
                return false;
            }
            self.graph.remove_ety(item);
        }
        self.graph.add_ety(item, mode, head, ety_items, confidences);
        self.own_ety.insert(item);
        true
    }

    // A part read as a prefix by its final hyphen may be a root (PIE *peh₂-
    // in *ph₂tḗr = *peh₂- + *-tḗr), which is a base, not an affix. Module:etymon
    // tells these apart by the part's entry; so do we, once parts are items.
    fn revise_affixes(&self, template: &RawEtyTemplate, ety_items: &[ItemId]) -> (EtyMode, Option<u8>) {
        let is_root = |item: ItemId| {
            self.get(item)
                .pos()
                .is_some_and(|pos| pos.contains(&Pos::root_pos()))
        };
        if template.affixes.len() != ety_items.len()
            || !template
                .affixes
                .iter()
                .zip(ety_items)
                .any(|(&affix, &item)| affix == Affix::Prefix && is_root(item))
        {
            return (template.mode, template.head);
        }
        let affixes: Vec<Affix> = template
            .affixes
            .iter()
            .zip(ety_items)
            .map(|(&affix, &item)| {
                if affix == Affix::Prefix && is_root(item) {
                    Affix::Base
                } else {
                    affix
                }
            })
            .collect();
        let bases: Vec<usize> = (0..affixes.len())
            .filter(|&k| affixes[k] == Affix::Base)
            .collect();
        let mode = if template.refine_mode {
            affixation_kind(&affixes, bases.len())
        } else {
            template.mode
        };
        let head = match bases[..] {
            [k] => u8::try_from(k).ok(),
            _ => None,
        };
        (mode, head)
    }

    // Whether `ancestor` is among `item`'s etymological ancestors.
    fn is_ancestor(&self, ancestor: ItemId, item: ItemId) -> bool {
        let mut seen = HashSet::default();
        let mut stack = vec![item];
        while let Some(i) = stack.pop() {
            if let Some(ety) = self.graph.immediate_ety(i) {
                for parent in ety.items {
                    if parent == ancestor {
                        return true;
                    }
                    if seen.insert(parent) {
                        stack.push(parent);
                    }
                }
            }
        }
        false
    }

    pub(crate) fn process_raw_etymologies(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
    ) -> Result<()> {
        let n = self.raw_templates.ety.len();
        let pb = progress_bar(n, "Processing etymologies")?;
        // in id order, so that the result does not depend on hash-map order
        let mut raw_templates_ety: Vec<_> = mem::take(&mut self.raw_templates.ety).into_iter().collect();
        raw_templates_ety.sort_unstable_by_key(|(item_id, _)| *item_id);
        for (item_id, ety) in raw_templates_ety {
            self.process_item_raw_etymology(string_pool, embeddings, item_id, &ety)?;
            pb.inc(1);
        }
        pb.finish();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::to_borrowed_value;

    // (mode, [term]) for each template in the chain; None for a skipped one
    fn chain(json: &str) -> Vec<Option<(EtyMode, Vec<String>)>> {
        chain_with("en", json, false)
    }

    // as chain(), with each term prefixed by its language code
    fn chain_in(lang: &str, json: &str) -> Vec<Option<(EtyMode, Vec<String>)>> {
        chain_with(lang, json, true)
    }

    fn chain_with(lang: &str, json: &str, codes: bool) -> Vec<Option<(EtyMode, Vec<String>)>> {
        let mut bytes = json.as_bytes().to_vec();
        let json = to_borrowed_value(&mut bytes).unwrap();
        let mut string_pool = StringPool::new();
        let lang = Lang::from_str(lang).unwrap();
        WiktextractJsonItem { json }
            .get_etymology(&mut string_pool, lang)
            .map_or_else(Vec::new, |ety| Vec::from(ety.templates))
            .into_iter()
            .map(|t| match t {
                ParsedRawEtyTemplate::Parsed(t) => Some((
                    t.mode,
                    t.langterms
                        .iter()
                        .map(|lt| {
                            let term = lt.term.resolve(&string_pool);
                            if codes && lt.lang != lang {
                                format!("{}:{term}", lt.lang.code())
                            } else {
                                term.to_owned()
                            }
                        })
                        .collect(),
                )),
                ParsedRawEtyTemplate::Skipped => None,
            })
            .collect()
    }

    fn step(mode: EtyMode, term: &str) -> Option<(EtyMode, Vec<String>)> {
        Some((mode, vec![term.to_owned()]))
    }

    #[test]
    fn etymon_is_continued_by_the_classic_chain_after_its_parent() {
        // en "free"
        let got = chain(
            r#"{"etymology_templates": [
                {"name": "etymon", "args": {"1": "en", "id": "adjective", "2": ":inh", "3": "enm:fre", "tree": "1"}},
                {"name": "inh", "args": {"1": "en", "2": "enm", "3": "fre"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "frēo"}},
                {"name": "inh", "args": {"1": "en", "2": "gmw-pro", "3": "*frī"}},
                {"name": "cog", "args": {"1": "sco", "2": "fre"}}
            ]}"#,
        );
        assert_eq!(
            got,
            [
                step(EtyMode::Inherited, "fre"),
                step(EtyMode::Inherited, "frēo"),
                step(EtyMode::Inherited, "frī"),
                None,
            ]
        );
    }

    #[test]
    fn etymon_wins_over_disagreeing_classic_templates() {
        let got = chain(
            r#"{"etymology_templates": [
                {"name": "etymon", "args": {"1": "en", "2": ":lbor", "3": "la-new:encyclopaedīa<id:encyclopedia>"}},
                {"name": "der", "args": {"1": "en", "2": "grc-koi", "3": "ἐγκύκλιος παιδείᾱ"}}
            ]}"#,
        );
        assert_eq!(got, [step(EtyMode::LearnedBorrowing, "encyclopaedīa")]);
    }

    #[test]
    fn classic_templates_without_usable_etymon() {
        let got = chain(
            r#"{"etymology_templates": [
                {"name": "etymon", "args": {"1": "en", "id": "female parent", "tree": "1"}},
                {"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}}
            ]}"#,
        );
        assert_eq!(got, [None, step(EtyMode::Inherited, "moder")]);
        // an {{etymon}} for another language is not the entry's
        let got = chain(
            r#"{"etymology_templates": [
                {"name": "etymon", "args": {"1": "enm", "2": ":inh", "3": "ang:mōdor"}},
                {"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}}
            ]}"#,
        );
        assert_eq!(got, [None, step(EtyMode::Inherited, "moder")]);
    }

    #[test]
    fn unknown_origin_ends_the_etymology() {
        let got = chain(
            r#"{"etymology_templates": [
                {"name": "unk", "args": {"1": "abs"}},
                {"name": "bor", "args": {"1": "en", "2": "nl", "3": "smuilen"}}
            ]}"#,
        );
        assert!(got.is_empty());
        // but not when it qualifies a later part
        let got = chain(
            r#"{"etymology_templates": [
                {"name": "inh", "args": {"1": "en", "2": "enm", "3": "foo"}},
                {"name": "unk", "args": {"1": "en"}}
            ]}"#,
        );
        assert_eq!(got, [step(EtyMode::Inherited, "foo"), None]);
    }

    fn parts(lang: &str, args: &str) -> Option<(EtyMode, Vec<String>)> {
        let json = format!(r#"{{"etymology_templates": [{{"name": "af", "args": {args}}}]}}"#);
        chain_in(lang, &json).into_iter().next().flatten()
    }

    #[test]
    fn affix_parts() {
        let strs = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        // language prefixes and inline modifiers
        assert_eq!(
            parts("pt", r#"{"1": "pt", "2": "a-<id:privative>", "3": "grc:χρῶμα<t:color>", "4": "-ia"}"#),
            Some((EtyMode::Affix, strs(&["a-", "grc:χρῶμα", "-ia"])))
        );
        // a link to the lemma
        assert_eq!(
            parts("it", r#"{"1": "it", "2": "la:[[lītus|lītoris]]<t:shore>", "3": "-aneo"}"#),
            Some((EtyMode::Suffix, strs(&["la:lītus", "-aneo"])))
        );
        // langN= and typeN= count parts from 1
        assert_eq!(
            parts("en", r#"{"1": "en", "2": "vexillum", "3": "-ology", "lang1": "la"}"#),
            Some((EtyMode::Suffix, strs(&["la:vexillum", "-ology"])))
        );
        assert_eq!(
            parts("tl", r#"{"1": "tl", "2": "sa-", "3": "ibabaw", "4": "-um-<type:infix>"}"#)
                .map(|(mode, _)| mode),
            Some(EtyMode::Affix)
        );
        assert_eq!(
            parts("en", r#"{"1": "en", "2": "bʰer-", "3": "-tis", "type1": "root"}"#)
                .map(|(mode, _)| mode),
            Some(EtyMode::Suffix)
        );
    }

    mod chains {
        use super::*;
        use crate::embeddings::{self, Config};
        use wety_core::{gloss::Gloss, items::RealItem, langterm::Term, pos::Pos};

        struct Setup {
            sp: StringPool,
            items: Items,
            embeddings: Embeddings,
        }

        impl Setup {
            fn new(name: &str) -> Self {
                let config = Config {
                    model_name: embeddings::DEFAULT_MODEL.to_string(),
                    model_revision: embeddings::DEFAULT_MODEL_REVISION.to_string(),
                    batch_size: 1,
                    cache_path: std::env::temp_dir().join(format!("wety-chain-tests-{name}")),
                };
                Self {
                    sp: StringPool::new(),
                    items: Items::new().unwrap(),
                    embeddings: Embeddings::new(&config).unwrap(),
                }
            }

            fn add(&mut self, lang: &str, term: &str) -> ItemId {
                let item = RealItem {
                    ety_num: 1,
                    ety_label: None,
                    lang: Lang::from_str(lang).unwrap(),
                    term: Term::new(&mut self.sp, term),
                    pos: vec![Pos::from_str("noun").unwrap()],
                    gloss: vec![Gloss::new(&mut self.sp, "x")],
                    page_term: None,
                    romanization: None,
                    is_reconstructed: false,
                };
                self.items.add_real(item).0
            }

            fn process(&mut self, item: ItemId, templates: &str) {
                let lang = self.items.get(item).lang();
                let json = format!(r#"{{"etymology_templates": {templates}}}"#);
                let mut bytes = json.into_bytes();
                let json = to_borrowed_value(&mut bytes).unwrap();
                let ety = WiktextractJsonItem { json }
                    .get_etymology(&mut self.sp, lang)
                    .unwrap();
                self.items
                    .process_item_raw_etymology(&self.sp, &self.embeddings, item, &ety)
                    .unwrap();
            }

            // record an item's own etymology (as read), for other citations to
            // weigh, without processing it
            fn record(&mut self, item: ItemId, templates: &str) {
                let lang = self.items.get(item).lang();
                let json = format!(r#"{{"etymology_templates": {templates}}}"#);
                let mut bytes = json.into_bytes();
                let json = to_borrowed_value(&mut bytes).unwrap();
                let ety = WiktextractJsonItem { json }
                    .get_etymology(&mut self.sp, lang)
                    .unwrap();
                self.items.raw_templates.ety.insert(item, ety);
                self.items.index_mentions();
            }

            fn add_glossed(&mut self, lang: &str, term: &str, gloss: &str) -> ItemId {
                let item = self.add(lang, term);
                if let wety_core::items::Item::Real(r) = self.items.graph.item_mut(item) {
                    r.gloss = vec![Gloss::new(&mut self.sp, gloss)];
                }
                item
            }

            // parents as "lang:term" (with "?" for an imputed one)
            fn parents(&self, item: ItemId) -> Vec<String> {
                let ety = self.items.graph.immediate_ety(item);
                ety.map(|e| e.items).unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        let p = self.items.get(p);
                        let imputed = if p.is_imputed() { "?" } else { "" };
                        format!("{}:{}{imputed}", p.lang().code(), p.term().resolve(&self.sp))
                    })
                    .collect()
            }

            fn parent(&self, item: ItemId) -> ItemId {
                self.items.graph.immediate_ety(item).unwrap().items[0]
            }
        }

        #[test]
        fn missing_parents_are_linked() {
            let mut t = Setup::new("missing");
            // a borrowing from a term missing from the data (was dropped)
            let rendezvous = t.add("en", "rendezvous");
            t.process(rendezvous, r#"[{"name": "bor", "args": {"1": "en", "2": "fr", "3": "rendez-vous"}}]"#);
            assert_eq!(t.parents(rendezvous), ["fr:rendez-vous?"]);
            // a compound with a missing part (was dropped)
            t.add("en", "tooth");
            let toothbrush = t.add("en", "toothbrush");
            t.process(toothbrush, r#"[{"name": "af", "args": {"1": "en", "2": "tooth", "3": "brusch"}}]"#);
            assert_eq!(t.parents(toothbrush), ["en:tooth", "en:brusch?"]);
        }

        #[test]
        fn chains_through_missing_parents() {
            let mut t = Setup::new("chains");
            t.add("ang", "mōdor");
            t.add("la", "muttum");
            // within one line of descent
            let mother = t.add("en", "mother");
            t.process(mother, r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "mōdor"}}]"#);
            assert_eq!(t.parents(mother), ["enm:moder?"]);
            assert_eq!(t.parents(t.parent(mother)), ["ang:mōdor"]);
            // not across a borrowing, unless {{etymon}} says so
            let mot = t.add("en", "mot");
            t.process(mot, r#"[{"name": "bor", "args": {"1": "en", "2": "fro", "3": "mot"}},
                {"name": "der", "args": {"1": "en", "2": "la", "3": "muttum"}}]"#);
            assert!(t.parents(t.parent(mot)).is_empty());
            let word = t.add("en", "motword");
            t.process(word, r#"[{"name": "etymon", "args": {"1": "en", "2": ":bor",
                "3": "fro:mot<ety:inh<la:muttum>>"}}]"#);
            assert_eq!(t.parents(t.parent(word)), ["la:muttum"]);
        }

        #[test]
        fn own_etymology_and_descendants_lists() {
            let mut t = Setup::new("own");
            let octo = t.add("la", "Octōber");
            let octobre = t.add("fro", "octobre");
            let october = t.add("en", "October");
            t.items.graph.add_ety(octobre, EtyMode::Inherited, Some(0), &[octo], &[0.5]);
            // a Descendants list gave the more specific parent: kept
            t.items.graph.add_ety(october, EtyMode::Borrowed, Some(0), &[octobre], &[0.5]);
            t.process(october, r#"[{"name": "der", "args": {"1": "en", "2": "la", "3": "Octōber"}}]"#);
            assert_eq!(t.parents(october), ["fro:octobre"]);
            // a different parent: the item's own section wins
            let enm = t.add("enm", "Octobre");
            let october2 = t.add("en", "Octobre");
            t.items.graph.add_ety(october2, EtyMode::Borrowed, Some(0), &[octo], &[0.9]);
            t.process(october2, r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "Octobre"}}]"#);
            assert_eq!(t.parents(october2), ["enm:Octobre"]);
            // but not for a term missing from the data
            t.items.graph.remove_ety(october2);
            t.items.graph.add_ety(october2, EtyMode::Inherited, Some(0), &[enm], &[0.5]);
            t.process(october2, r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "Octobere"}}]"#);
            assert_eq!(t.parents(october2), ["enm:Octobre"]);
        }

        #[test]
        fn citations_of_a_missing_term_share_it() {
            let mut t = Setup::new("shared");
            t.add("ang", "mōdor");
            let mother = t.add("en", "mother");
            let motherly = t.add("en", "motherly");
            t.process(mother, r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "mōdor"}}]"#);
            t.process(motherly, r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "modor"}}]"#);
            assert_eq!(t.parent(mother), t.parent(motherly));
            // the first citation's chain is kept
            assert_eq!(t.parents(t.parent(motherly)), ["ang:mōdor"]);
        }

        #[test]
        fn hyphenated_roots_are_bases() {
            let mut t = Setup::new("roots");
            let root = t.add("ine-pro", "peh₂-");
            if let wety_core::items::Item::Real(r) = t.items.graph.item_mut(root) {
                r.pos = vec![Pos::root_pos()];
            }
            t.add("ine-pro", "-tḗr");
            let father = t.add("ine-pro", "ph₂tḗr");
            t.process(father, r#"[{"name": "etymon", "args": {"1": "ine-pro", "2": ":af",
                "3": "*peh₂-", "4": "*-tḗr"}}]"#);
            let ety = t.items.graph.immediate_ety(father).unwrap();
            assert_eq!((ety.mode, ety.head), (EtyMode::Suffix, Some(0)));
        }

        #[test]
        fn the_continuation_picks_the_homograph() {
            // en bairn < enm bern < ang bearn; enm "bern" has three entries
            let mut t = Setup::new("bairn");
            let bearn = t.add("ang", "bearn");
            t.add("ang", "beorn");
            t.add("ang", "bere");
            let barn_building = t.add_glossed("enm", "bern", "barn, farm building");
            let man = t.add_glossed("enm", "bern", "a man or human");
            let child = t.add_glossed("enm", "bern", "child");
            t.record(barn_building, r#"[{"name": "af", "args": {"1": "enm", "2": "bere", "3": "-ern"}}]"#);
            t.record(man, r#"[{"name": "inh", "args": {"1": "enm", "2": "ang", "3": "beorn"}}]"#);
            t.record(child, r#"[{"name": "inh", "args": {"1": "enm", "2": "ang", "3": "bearn"}}]"#);
            let bairn = t.add_glossed("en", "bairn", "a child or baby");
            t.process(bairn, r#"[{"name": "der", "args": {"1": "en", "2": "enm", "3": "bern"}},
                {"name": "der", "args": {"1": "en", "2": "ang", "3": "bearn"}}]"#);
            assert_eq!(t.parent(bairn), child);
            let _ = bearn;
        }

        #[test]
        fn a_missing_homograph_is_imputed() {
            // the only enm "bern" is another word: the one meant is missing
            let mut t = Setup::new("missing-homograph");
            t.add("ang", "bearn");
            t.add("ang", "beorn");
            let man = t.add_glossed("enm", "bern", "a man or human");
            t.record(man, r#"[{"name": "inh", "args": {"1": "enm", "2": "ang", "3": "beorn"}}]"#);
            let bairn = t.add("en", "bairn");
            t.process(bairn, r#"[{"name": "der", "args": {"1": "en", "2": "enm", "3": "bern"}},
                {"name": "der", "args": {"1": "en", "2": "ang", "3": "bearn"}}]"#);
            assert_eq!(t.parents(bairn), ["enm:bern?"]);
            assert_eq!(t.parents(t.parent(bairn)), ["ang:bearn"]);
            // a spelling variant of the source is not a contradiction
            let mut t = Setup::new("variant");
            let freo = t.add("ang", "frēo");
            let freoh = t.add("ang", "frēoh");
            t.items.graph.add_ety(freoh, EtyMode::Form, Some(0), &[freo], &[1.0]);
            let fre = t.add("enm", "fre");
            t.record(fre, r#"[{"name": "inh", "args": {"1": "enm", "2": "ang", "3": "frēoh"}}]"#);
            let free = t.add("en", "free");
            t.process(free, r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "fre"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "frēo"}}]"#);
            assert_eq!(t.parent(free), fre);
        }

        #[test]
        fn glosses_and_ids_pick_the_homograph() {
            let mut t = Setup::new("glosses");
            let _fruit = t.add_glossed("en", "lime", "a green citrus fruit");
            let mineral = t.add_glossed("en", "lime", "calcium oxide, a mineral");
            let limey = t.add("en", "limekiln");
            t.process(limey, r#"[{"name": "af", "args": {"1": "en", "2": "lime", "t1": "calcium oxide", "3": "kiln"}}]"#);
            assert_eq!(t.parent(limey), mineral);
            // a sense id
            t.items.add_id(mineral, "quicklime");
            let quick = t.add("en", "limewash");
            t.process(quick, r#"[{"name": "af", "args": {"1": "en", "2": "lime<id:quicklime>", "3": "wash"}}]"#);
            assert_eq!(t.parent(quick), mineral);
        }

        #[test]
        fn derived_terms_lists_pick_the_homograph() {
            let mut t = Setup::new("derived");
            let _fruit = t.add_glossed("en", "lime", "a citrus fruit");
            let mineral = t.add_glossed("en", "lime", "calcium oxide");
            let en = Lang::from_str("en").unwrap();
            let kiln = en.new_langterm(&mut t.sp, "limekiln");
            t.items.raw_templates.derived.insert(mineral, vec![kiln]);
            t.items.index_mentions();
            let limekiln = t.add("en", "limekiln");
            t.process(limekiln, r#"[{"name": "af", "args": {"1": "en", "2": "lime", "3": "kiln"}}]"#);
            assert_eq!(t.parent(limekiln), mineral);
        }

        #[test]
        fn descendants_lists_settle_homographs() {
            let mut t = Setup::new("homographs");
            let _lime1 = t.add("en", "lime");
            let lime2 = t.add("en", "lime");
            let limey = t.add("en", "limey");
            t.items.graph.add_ety(limey, EtyMode::MorphologicalDerivation, Some(0), &[lime2], &[0.1]);
            t.process(limey, r#"[{"name": "af", "args": {"1": "en", "2": "lime", "3": "-y"}}]"#);
            assert_eq!(t.items.graph.immediate_ety(limey).unwrap().items[0], lime2);
        }
    }
}
