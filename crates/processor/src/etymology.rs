use crate::{
    HashSet,
    embeddings::Embeddings,
    entries::{
        Affix, EntryPos, Json, JsonStr, affix_by_hyphens, affix_type, clean_template_term,
        modifier, parse_pos, split_lang_prefix, split_modifiers,
    },
    etymon,
    items::{EmbeddingNeeds, Items, intern},
    notes::RawNote,
    progress_bar,
    references::References,
    resolve::{Citation, Relation, Resolution},
};
use wety_core::{
    assertions::{self, Asserted, Credit, Scope},
    ety_graph::{Certainty, EdgeBasis, LanguageOnly, NewEtymology, SourceLang},
    etymology_templates::{EtyMode, TemplateKind},
    items::ItemId,
    langterm::LangTerm,
    languages::{Family, Lang},
    pos::Pos,
    string_pool::StringPool,
};

use std::{mem, str::FromStr};

use anyhow::{Ok, Result, anyhow, ensure};
use simd_json::ValueAccess;

// models the basic info from a wiktionary etymology template
#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawEtyTemplate {
    pub(crate) langterms: Box<[LangTerm]>, // e.g. "en" "re-", "en" "do"
    // What the template says of each langterm besides the term (or nothing)
    pub(crate) parts: Box<[Part]>,
    // Whether this template is known to give the etymology of the previous
    // template's single parent (an {{etymon}} inline etymology), rather than
    // only presumed to (the next classic template).
    pub(crate) continues: bool,
    // For compound-kind templates, each part's affix type, and whether the
    // mode was inferred from them (as for {{af}}), so that both can be
    // revised once the parts are known items (see revise_affixes).
    pub(crate) affixes: Box<[Affix]>,
    pub(crate) refine_mode: bool,
    pub(crate) mode: EtyMode,    // e.g. Prefix
    pub(crate) head: Option<u8>, // e.g. 1 (the index of "do")
    // how firmly the template gives its derivation: possible when hedged
    // (an {{etymon}} keyword's <unc>)
    pub(crate) certainty: Certainty,
}

// What a template says of a cited term besides the term: an id naming its
// etymology or sense (id=, <id:...>; e.g. "mother" in "enm:moder<id:mother>"),
// its gloss (t=, <t:...>), which glosses the cited term itself, unlike the
// citing item's own glosses, and its part of speech (pos=, <pos:...>).
#[derive(Hash, Eq, PartialEq, Debug, Default, Clone)]
pub(crate) struct Part {
    pub(crate) id: Option<Box<str>>,
    pub(crate) gloss: Option<Box<str>>,
    pub(crate) pos: Option<Pos>,
    // the source is unsure of this term's part (an {{etymon}} etymon's <unc>)
    pub(crate) uncertain: bool,
}

impl RawEtyTemplate {
    fn with_part(mut self, part: Part) -> Self {
        self.parts = Box::new([part]);
        self
    }

    fn new(langterm: LangTerm, mode: EtyMode) -> Self {
        Self {
            langterms: Box::from([langterm]),
            parts: Box::default(),
            continues: false,
            affixes: Box::default(),
            refine_mode: false,
            mode,
            head: Some(0),
            certainty: Certainty::Stated,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) enum ParsedRawEtyTemplate {
    Parsed(RawEtyTemplate),
    Skipped,
    // {{unk}} after a template: the origin of the term before it is unknown
    // ("from Middle English foo, of unknown origin")
    Unknown,
    // a language or family named as the source, but no word of it
    // ({{bor|en|cy|-}}, {{etymon}}'s "trk:"); true when an inline etymology
    // gives it for the parent before it (see RawEtyTemplate::continues)
    LanguageOnly(LanguageOnly, bool),
    // after a template with a single parent: that parent's own etymologies,
    // given inline as alternatives ({{etymon}}'s <ety:...> naming several),
    // each a chain
    Alternatives(Box<[Box<[ParsedRawEtyTemplate]>]>),
}

impl ParsedRawEtyTemplate {
    fn collect_templates<'a>(chain: &'a [Self], out: &mut Vec<&'a RawEtyTemplate>) {
        for t in chain {
            match t {
                Self::Parsed(template) => out.push(template),
                Self::Alternatives(chains) => {
                    for chain in chains {
                        Self::collect_templates(chain, out);
                    }
                }
                Self::Skipped | Self::Unknown | Self::LanguageOnly(..) => {}
            }
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct RawEtymology {
    // The etymologies an entry's section gives, alternatives ("from A or B"),
    // each a chain of templates whose first gives the entry's parents. None:
    // the section says the origin is unknown ({{unk}}).
    pub(crate) alternatives: Box<[Box<[ParsedRawEtyTemplate]>]>,
    // what the templates were read from: {{etymon}}, the classic templates, a
    // lone {{m}} or a form-of sense
    pub(crate) basis: EdgeBasis,
    // the section's footnotes
    pub(crate) notes: Box<[RawNote]>,
}

impl RawEtymology {
    fn new(templates: Vec<ParsedRawEtyTemplate>, basis: EdgeBasis) -> Self {
        Self::alternatives(vec![templates], basis)
    }

    fn alternatives(alternatives: Vec<Vec<ParsedRawEtyTemplate>>, basis: EdgeBasis) -> Self {
        Self {
            alternatives: alternatives
                .into_iter()
                .map(Vec::into_boxed_slice)
                .collect(),
            basis,
            notes: Box::default(),
        }
    }

    fn unknown() -> Self {
        Self::alternatives(vec![], EdgeBasis::Template)
    }

    fn is_unknown(&self) -> bool {
        self.alternatives.is_empty()
    }

    pub(crate) fn templates(&self) -> impl Iterator<Item = &RawEtyTemplate> {
        let mut out = vec![];
        for chain in &self.alternatives {
            ParsedRawEtyTemplate::collect_templates(chain, &mut out);
        }
        out.into_iter()
    }
}

fn process_derived_kind_json_template(
    string_pool: &mut StringPool,
    args: &Json,
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
    let part = term_part(args, "3", &["t", "gloss", "5"]);
    Some(RawEtyTemplate::new(ety_langterm, mode).with_part(part))
}

// What a template gives for the term in arg `key` (see Part).
fn term_part(args: &Json, key: &str, gloss_keys: &[&str]) -> Part {
    let modifiers = args
        .get_str(key)
        .map(|arg| split_modifiers(arg).1)
        .unwrap_or_default();
    let id = modifier(&modifiers, "id").or_else(|| args.get_valid_str("id"));
    let gloss = modifier(&modifiers, "t")
        .or_else(|| modifier(&modifiers, "gloss"))
        .or_else(|| gloss_keys.iter().find_map(|k| args.get_valid_str(k)));
    let pos = modifier(&modifiers, "pos").or_else(|| args.get_valid_str("pos"));
    Part {
        uncertain: false,
        id: id.filter(|s| !s.is_empty()).map(Box::from),
        gloss: gloss.filter(|s| !s.is_empty()).map(Box::from),
        pos: pos.and_then(parse_pos),
    }
}

fn process_abbrev_kind_json_template(
    string_pool: &mut StringPool,
    args: &Json,
    mode: EtyMode,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_term("2")?;
    let ety_langterm = lang.new_langterm(string_pool, ety_term);
    let part = term_part(args, "2", &["t", "gloss", "4"]);
    Some(RawEtyTemplate::new(ety_langterm, mode).with_part(part))
}

fn process_prefix_json_template(
    string_pool: &mut StringPool,
    args: &Json,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_prefix = args.get_affix_term("2", &Affix::Prefix)?;
    let ety_prefix = lang.new_langterm(string_pool, &ety_prefix);
    let ety_term = args.get_valid_term("3")?;
    let ety_term = lang.new_langterm(string_pool, ety_term);
    Some(RawEtyTemplate {
        certainty: Certainty::Stated,
        langterms: Box::new([ety_prefix, ety_term]),
        parts: Box::default(),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
        mode: EtyMode::Prefix,
        head: Some(1),
    })
}

fn process_suffix_json_template(
    string_pool: &mut StringPool,
    args: &Json,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_term("2")?;
    let ety_term = lang.new_langterm(string_pool, ety_term);
    let ety_suffix = args.get_affix_term("3", &Affix::Suffix)?;
    let ety_suffix = lang.new_langterm(string_pool, &ety_suffix);
    Some(RawEtyTemplate {
        certainty: Certainty::Stated,
        langterms: Box::new([ety_term, ety_suffix]),
        parts: Box::default(),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
        mode: EtyMode::Suffix,
        head: Some(0),
    })
}

fn process_circumfix_json_template(
    string_pool: &mut StringPool,
    args: &Json,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_prefix = args.get_affix_term("2", &Affix::Prefix)?;
    let ety_term = args.get_valid_term("3")?;
    let ety_suffix = args.get_affix_term("4", &Affix::Suffix)?;

    let ety_term = lang.new_langterm(string_pool, ety_term);
    let ety_circumfix = format!("{ety_prefix} {ety_suffix}");
    let ety_circumfix = lang.new_langterm(string_pool, &ety_circumfix);
    Some(RawEtyTemplate {
        certainty: Certainty::Stated,
        langterms: Box::new([ety_term, ety_circumfix]),
        parts: Box::default(),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
        mode: EtyMode::Circumfix,
        head: Some(0),
    })
}

fn process_infix_json_template(
    string_pool: &mut StringPool,
    args: &Json,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_term = args.get_valid_term("2")?;
    let ety_infix = args.get_affix_term("3", &Affix::Infix)?;

    let ety_term = lang.new_langterm(string_pool, ety_term);
    let ety_infix = lang.new_langterm(string_pool, &ety_infix);
    Some(RawEtyTemplate {
        certainty: Certainty::Stated,
        langterms: Box::new([ety_term, ety_infix]),
        parts: Box::default(),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
        mode: EtyMode::Infix,
        head: Some(0),
    })
}

fn process_confix_json_template(
    string_pool: &mut StringPool,
    args: &Json,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let ety_prefix = args.get_affix_term("2", &Affix::Prefix)?;
    let ety2 = args.get_valid_term("3")?;

    let ety_prefix = lang.new_langterm(string_pool, &ety_prefix);
    if let Some(ety_suffix) = args.get_affix_term("4", &Affix::Suffix) {
        let ety_term = lang.new_langterm(string_pool, ety2);
        let ety_suffix = lang.new_langterm(string_pool, &ety_suffix);
        return Some(RawEtyTemplate {
            certainty: Certainty::Stated,
            langterms: Box::new([ety_prefix, ety_term, ety_suffix]),
            parts: Box::default(),
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
        certainty: Certainty::Stated,
        langterms: Box::new([ety_prefix, ety_suffix]),
        parts: Box::default(),
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
    args: &Json,
    mode: EtyMode,
    lang: Lang,
) -> Option<RawEtyTemplate> {
    let mut ety_langterms = vec![];
    let mut parts = vec![];
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
        parts.push(Part {
            uncertain: false,
            id: part_param("id"),
            gloss: part_param("t").or_else(|| part_param("gloss")),
            pos: part_param("pos").as_deref().and_then(parse_pos),
        });
    }
    if !ety_langterms.is_empty() {
        return Some(RawEtyTemplate {
            certainty: Certainty::Stated,
            langterms: ety_langterms.into_boxed_slice(),
            parts: parts.into_boxed_slice(),
            continues: false,
            refine_mode: mode == EtyMode::Affix,
            affixes: affixes.clone().into_boxed_slice(),
            mode: if mode == EtyMode::Affix {
                affixation_kind(&affixes, n_base_terms)
            } else {
                mode
            },
            // see above
            head: (n_base_terms == 1)
                .then(|| u8::try_from(head).ok())
                .flatten(),
        });
    }
    None
}

fn process_vrddhi_kind_json_template(
    string_pool: &mut StringPool,
    args: &Json,
    ety_mode: EtyMode,
) -> Option<RawEtyTemplate> {
    let ety_lang = args.get_valid_str("1")?;
    let ety_lang = Lang::from_str(ety_lang).ok()?;
    let ety_term = args.get_valid_term("2")?;
    let ety_langterm = ety_lang.new_langterm(string_pool, ety_term);
    Some(RawEtyTemplate::new(ety_langterm, ety_mode))
}

pub(crate) fn validate_ety_template_lang(args: &Json, lang: Lang) -> Result<()> {
    let item_lang = lang.code();
    let template_lang = args.get_valid_str("1").ok_or_else(|| {
        anyhow!("ety template does not contain valid \"1\" lang arg: it has args:\n{args}")
    })?;
    ensure!(
        template_lang == item_lang,
        "ety template \"1\" lang arg was {template_lang}, should have matched item lang {item_lang}"
    );
    Ok(())
}

fn process_json_ety_template(
    string_pool: &mut StringPool,
    template: &Json,
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

// A derived-kind template naming a language or family but no word
// ({{bor|en|cy|-}}, {{der|mhr|trk}}): the origin, its exact source uncertain.
// One that shows a word unlinked (an alt) is no such origin.
fn process_language_only_json_template(template: &Json, lang: Lang) -> Option<LanguageOnly> {
    let mode = EtyMode::from_str(template.get_valid_str("name")?).ok()?;
    (mode.template_kind() == Some(TemplateKind::Derived)).then_some(())?;
    let args = template.get("args")?;
    validate_ety_template_lang(args, lang).ok()?;
    if args.get_valid_term("3").is_some()
        || args.get_valid_str("4").is_some()
        || args.get_valid_str("alt").is_some()
    {
        return None;
    }
    let code = args.get_valid_str("2")?;
    let code = code.split_once(',').map_or(code, |(c, _)| c);
    let source = Lang::from_str(code)
        .map(SourceLang::Lang)
        .or_else(|_| Family::from_str(code).map(SourceLang::Family))
        .ok()?;
    Some(LanguageOnly {
        source,
        mode,
        certainty: Certainty::Stated,
    })
}

impl EntryPos<'_> {
    // The args of the entry's {{etymon}} (or {{ety}}) template for its own
    // language. A page section rarely has more than one; we take the first.
    fn get_etymon_args(&self, lang: Lang) -> Option<&Json<'_>> {
        self.templates
            .iter()
            .filter(|t| {
                t.get_valid_str("name")
                    .is_some_and(|n| matches!(n, "etymon" | "ety"))
            })
            .filter_map(|t| t.get("args"))
            .find(|args| validate_ety_template_lang(args, lang).is_ok())
    }

    /// The ids this entry declares, by which references can name it: its
    /// {{etymon}} id=, its {{etymid}} ids and its senses' {{senseid}}s.
    pub(crate) fn get_ids(&self, lang: Lang) -> Vec<&str> {
        let mut ids: Vec<&str> = self
            .get_etymon_args(lang)
            .and_then(|args| args.get_valid_str("id"))
            .into_iter()
            .collect();
        for template in self.templates {
            if template.get_valid_str("name") == Some("etymid")
                && let Some(args) = template.get("args")
                && args.get_valid_str("1") == Some(lang.code())
                && let Some(id) = args.get_valid_str("2")
            {
                ids.push(id);
            }
        }
        for sense in self.pos.get_array("senses").into_iter().flatten() {
            for id in sense
                .get_array("senseids")
                .into_iter()
                .flatten()
                .filter_map(|id| id.as_str())
            {
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
        let mut alternatives = etymon::parse_etymon_args(string_pool, args, lang);
        if alternatives.is_empty() {
            return None;
        }
        if let [templates] = alternatives.as_mut_slice()
            && let [ParsedRawEtyTemplate::Parsed(parent)] = templates.as_slice()
            && parent.langterms.len() == 1
            && let Some(standard) = self.get_standard_ety(string_pool, lang)
            && let Some(chain) = Vec::from(standard.alternatives).into_iter().next()
        {
            let parent = parent.langterms[0];
            let continuation = Vec::from(chain)
                .into_iter()
                .skip_while(
                    |t| !matches!(t, ParsedRawEtyTemplate::Parsed(c) if *c.langterms == [parent]),
                )
                .skip(1);
            templates.extend(continuation);
        }
        Some(RawEtymology::alternatives(alternatives, EdgeBasis::Etymon))
    }

    // Many ety sections contain a single {{m}} template and no others, and
    // consist only of "From {{m..." (or similar). This is to handle this case.
    fn get_single_mention_ety(&self, string_pool: &mut StringPool) -> Option<RawEtymology> {
        // Since we handle {{root}} separately, we still want to get etys where
        // there is say one {{root}} then one "From {{m..."
        let mut templates = self
            .templates
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
        Some(RawEtymology::new(
            vec![ParsedRawEtyTemplate::Parsed(ety)],
            EdgeBasis::Mention,
        ))
    }

    // An etymology that opens as unknown ({{unk}}) is read as such. One that
    // opens as unknown or uncertain ({{unc}}) goes on to conjecture or compare
    // ("Perhaps from ...", "Compare ..."): no template after it names the
    // parent. $$ Reading the conjectures as possible etymologies needs their
    // prose (syn-2). An {{unk}} later on says the origin of the term before it
    // is unknown.
    fn get_standard_ety(&self, string_pool: &mut StringPool, lang: Lang) -> Option<RawEtymology> {
        let templates = self.templates;
        (!templates.is_empty()).then_some(())?;
        let mut raw_ety_templates = Vec::with_capacity(templates.len());
        for &template in templates {
            let opens = |names: &[&str]| {
                template
                    .get_valid_str("name")
                    .is_some_and(|name| names.contains(&name))
                    && !raw_ety_templates
                        .iter()
                        .any(|t| matches!(t, ParsedRawEtyTemplate::Parsed(_)))
            };
            if opens(&["unk", "unknown"]) {
                return Some(RawEtymology::unknown());
            }
            if opens(&["unc", "uncertain"]) {
                break;
            }
            if template
                .get_valid_str("name")
                .is_some_and(|name| matches!(name, "unk" | "unknown"))
            {
                raw_ety_templates.push(ParsedRawEtyTemplate::Unknown);
                continue;
            }
            if let Some(raw_ety_template) = process_json_ety_template(string_pool, template, lang) {
                raw_ety_templates.push(ParsedRawEtyTemplate::Parsed(raw_ety_template));
            } else if let Some(l) = process_language_only_json_template(template, lang) {
                raw_ety_templates.push(ParsedRawEtyTemplate::LanguageOnly(l, false));
            } else {
                raw_ety_templates.push(ParsedRawEtyTemplate::Skipped);
            }
        }
        (!raw_ety_templates.is_empty())
            .then(|| RawEtymology::new(raw_ety_templates, EdgeBasis::Template))
    }

    // if no ety section or no templates, as a fallback we see if the first sense
    // is a form-of definition (an alternative form, an inflection) of another
    // term: e.g. "happenin'" is an alternative form of "happening".
    fn get_form_ety(&self, string_pool: &mut StringPool, lang: Lang) -> Option<RawEtymology> {
        let target = self
            .pos
            .get_array("senses")
            .and_then(|senses| senses.first())
            .and_then(|sense| sense.get_array("form_of"))
            .and_then(|targets| targets.first())?;
        let term = target.get_valid_term("term")?;
        let langterm = lang.new_langterm(string_pool, term);
        let mut ety = RawEtyTemplate::new(langterm, EtyMode::Form);
        // the gloss the definition gives the term (enm bern: of barn, "child")
        if let Some(gloss) = target.get_valid_str("gloss") {
            ety = ety.with_part(Part {
                uncertain: false,
                id: None,
                gloss: Some(gloss.into()),
                pos: None,
            });
        }
        Some(RawEtymology::new(
            vec![ParsedRawEtyTemplate::Parsed(ety)],
            EdgeBasis::FormOf,
        ))
    }

    pub(crate) fn get_etymology(
        &self,
        string_pool: &mut StringPool,
        lang: Lang,
        references: &References,
    ) -> Option<RawEtymology> {
        let mut ety = self
            .get_etymon_ety(string_pool, lang)
            .or_else(|| self.get_single_mention_ety(string_pool))
            .or_else(|| match self.get_standard_ety(string_pool, lang) {
                // a form-of sense still links an entry of unknown origin
                Some(unknown) if unknown.is_unknown() => {
                    self.get_form_ety(string_pool, lang).or(Some(unknown))
                }
                standard => standard,
            })
            .or_else(|| self.get_form_ety(string_pool, lang))?;
        ety.notes = self.get_notes(references);
        Some(ety)
    }
}

// The parents one template gives an item, resolved to items.
struct Resolved {
    items: Vec<ItemId>,
    confidences: Vec<f32>,
    certainties: Vec<Certainty>,
    mode: EtyMode,
    head: Option<u8>,
}

impl Items {
    pub(crate) fn get_ety_embedding_needs(
        &self,
        item: ItemId,
        raw_etymology: &RawEtymology,
        needs: &mut EmbeddingNeeds,
    ) {
        // The citing item is the item, or an imputed item of its chain, which
        // borrows the item's embeddings.
        for template in raw_etymology.templates() {
            for (n, &langterm) in template.langterms.iter().enumerate() {
                if let Some(candidates) = self.candidates(langterm)
                    && candidates.len() > 1
                {
                    needs.items.insert(item);
                    needs.items.extend(candidates);
                    if let Some(gloss) = template.parts.get(n).and_then(|p| p.gloss.clone()) {
                        needs.texts.insert(gloss);
                    }
                }
            }
        }
    }

    // Each alternative's first parsed template gives the item's parent(s) in
    // that etymology; the item gets them all as its etymologies at once. If an
    // alternative names a single parent missing from the data (and so
    // imputed), its next template is taken to give the imputed parent's own
    // parent, and so on, as long as it is known to (an {{etymon}} inline
    // etymology) or the chain stays within one line of descent (e.g. en < enm <
    // ang). $$ Language dates would allow the latter for e.g. Middle English <
    // Latin too. Leading templates that could not be parsed are passed over.
    fn process_item_raw_etymology(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        item: ItemId,
        raw_etymology: &RawEtymology,
    ) -> Result<()> {
        if raw_etymology.is_unknown() {
            if self.graph.set_unknown(item) {
                let credits = self.own_credits(item, raw_etymology.basis, &raw_etymology.notes)?;
                self.graph.set_asserted(
                    item,
                    vec![Asserted {
                        credits,
                        ..Asserted::default()
                    }],
                );
            }
            return Ok(());
        }
        let chains: Vec<&[ParsedRawEtyTemplate]> =
            raw_etymology.alternatives.iter().map(|c| &c[..]).collect();
        self.set_alternatives(string_pool, embeddings, item, item, raw_etymology, &chains)
    }

    // Give `target` (the item, or an imputed parent its section names) the
    // etymologies `chains` give, as stated in `item`'s section, and carry each
    // chain on from there.
    fn set_alternatives(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        item: ItemId,
        target: ItemId,
        raw_etymology: &RawEtymology,
        chains: &[&[ParsedRawEtyTemplate]],
    ) -> Result<()> {
        // A chain with a word gives it, as before; one with none, the first
        // language or family it names.
        let mut firsts = vec![];
        let mut origins = vec![];
        for &chain in chains {
            let Some(i) = chain
                .iter()
                .position(|t| matches!(t, ParsedRawEtyTemplate::Parsed(_)))
            else {
                origins.extend(chain.iter().find_map(|t| match t {
                    ParsedRawEtyTemplate::LanguageOnly(l, _) => Some(*l),
                    _ => None,
                }));
                continue;
            };
            let resolved =
                self.resolve_template(string_pool, embeddings, item, target, chain, i)?;
            firsts.push((chain, i, resolved));
        }
        if firsts.is_empty() && origins.is_empty() {
            return Ok(());
        }
        let asserted = if target == item {
            Asserted {
                credits: self.own_credits(item, raw_etymology.basis, &raw_etymology.notes)?,
                ..Asserted::default()
            }
        } else {
            // stated in `item`'s section, under its footnotes
            Asserted {
                page: Some(item),
                credits: self.credits(&raw_etymology.notes),
                ..Asserted::default()
            }
        };
        if firsts.is_empty() {
            if self.graph.set_language_only(target, &origins) {
                self.graph
                    .set_asserted(target, vec![asserted; origins.len()]);
            }
            return Ok(());
        }
        let resolved: Vec<&Resolved> = firsts.iter().map(|(_, _, r)| r).collect();
        if !self.set_own_etys(target, raw_etymology.basis, &resolved, &asserted) {
            return Ok(());
        }
        if !origins.is_empty() {
            self.graph.push_language_only(target, &origins);
            self.graph
                .set_asserted(target, vec![asserted; resolved.len() + origins.len()]);
        }
        for (chain, i, resolved) in firsts {
            self.continue_chain(
                string_pool,
                embeddings,
                item,
                raw_etymology,
                target,
                chain,
                i,
                resolved,
            )?;
        }
        Ok(())
    }

    // The parents a chain's template at `i` gives `current_item`, cited from
    // `item`'s section.
    fn resolve_template(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        item: ItemId,
        current_item: ItemId,
        templates: &[ParsedRawEtyTemplate],
        i: usize,
    ) -> Result<Resolved> {
        let ParsedRawEtyTemplate::Parsed(template) = &templates[i] else {
            unreachable!("only parsed templates are resolved");
        };
        // What the etymology gives as a single parent's own source: the next
        // template, if it is known to continue the chain, or if both are
        // derived-kind ({{inh}}, {{bor}}, {{der}}, ...) and the next is in
        // an earlier stage of the parent's line of descent ("from enm m,
        // from ang o"; not "from French X or Spanish Y", nor parts of a
        // word listed one by one).
        let derived = |t: &RawEtyTemplate| t.mode.template_kind() == Some(TemplateKind::Derived);
        let next = match (&template.langterms[..], templates.get(i + 1)) {
            ([parent], Some(ParsedRawEtyTemplate::Parsed(next))) => match &next.langterms[..] {
                [source]
                    if next.continues
                        || (derived(template)
                            && derived(next)
                            && parent
                                .lang
                                .ety2non()
                                .strictly_descends_from(source.lang.ety2non())) =>
                {
                    Some(*source)
                }
                _ => None,
            },
            _ => None,
        };
        let mut ety_items = Vec::with_capacity(template.langterms.len());
        let mut confidences = Vec::with_capacity(template.langterms.len());
        let mut certainties = Vec::with_capacity(template.langterms.len());
        for (n, &ety_langterm) in template.langterms.iter().enumerate() {
            let part = template.parts.get(n);
            let citation = Citation {
                langterm: ety_langterm,
                id: part.and_then(|p| p.id.as_deref()),
                gloss: part.and_then(|p| p.gloss.as_deref()),
                // given, or implied by the derivation (a deverbal's source
                // is a verb)
                pos: part.and_then(|p| p.pos).or_else(|| {
                    (template.mode == EtyMode::Deverbal)
                        .then(|| Pos::from_str("verb").ok())
                        .flatten()
                }),
                citer: current_item,
                relation: Relation::Parent { next },
            };
            let (ety_item, confidence) = match self.resolve(string_pool, embeddings, &citation)? {
                Resolution::Found { item, confidence } => (item, confidence),
                Resolution::Missing => (self.impute_item(ety_langterm, item), 0.0),
            };
            // two terms that name one item ("from *bratrъ and *bratъ", one
            // entry) give one parent, unless they are parts of a word
            // (reduplication)
            if template.affixes.is_empty() && ety_items.contains(&ety_item) {
                continue;
            }
            ety_items.push(ety_item);
            confidences.push(confidence);
            certainties.push(if part.is_some_and(|p| p.uncertain) {
                template.certainty.max(Certainty::Possible)
            } else {
                template.certainty
            });
        }
        let (mode, mut head) = self.revise_affixes(template, &ety_items);
        if template.affixes.is_empty() && ety_items.len() == 1 {
            head = Some(0);
        }
        Ok(Resolved {
            items: ety_items,
            confidences,
            certainties,
            mode,
            head,
        })
    }

    // Continue a chain of `item`'s section past its template at `i`, which gave
    // `current_item` the parents `resolved`: while the single parent is
    // imputed, the next template gives its own etymology, or the next
    // alternatives its own etymologies.
    #[allow(clippy::too_many_arguments)]
    fn continue_chain(
        &mut self,
        string_pool: &StringPool,
        embeddings: &Embeddings,
        item: ItemId,
        raw_etymology: &RawEtymology,
        mut current_item: ItemId,
        templates: &[ParsedRawEtyTemplate],
        mut i: usize,
        mut resolved: Resolved,
    ) -> Result<()> {
        let credits = self.credits(&raw_etymology.notes);
        loop {
            let [parent] = resolved.items[..] else {
                return Ok(());
            };
            let next = i + 1;
            // the parent's own etymologies, given inline as alternatives
            if let Some(ParsedRawEtyTemplate::Alternatives(chains)) = templates.get(next) {
                if !self.get(parent).is_imputed() {
                    return Ok(());
                }
                let chains: Vec<&[ParsedRawEtyTemplate]> = chains.iter().map(|c| &c[..]).collect();
                return self.set_alternatives(
                    string_pool,
                    embeddings,
                    item,
                    parent,
                    raw_etymology,
                    &chains,
                );
            }
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
            // "from Middle English foo, from Old French": the imputed parent's
            // origin is a language, its word not named
            if let Some(ParsedRawEtyTemplate::LanguageOnly(l, from_inline)) = templates.get(next)
                && self.get(parent).is_imputed()
                && (*from_inline
                    || self
                        .get(current_item)
                        .lang()
                        .descends_from(self.get(parent).lang()))
            {
                if self.graph.set_language_only(parent, &[*l]) {
                    self.graph.set_asserted(
                        parent,
                        vec![Asserted {
                            page: Some(item),
                            credits,
                            ..Asserted::default()
                        }],
                    );
                }
                return Ok(());
            }
            if !continues {
                // "from Middle English foo, of unknown origin": an imputed
                // parent's origin is unknown, as stated in `item`'s section
                if self.get(parent).is_imputed()
                    && matches!(templates.get(next), Some(ParsedRawEtyTemplate::Unknown))
                    && self.graph.set_unknown(parent)
                {
                    self.graph.set_asserted(
                        parent,
                        vec![Asserted {
                            page: Some(item),
                            credits,
                            ..Asserted::default()
                        }],
                    );
                }
                return Ok(());
            }
            current_item = parent;
            i = next;
            let ParsedRawEtyTemplate::Parsed(template) = &templates[i] else {
                return Ok(());
            };
            resolved =
                self.resolve_template(string_pool, embeddings, item, current_item, templates, i)?;
            // A later template is stated to give the parent's etymology only
            // when {{etymon}} gives it inline; otherwise it is read so.
            let basis = if template.continues {
                raw_etymology.basis
            } else {
                EdgeBasis::Chain
            };
            // stated in `item`'s section, under its footnotes
            let asserted = Asserted {
                page: Some(item),
                credits: credits.clone(),
                ..Asserted::default()
            };
            if !self.set_own_etys(current_item, basis, &[&resolved], &asserted) {
                return Ok(());
            }
        }
    }

    // Set an item's etymologies from its own etymology section (or, for an
    // imputed item, from the section of an item citing it). A Descendants list
    // may already have given it a parent; the item's own section is preferred,
    // except where that parent is more specific (it descends from the parent
    // named here: a later stage, e.g. Old French between Latin and English, or
    // a derivative, e.g. a PIE stem where the section names its root), where
    // the item would trade an entry for a term missing from the data, or where the
    // section gives only a mention or a form-of link. Returns whether the
    // etymologies were set.
    fn set_own_etys(
        &mut self,
        item: ItemId,
        basis: EdgeBasis,
        etys: &[&Resolved],
        asserted: &Asserted,
    ) -> bool {
        if let Some(existing) = self.graph.immediate_ety(item) {
            // An imputed item is shared by all citations of its term; the first
            // to give it an ety keeps it.
            if self.get(item).is_imputed() {
                return false;
            }
            // A derivative in the parent's own language is not more specific
            // where the section says the item was inherited or borrowed from an
            // attested word: it names the very word (English table inherited
            // from Middle English table, which the Descendants list of its
            // derivative tablen "to board" also names).
            let names_the_word = |new: &Resolved, parent: ItemId| {
                matches!(
                    new.mode,
                    EtyMode::Inherited
                        | EtyMode::Borrowed
                        | EtyMode::LearnedBorrowing
                        | EtyMode::SemiLearnedBorrowing
                        | EtyMode::UnadaptedBorrowing
                        | EtyMode::OrthographicBorrowing
                ) && !self.get(parent).is_reconstructed()
                    && !self.get(parent).is_imputed()
            };
            let more_specific = match (&existing.items[..], etys) {
                ([old], [new_ety]) => match new_ety.items[..] {
                    [new] => {
                        *old != new
                            && self.is_ancestor(new, *old)
                            && !(self.get(*old).lang() == self.get(new).lang()
                                && names_the_word(new_ety, new))
                    }
                    _ => false,
                },
                _ => false,
            };
            // judged per alternative: one that names an entry or is not weak
            // is reason enough to replace the existing ety
            let loses_entry = etys
                .iter()
                .all(|e| e.items.iter().all(|&p| self.get(p).is_imputed()))
                && existing.items.iter().all(|&e| !self.get(e).is_imputed());
            // a lone {{m}} or a form-of sense, used when there is no
            // etymology proper, is weaker evidence than a Descendants list
            let weak = etys
                .iter()
                .all(|e| matches!(e.mode, EtyMode::Mention | EtyMode::Form));
            if more_specific || loses_entry || weak {
                return false;
            }
        }
        let new: Vec<NewEtymology> = etys
            .iter()
            .map(|e| NewEtymology {
                mode: e.mode,
                head: e.head,
                basis,
                items: &e.items,
                confidences: &e.confidences,
                certainties: &e.certainties,
            })
            .collect();
        self.graph.add_etymologies(item, &new);
        self.graph
            .set_asserted(item, vec![asserted.clone(); etys.len()]);
        self.own_ety.insert(item);
        true
    }

    // A section's footnotes as credits, each work counted once in the graph.
    fn credits(&mut self, notes: &[RawNote]) -> Box<[Credit]> {
        notes
            .iter()
            .map(|note| Credit {
                work: intern(&mut self.works, &mut self.graph.works, &note.work),
                role: note.role,
                scope: Scope::Note,
                locator: note.locator.clone(),
            })
            .collect()
    }

    // What the item's own section asserts credits: its footnotes and, unless
    // a rule reads the section, the entry's References.
    fn own_credits(
        &mut self,
        item: ItemId,
        basis: EdgeBasis,
        notes: &[RawNote],
    ) -> Result<Box<[Credit]>> {
        let mut credits = Vec::from(self.credits(notes));
        if assertions::rule(basis).is_none() {
            credits.extend(self.entry_credits(item)?);
        }
        Ok(credits.into_boxed_slice())
    }

    // A part read as a prefix by its final hyphen may be a root (PIE *peh₂-
    // in *ph₂tḗr = *peh₂- + *-tḗr), which is a base, not an affix. Module:etymon
    // tells these apart by the part's entry; so do we, once parts are items.
    fn revise_affixes(
        &self,
        template: &RawEtyTemplate,
        ety_items: &[ItemId],
    ) -> (EtyMode, Option<u8>) {
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
        let mut raw_templates_ety: Vec<_> =
            mem::take(&mut self.raw_templates.ety).into_iter().collect();
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
    use crate::entries::test_entry;

    // (mode, [term]) for each template in the chain; None for a skipped one
    fn chain(json: &str) -> Vec<Option<(EtyMode, Vec<String>)>> {
        chain_with("en", json, false)
    }

    // as chain(), with each term prefixed by its language code
    fn chain_in(lang: &str, json: &str) -> Vec<Option<(EtyMode, Vec<String>)>> {
        chain_with(lang, json, true)
    }

    fn chain_with(lang: &str, json: &str, codes: bool) -> Vec<Option<(EtyMode, Vec<String>)>> {
        let mut string_pool = StringPool::new();
        let lang = Lang::from_str(lang).unwrap();
        test_entry::with(json, |e| {
            e.get_etymology(&mut string_pool, lang, &References::default())
        })
        .and_then(|ety| Vec::from(ety.alternatives).into_iter().next())
        .map_or_else(Vec::new, Vec::from)
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
            ParsedRawEtyTemplate::Skipped
            | ParsedRawEtyTemplate::Unknown
            | ParsedRawEtyTemplate::LanguageOnly(..)
            | ParsedRawEtyTemplate::Alternatives(_) => None,
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
            parts(
                "pt",
                r#"{"1": "pt", "2": "a-<id:privative>", "3": "grc:χρῶμα<t:color>", "4": "-ia"}"#
            ),
            Some((EtyMode::Affix, strs(&["a-", "grc:χρῶμα", "-ia"])))
        );
        // a link to the lemma
        assert_eq!(
            parts(
                "it",
                r#"{"1": "it", "2": "la:[[lītus|lītoris]]<t:shore>", "3": "-aneo"}"#
            ),
            Some((EtyMode::Suffix, strs(&["la:lītus", "-aneo"])))
        );
        // langN= and typeN= count parts from 1
        assert_eq!(
            parts(
                "en",
                r#"{"1": "en", "2": "vexillum", "3": "-ology", "lang1": "la"}"#
            ),
            Some((EtyMode::Suffix, strs(&["la:vexillum", "-ology"])))
        );
        assert_eq!(
            parts(
                "tl",
                r#"{"1": "tl", "2": "sa-", "3": "ibabaw", "4": "-um-<type:infix>"}"#
            )
            .map(|(mode, _)| mode),
            Some(EtyMode::Affix)
        );
        assert_eq!(
            parts(
                "en",
                r#"{"1": "en", "2": "bʰer-", "3": "-tis", "type1": "root"}"#
            )
            .map(|(mode, _)| mode),
            Some(EtyMode::Suffix)
        );
    }

    mod chains {
        use super::*;
        use crate::embeddings::{self, Config};
        use wety_core::{
            assertions::Role, ety_graph::EtyEdgeAccess, gloss::Gloss, items::RealItem,
            langterm::Term, pos::Pos,
        };

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
                self.add_on_page(lang, term, None)
            }

            // an item whose headword differs from its page title
            fn add_on_page(&mut self, lang: &str, term: &str, page: Option<&str>) -> ItemId {
                let item = RealItem {
                    ety_num: 1,
                    ety_label: None,
                    lang: Lang::from_str(lang).unwrap(),
                    term: Term::new(&mut self.sp, term),
                    pos: vec![Pos::from_str("noun").unwrap()],
                    gloss: vec![Gloss::new(&mut self.sp, "x")],
                    page_term: page.map(|page| Term::new(&mut self.sp, page)),
                    romanization: None,
                    is_reconstructed: false,
                };
                self.items.add_real(item).0
            }

            fn process(&mut self, item: ItemId, templates: &str) {
                self.process_entry(item, &format!(r#"{{"etymology_templates": {templates}}}"#));
            }

            fn process_entry(&mut self, item: ItemId, json: &str) {
                let lang = self.items.get(item).lang();
                let ety = test_entry::with(json, |e| {
                    e.get_etymology(&mut self.sp, lang, &References::default())
                })
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
                let ety = test_entry::with(&json, |e| {
                    e.get_etymology(&mut self.sp, lang, &References::default())
                })
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
                ety.map(|e| e.items)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        let p = self.items.get(p);
                        let imputed = if p.is_imputed() { "?" } else { "" };
                        format!(
                            "{}:{}{imputed}",
                            p.lang().code(),
                            p.term().resolve(&self.sp)
                        )
                    })
                    .collect()
            }

            fn parent(&self, item: ItemId) -> ItemId {
                self.items.graph.immediate_ety(item).unwrap().items[0]
            }

            fn bases(&self, item: ItemId) -> Vec<EdgeBasis> {
                self.items
                    .graph
                    .edges()
                    .filter(|e| e.child() == item)
                    .map(|e| e.basis())
                    .collect()
            }
        }

        #[test]
        fn missing_parents_are_linked() {
            let mut t = Setup::new("missing");
            // a borrowing from a term missing from the data (was dropped)
            let rendezvous = t.add("en", "rendezvous");
            t.process(
                rendezvous,
                r#"[{"name": "bor", "args": {"1": "en", "2": "fr", "3": "rendez-vous"}}]"#,
            );
            assert_eq!(t.parents(rendezvous), ["fr:rendez-vous?"]);
            // a compound with a missing part (was dropped)
            t.add("en", "tooth");
            let toothbrush = t.add("en", "toothbrush");
            t.process(
                toothbrush,
                r#"[{"name": "af", "args": {"1": "en", "2": "tooth", "3": "brusch"}}]"#,
            );
            assert_eq!(t.parents(toothbrush), ["en:tooth", "en:brusch?"]);
        }

        #[test]
        fn alternatives_and_unknown_origins() {
            let mut t = Setup::new("alternatives");
            t.add("fr", "docteur");
            // "From French docteur or English doctor": no primary, both followed,
            // each continuing its own chain
            let dokotera = t.add("mg", "dokotera");
            t.process(
                dokotera,
                r#"[{"name": "etymon", "args": {"1": "mg", "2": ":bor", "3": "fr:docteur",
                    "4": "en:doctor<ety:bor<la:doctor>>"}}]"#,
            );
            assert_eq!(t.parents(dokotera), ["fr:docteur", "en:doctor?"]);
            let etys = t.items.graph.etymologies(dokotera).unwrap();
            assert_eq!((etys.kinds.len(), etys.primary), (2, None));
            let doctor = t.items.graph.immediate_ety(dokotera).unwrap().items[1];
            assert_eq!(t.parents(doctor), ["la:doctor?"]);
            // an inline etymology's alternatives are the missing parent's own
            // ("from Old French aloer, from Latin allaudāre or allocāre")
            t.add("la", "allaudare");
            t.add("la", "allocare");
            let allowance = t.add("en", "allowance");
            t.process(
                allowance,
                r#"[{"name": "etymon", "args": {"1": "en", "2": ":bor",
                    "3": "fro:aloer<ety:inh<la:allaudare><la:allocare>>"}}]"#,
            );
            let aloer = t.parent(allowance);
            assert_eq!(t.parents(aloer), ["la:allaudare", "la:allocare"]);
            let etys = t.items.graph.etymologies(aloer).unwrap();
            assert_eq!((etys.kinds.len(), etys.primary), (2, None));
            // "From X, or possibly from Y": X is primary, Y kept but not followed
            let tiger = t.add("sk", "tiger");
            t.process(
                tiger,
                r#"[{"name": "etymon", "args": {"1": "sk", "2": ":lbor", "3": "la:tigris",
                    "4": ":bor<unc>", "5": "de:Tiger"}}]"#,
            );
            assert_eq!(t.parents(tiger), ["la:tigris?"]);
            assert_eq!(t.items.graph.etymologies(tiger).unwrap().primary, Some(0));
            assert_eq!(t.items.graph.all_parent_edges(tiger).count(), 2);
            // two terms joined by "and" naming one entry ("from *bratrъ and
            // *bratъ"): one parent, the head
            t.add("sla-pro", "bratrъ");
            let bratru = t.add("cu", "братръ");
            t.process(
                bratru,
                r#"[{"name": "etymon", "args": {"1": "cu", "2": ":inh<conj:and>",
                    "3": "sla-pro:*bratrъ", "4": "sla-pro:*bratrъ"}}]"#,
            );
            assert_eq!(t.parents(bratru), ["sla-pro:bratrъ"]);
            assert_eq!(t.items.graph.immediate_ety(bratru).unwrap().head, Some(0));
            // {{unk}}: known to be unknown
            let unk = t.add("ro", "prăștilă");
            t.process(unk, r#"[{"name": "unk", "args": {"1": "ro"}}]"#);
            assert!(t.parents(unk).is_empty());
            assert_eq!(
                t.items.graph.etymologies(unk).unwrap().kinds[..],
                [wety_core::ety_graph::EtyKind::Unknown]
            );
            // after a template, the origin of the missing word before it
            let bird = t.add("en", "bird");
            t.process(
                bird,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "brid"}},
                {"name": "unk", "args": {"1": "en", "nocap": "1"}}]"#,
            );
            let brid = t.parent(bird);
            assert!(t.parents(brid).is_empty());
            assert_eq!(
                t.items.graph.etymologies(brid).unwrap().kinds[..],
                [wety_core::ety_graph::EtyKind::Unknown]
            );
        }

        #[test]
        fn origins_naming_only_a_language() {
            use wety_core::ety_graph::{EtyKind, LanguageOnly};
            let lang = |code: &str| SourceLang::Lang(Lang::from_str(code).unwrap());
            let family = |code: &str| SourceLang::Family(Family::from_str(code).unwrap());
            let only = |source, mode, certainty| {
                EtyKind::LanguageOnly(LanguageOnly {
                    source,
                    mode,
                    certainty,
                })
            };
            let mut t = Setup::new("language only");
            // "From Welsh."
            let aberdaron = t.add("en", "Aberdaron");
            t.process(
                aberdaron,
                r#"[{"name": "bor", "args": {"1": "en", "2": "cy", "3": "-"}}]"#,
            );
            assert!(t.parents(aberdaron).is_empty());
            let etys = t.items.graph.etymologies(aberdaron).unwrap();
            assert_eq!(
                (&etys.kinds[..], etys.primary),
                (
                    &[only(lang("cy"), EtyMode::Borrowed, Certainty::Stated)][..],
                    Some(0)
                )
            );
            // "Borrowed from some Turkic language", a family
            let rak = t.add("mhr", "-рак");
            t.process(
                rak,
                r#"[{"name": "bor", "args": {"1": "mhr", "2": "trk"}}]"#,
            );
            assert_eq!(
                t.items.graph.etymologies(rak).unwrap().kinds[..],
                [only(family("trk"), EtyMode::Borrowed, Certainty::Stated)]
            );
            // "From Latin x, or possibly from a Celtic language": alternatives,
            // the firmer preferred
            t.add("la", "carrus");
            let car = t.add("en", "car");
            t.process(
                car,
                r#"[{"name": "etymon", "args": {"1": "en", "2": ":bor", "3": "la:carrus",
                    "4": "cel:<unc>"}}]"#,
            );
            assert_eq!(t.parents(car), ["la:carrus"]);
            let etys = t.items.graph.etymologies(car).unwrap();
            assert_eq!(
                (&etys.kinds[1..], etys.primary),
                (
                    &[only(family("cel"), EtyMode::Borrowed, Certainty::Possible)][..],
                    Some(0)
                )
            );
            // "From Middle English brid, from Old French": the missing word's
            let bird = t.add("en", "bird");
            t.process(
                bird,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "brid"}},
                {"name": "bor", "args": {"1": "en", "2": "fro", "3": "-"}}]"#,
            );
            let brid = t.parent(bird);
            assert_eq!(
                t.items.graph.etymologies(brid).unwrap().kinds[..],
                [only(lang("fro"), EtyMode::Borrowed, Certainty::Stated)]
            );
            // a word the section names elsewhere still comes first
            t.add("la", "octobris");
            let october = t.add("en", "October");
            t.process(
                october,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "-"}},
                {"name": "der", "args": {"1": "en", "2": "la", "3": "octobris"}}]"#,
            );
            assert_eq!(t.parents(october), ["la:octobris"]);
            assert!(t.items.graph.etymologies(october).is_none());
            // a term shown but not linked is a word: the inline etymology goes
            // on as before, its first alternative
            t.add("la", "-arius");
            let ar = t.add("en", "-ar");
            t.process(
                ar,
                r#"[{"name": "etymon", "args": {"1": "en", "2": ":inh",
                    "3": "enm:<alt:-ar, -er><ety:bor<la:-arius><fro:-aire>>"}}]"#,
            );
            assert_eq!(t.parents(ar), ["la:-arius"]);
            assert!(t.items.graph.etymologies(ar).is_none());
        }

        #[test]
        fn chains_through_missing_parents() {
            let mut t = Setup::new("chains");
            t.add("ang", "mōdor");
            t.add("la", "muttum");
            // within one line of descent
            let mother = t.add("en", "mother");
            t.process(
                mother,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "mōdor"}}]"#,
            );
            assert_eq!(t.parents(mother), ["enm:moder?"]);
            assert_eq!(t.parents(t.parent(mother)), ["ang:mōdor"]);
            // not across a borrowing, unless {{etymon}} says so
            let mot = t.add("en", "mot");
            t.process(
                mot,
                r#"[{"name": "bor", "args": {"1": "en", "2": "fro", "3": "mot"}},
                {"name": "der", "args": {"1": "en", "2": "la", "3": "muttum"}}]"#,
            );
            assert!(t.parents(t.parent(mot)).is_empty());
            let word = t.add("en", "motword");
            t.process(
                word,
                r#"[{"name": "etymon", "args": {"1": "en", "2": ":bor",
                "3": "fro:mot<ety:inh<la:muttum>>"}}]"#,
            );
            assert_eq!(t.parents(t.parent(word)), ["la:muttum"]);
        }

        #[test]
        fn edges_record_what_they_were_read_from() {
            let mut t = Setup::new("bases");
            t.add("ang", "mōdor");
            t.add("la", "muttum");
            t.add("la", "lupus");
            // the classic templates: the first is stated, a later one is read
            // as its missing parent's etymology
            let mother = t.add("en", "mother");
            t.process(
                mother,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "mōdor"}}]"#,
            );
            assert_eq!(t.bases(mother), [EdgeBasis::Template]);
            assert_eq!(t.bases(t.parent(mother)), [EdgeBasis::Chain]);
            // {{etymon}} states its inline etymology too
            let word = t.add("en", "motword");
            t.process(
                word,
                r#"[{"name": "etymon", "args": {"1": "en", "2": ":bor",
                "3": "fro:mot<ety:inh<la:muttum>>"}}]"#,
            );
            assert_eq!(t.bases(word), [EdgeBasis::Etymon]);
            assert_eq!(t.bases(t.parent(word)), [EdgeBasis::Etymon]);
            // a lone {{m}} is read as the parent
            let lupine = t.add("en", "lupine");
            t.process(
                lupine,
                r#"[{"name": "m", "args": {"1": "la", "2": "lupus"}}]"#,
            );
            assert_eq!(t.bases(lupine), [EdgeBasis::Mention]);
            assert!(EdgeBasis::Mention.is_rule() && !EdgeBasis::Etymon.is_rule());
        }

        #[test]
        fn own_etymology_and_descendants_lists() {
            let mut t = Setup::new("own");
            let octo = t.add("la", "Octōber");
            let octobre = t.add("fro", "octobre");
            let october = t.add("en", "October");
            t.items.graph.add_ety(
                octobre,
                EtyMode::Inherited,
                Some(0),
                EdgeBasis::Descendants,
                &[octo],
                &[0.5],
            );
            // a Descendants list gave the more specific parent: kept
            t.items.graph.add_ety(
                october,
                EtyMode::Borrowed,
                Some(0),
                EdgeBasis::Descendants,
                &[octobre],
                &[0.5],
            );
            t.process(
                october,
                r#"[{"name": "der", "args": {"1": "en", "2": "la", "3": "Octōber"}}]"#,
            );
            assert_eq!(t.parents(october), ["fro:octobre"]);
            // a derivative of the parent in its own language: kept where the
            // section names only a base it was derived from...
            let arbor = t.add("la", "arbor");
            let arboreus = t.add("la", "arboreus");
            let arboreous = t.add("en", "arboreous");
            t.items.graph.add_ety(
                arboreus,
                EtyMode::Suffix,
                Some(0),
                EdgeBasis::Template,
                &[arbor],
                &[0.5],
            );
            t.items.graph.add_ety(
                arboreous,
                EtyMode::Borrowed,
                Some(0),
                EdgeBasis::Descendants,
                &[arboreus],
                &[0.5],
            );
            t.process(
                arboreous,
                r#"[{"name": "der", "args": {"1": "en", "2": "la", "3": "arbor"}}]"#,
            );
            assert_eq!(t.parents(arboreous), ["la:arboreus"]);
            // ...but not where it says the item was inherited from that word
            let table = t.add("enm", "table");
            let tablen = t.add("enm", "tablen");
            let table_en = t.add("en", "table");
            t.items.graph.add_ety(
                tablen,
                EtyMode::Suffix,
                Some(0),
                EdgeBasis::Template,
                &[table],
                &[0.5],
            );
            t.items.graph.add_ety(
                table_en,
                EtyMode::Inherited,
                Some(0),
                EdgeBasis::Descendants,
                &[tablen],
                &[0.5],
            );
            t.process(
                table_en,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "table"}}]"#,
            );
            assert_eq!(t.parents(table_en), ["enm:table"]);
            // a different parent: the item's own section wins
            let enm = t.add("enm", "Octobre");
            let october2 = t.add("en", "Octobre");
            t.items.graph.add_ety(
                october2,
                EtyMode::Borrowed,
                Some(0),
                EdgeBasis::Descendants,
                &[octo],
                &[0.9],
            );
            t.process(
                october2,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "Octobre"}}]"#,
            );
            assert_eq!(t.parents(october2), ["enm:Octobre"]);
            // but not for a term missing from the data
            t.items.graph.remove_ety(october2);
            t.items.graph.add_ety(
                october2,
                EtyMode::Inherited,
                Some(0),
                EdgeBasis::Descendants,
                &[enm],
                &[0.5],
            );
            t.process(
                october2,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "Octobere"}}]"#,
            );
            assert_eq!(t.parents(october2), ["enm:Octobre"]);
            // alternatives, one of them missing from the data, still win:
            // the other keeps to an entry
            let ang = t.add("ang", "Æþelbeorht");
            let albert = t.add("en", "Albert");
            t.items.graph.add_ety(
                albert,
                EtyMode::Borrowed,
                Some(0),
                EdgeBasis::Descendants,
                &[ang],
                &[0.9],
            );
            t.process(
                albert,
                r#"[{"name": "etymon", "args": {"1": "en", "2": ":inh", "3": "ang:Æþelbeorht",
                    "4": ":der", "5": "fro:Albert"}}]"#,
            );
            assert_eq!(t.parents(albert), ["ang:Æþelbeorht", "fro:Albert?"]);
            // so do parents joined by "and", some of them missing
            let litany = t.add("en", "litany");
            let grc = t.add("grc", "λιτανεία");
            let litanio = t.add("eo", "litanio");
            t.items.graph.add_ety(
                litanio,
                EtyMode::Borrowed,
                Some(0),
                EdgeBasis::Descendants,
                &[grc],
                &[0.9],
            );
            t.process(
                litanio,
                r#"[{"name": "etymon", "args": {"1": "eo", "2": ":bor<conj:and>", "3": "en:litany",
                    "4": "pl:litania"}}]"#,
            );
            assert_eq!(t.parents(litanio), ["en:litany", "pl:litania?"]);
            assert_eq!(t.parent(litanio), litany);
        }

        #[test]
        fn assertions_record_their_page_and_footnotes() {
            let mut t = Setup::new("asserted");
            t.add("ang", "mōdor");
            let mother = t.add("en", "mother");
            t.process_entry(
                mother,
                r#"{"etymology_templates": [
                    {"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}},
                    {"name": "inh", "args": {"1": "en", "2": "ang", "3": "mōdor"}}],
                "etymology": {
                    "blocks": [{"kind": "paragraph", "text": "From moder, from mōdor.", "spans": [
                        {"kind": "ref", "start": 23, "note": "cite_note-1"}]}],
                    "notes": [{"id": "cite_note-1", "text": "Etymologicon (1999), page 3", "calls": []}]}}"#,
            );
            // stated on its own page, under the section's footnote
            let own = t.items.graph.asserted(mother, 0).unwrap();
            assert_eq!(own.page, None);
            let [credit] = &own.credits[..] else {
                panic!("one credit")
            };
            assert_eq!((credit.role, credit.scope), (Role::Follows, Scope::Note));
            assert_eq!(credit.locator.as_deref(), Some("page 3"));
            let work = &t.items.graph.works[credit.work as usize];
            assert_eq!(
                (&*work.citation, work.year),
                ("Etymologicon (1999)", Some(1999))
            );
            // the missing parent's etymology: stated in mother's section
            let moder = t.parent(mother);
            let chain = t.items.graph.asserted(moder, 0).unwrap();
            assert_eq!((chain.page, chain.credits.len()), (Some(mother), 1));
            // an uncredited section on its own page stores nothing
            let father = t.add("en", "father");
            t.process(
                father,
                r#"[{"name": "inh", "args": {"1": "en", "2": "ang", "3": "mōdor"}}]"#,
            );
            assert!(t.items.graph.asserted(father, 0).is_none());
        }

        #[test]
        fn entries_credit_their_references() {
            use std::io::Write;
            let dir = std::env::temp_dir().join("wety-references-test");
            std::fs::create_dir_all(&dir).unwrap();
            let sections = dir.join("section_credits.jsonl.gz");
            let mut f = flate2::write::GzEncoder::new(
                std::fs::File::create(&sections).unwrap(),
                flate2::Compression::default(),
            );
            // one section under Etymology 1, one beside the etymologies
            writeln!(
                f,
                r#"{{"page": "lupus", "lang": "Latin", "ety": "1", "credits": [{{"work": "work:wiktionary:R:itc:EDL", "role": "follows", "scope": "entry", "locator": "353"}}]}}
{{"page": "lupus", "lang": "Latin", "ety": null, "credits": [{{"work": "work:wiktionary:R:la:Gaffiot", "role": "follows", "scope": "entry"}}]}}"#
            )
            .unwrap();
            f.finish().unwrap();
            let bibliography = dir.join("bibliography.jsonl");
            std::fs::write(
                &bibliography,
                r#"{"type": "work", "id": "work:wiktionary:R:itc:EDL", "citation": "De Vaan (2008)", "year": 2008}
{"type": "work", "id": "work:wiktionary:R:la:Gaffiot", "citation": "Gaffiot (1934)", "year": 1934}
"#,
            )
            .unwrap();
            let mut t = Setup::new("references");
            t.items.references = References::load(&sections, &bibliography).unwrap();
            t.add("itc-pro", "lukʷos");
            let entry = |t: &mut Setup, label: &str| {
                let item = t
                    .items
                    .add_real(RealItem {
                        ety_num: label.parse().unwrap(),
                        ety_label: Some(label.into()),
                        lang: Lang::from_str("la").unwrap(),
                        term: Term::new(&mut t.sp, "lupus"),
                        pos: vec![Pos::from_str("noun").unwrap()],
                        gloss: vec![Gloss::new(&mut t.sp, "x")],
                        page_term: None,
                        romanization: None,
                        is_reconstructed: false,
                    })
                    .0;
                let json = format!(
                    r#"{{"page": "lupus", "lang_heading": "Latin", "ety_label": "{label}"}}"#
                );
                test_entry::with(&json, |e| t.items.references.cover(item, e));
                item
            };
            let one = entry(&mut t, "1");
            let two = entry(&mut t, "2");
            t.process(
                one,
                r#"[{"name": "inh", "args": {"1": "la", "2": "itc-pro", "3": "*lukʷos"}}]"#,
            );
            t.process(two, r#"[{"name": "unk", "args": {"1": "la"}}]"#);
            let works = |item| {
                let asserted = t.items.graph.asserted(item, 0).unwrap();
                asserted
                    .credits
                    .iter()
                    .map(|c| {
                        assert_eq!((c.role, c.scope), (Role::Follows, Scope::Entry));
                        (
                            &*t.items.graph.works[c.work as usize].id,
                            c.locator.as_deref(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                works(one),
                [
                    ("work:wiktionary:R:itc:EDL", Some("353")),
                    ("work:wiktionary:R:la:Gaffiot", None)
                ]
            );
            // an unknown origin is credited too; each work is one in the graph
            assert_eq!(works(two), [("work:wiktionary:R:la:Gaffiot", None)]);
            assert_eq!(t.items.graph.works.len(), 2);
        }

        #[test]
        fn citations_of_a_missing_term_share_it() {
            let mut t = Setup::new("shared");
            t.add("ang", "mōdor");
            let mother = t.add("en", "mother");
            let motherly = t.add("en", "motherly");
            t.process(
                mother,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "mōdor"}}]"#,
            );
            t.process(
                motherly,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "moder"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "modor"}}]"#,
            );
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
            t.process(
                father,
                r#"[{"name": "etymon", "args": {"1": "ine-pro", "2": ":af",
                "3": "*peh₂-", "4": "*-tḗr"}}]"#,
            );
            let ety = t.items.graph.immediate_ety(father).unwrap();
            assert_eq!((ety.mode, ety.head), (EtyMode::Suffix, Some(0)));
        }

        #[test]
        fn a_root_takes_no_root() {
            // sla-pro *bȍrъ < (missing) *bʰor-; its root categories name
            // *dʰeh₁- as well, which *bʰor-, a root itself, does not descend from
            let mut t = Setup::new("root-of-root");
            let dheh = t.add("ine-pro", "dʰeh₁-");
            let bor = t.add("sla-pro", "bȍrъ");
            t.process(
                bor,
                r#"[{"name": "inh", "args": {"1": "sla-pro", "2": "ine-pro", "3": "*bʰor-"}}]"#,
            );
            let bhor = t.parent(bor);
            assert!(t.items.get(bhor).is_imputed());
            let langterm = t.items.get(dheh).lang().new_langterm(&mut t.sp, "dʰeh₁-");
            t.items.raw_templates.root.insert(
                bor,
                Box::new([crate::root::RawRoot {
                    langterm,
                    sense_id: None,
                }]),
            );
            t.items.impute_root_etys(&t.sp, &t.embeddings).unwrap();
            assert!(t.items.graph.immediate_ety(bhor).is_none());
        }

        // A PIE root entry; homograph roots on one page stay separate items.
        fn add_root(t: &mut Setup, term: &str, id: &str) -> ItemId {
            let lang = Lang::from_str("ine-pro").unwrap();
            let item = RealItem {
                ety_num: 1,
                ety_label: None,
                lang,
                term: Term::new(&mut t.sp, term),
                pos: vec![Pos::root_pos()],
                gloss: vec![Gloss::new(&mut t.sp, id)],
                page_term: None,
                romanization: None,
                is_reconstructed: true,
            };
            let root = t.items.add_real(item).0;
            let langterm = lang.new_langterm(&mut t.sp, term);
            t.items.ids.insert((langterm, Box::from(id)), root);
            root
        }

        fn set_roots(t: &mut Setup, item: ItemId, roots: &[(&str, Option<&str>)]) {
            let lang = Lang::from_str("ine-pro").unwrap();
            let roots = roots
                .iter()
                .map(|&(term, sense)| crate::root::RawRoot {
                    langterm: lang.new_langterm(&mut t.sp, term),
                    sense_id: sense.map(Box::from),
                })
                .collect();
            t.items.raw_templates.root.insert(item, roots);
        }

        #[test]
        fn a_prefix_root_does_not_root_the_base() {
            // la testum has no ety; en test names its root, *ters-. Derivatives
            // of test also name the roots of their prefixes, which their
            // prefixes reach as another homograph (pl prze- < *per- (before),
            // named as *per- (fare)) or partway up (en pre- < *preh₂- < *per-).
            let mut t = Setup::new("prefix-roots");
            let per_before = add_root(&mut t, "per-", "before");
            add_root(&mut t, "per-", "fare");
            let preh = add_root(&mut t, "preh₂-", "before");
            add_root(&mut t, "ters-", "dry");
            let testum = t.add("la", "testum");
            let test = t.add("en", "test");
            let pre = t.add("en", "pre-");
            let pretest = t.add("en", "pretest");
            let prze = t.add("pl", "prze-");
            let przetest = t.add("pl", "przetestować");
            let g = &mut t.items.graph;
            g.add_ety(
                test,
                EtyMode::Derived,
                Some(0),
                EdgeBasis::Template,
                &[testum],
                &[1.0],
            );
            g.add_ety(
                preh,
                EtyMode::Derived,
                Some(0),
                EdgeBasis::Template,
                &[per_before],
                &[1.0],
            );
            g.add_ety(
                pre,
                EtyMode::Derived,
                Some(0),
                EdgeBasis::Template,
                &[preh],
                &[1.0],
            );
            g.add_ety(
                pretest,
                EtyMode::Prefix,
                Some(1),
                EdgeBasis::Template,
                &[pre, test],
                &[1.0, 1.0],
            );
            g.add_ety(
                prze,
                EtyMode::Inherited,
                Some(0),
                EdgeBasis::Template,
                &[per_before],
                &[1.0],
            );
            g.add_ety(
                przetest,
                EtyMode::Prefix,
                Some(1),
                EdgeBasis::Template,
                &[prze, test],
                &[1.0, 1.0],
            );
            set_roots(&mut t, test, &[("ters-", None)]);
            set_roots(
                &mut t,
                pretest,
                &[("per-", Some("before")), ("preh₂-", None)],
            );
            set_roots(
                &mut t,
                przetest,
                &[("per-", Some("before")), ("per-", Some("fare"))],
            );
            t.items.impute_root_etys(&t.sp, &t.embeddings).unwrap();
            assert_eq!(t.parents(testum), ["ine-pro:ters-"]);
        }

        #[test]
        fn root_votes_keep_a_headless_etymology() {
            // de anderweit = ander + weit has no head, so the head chain of a
            // derivative stops at it, but its own etymology is no gap to fill
            let mut t = Setup::new("headless-roots");
            add_root(&mut t, "dʰeh₁-", "put");
            let ander = t.add("de", "ander");
            let weit = t.add("de", "weit");
            let anderweit = t.add("de", "anderweit");
            let anderweitig = t.add("de", "anderweitig");
            let g = &mut t.items.graph;
            g.add_ety(
                anderweit,
                EtyMode::Compound,
                None,
                EdgeBasis::Etymon,
                &[ander, weit],
                &[0.5, 0.5],
            );
            g.add_ety(
                anderweitig,
                EtyMode::Suffix,
                Some(0),
                EdgeBasis::Template,
                &[anderweit],
                &[1.0],
            );
            set_roots(&mut t, anderweitig, &[("dʰeh₁-", None)]);
            t.items.impute_root_etys(&t.sp, &t.embeddings).unwrap();
            assert_eq!(t.parents(anderweit), ["de:ander", "de:weit"]);
        }

        #[test]
        fn a_root_another_part_may_carry_is_no_vote_for_the_head() {
            // mul Lycosa = grc λύκος + la -ōsus, filed under *h₃ed- (smell)
            // for the suffix; neither part has an etymology, so the category
            // may be either's and roots neither
            let mut t = Setup::new("other-part-roots");
            add_root(&mut t, "h₃ed-", "smell");
            let lukos = t.add("grc", "λύκος");
            let osus = t.add("la", "-ōsus");
            let lycosa = t.add("mul", "Lycosa");
            t.items.graph.add_ety(
                lycosa,
                EtyMode::Suffix,
                Some(0),
                EdgeBasis::Template,
                &[lukos, osus],
                &[1.0, 1.0],
            );
            set_roots(&mut t, lycosa, &[("h₃ed-", None)]);
            t.items.impute_root_etys(&t.sp, &t.embeddings).unwrap();
            assert!(t.items.graph.immediate_ety(lukos).is_none());
            assert!(t.items.graph.immediate_ety(osus).is_none());
        }

        #[test]
        fn tied_root_votes_impute_none() {
            // one item naming two roots its chain does not reach: either may be
            // its progenitor's
            let mut t = Setup::new("tied-roots");
            add_root(&mut t, "per-", "before");
            add_root(&mut t, "ters-", "dry");
            let testum = t.add("la", "testum");
            let test = t.add("en", "test");
            t.items.graph.add_ety(
                test,
                EtyMode::Derived,
                Some(0),
                EdgeBasis::Template,
                &[testum],
                &[1.0],
            );
            set_roots(&mut t, test, &[("per-", None), ("ters-", None)]);
            t.items.impute_root_etys(&t.sp, &t.embeddings).unwrap();
            assert!(t.items.graph.immediate_ety(testum).is_none());
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
            t.record(
                barn_building,
                r#"[{"name": "af", "args": {"1": "enm", "2": "bere", "3": "-ern"}}]"#,
            );
            t.record(
                man,
                r#"[{"name": "inh", "args": {"1": "enm", "2": "ang", "3": "beorn"}}]"#,
            );
            t.record(
                child,
                r#"[{"name": "inh", "args": {"1": "enm", "2": "ang", "3": "bearn"}}]"#,
            );
            let bairn = t.add_glossed("en", "bairn", "a child or baby");
            t.process(
                bairn,
                r#"[{"name": "der", "args": {"1": "en", "2": "enm", "3": "bern"}},
                {"name": "der", "args": {"1": "en", "2": "ang", "3": "bearn"}}]"#,
            );
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
            t.record(
                man,
                r#"[{"name": "inh", "args": {"1": "enm", "2": "ang", "3": "beorn"}}]"#,
            );
            let bairn = t.add("en", "bairn");
            t.process(
                bairn,
                r#"[{"name": "der", "args": {"1": "en", "2": "enm", "3": "bern"}},
                {"name": "der", "args": {"1": "en", "2": "ang", "3": "bearn"}}]"#,
            );
            assert_eq!(t.parents(bairn), ["enm:bern?"]);
            assert_eq!(t.parents(t.parent(bairn)), ["ang:bearn"]);
            // nor is one alike in meaning (a variant with its own entry)
            t.add_glossed("ang", "amǣnsumian", "to excommunicate");
            t.add_glossed("ang", "amānsumian", "to excommunicate");
            let amansen = t.add("enm", "amansen");
            t.record(
                amansen,
                r#"[{"name": "inh", "args": {"1": "enm", "2": "ang", "3": "amānsumian"}}]"#,
            );
            let amanse = t.add("en", "amanse");
            t.process(
                amanse,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "amansen"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "amǣnsumian"}}]"#,
            );
            assert_eq!(t.parent(amanse), amansen);
            // a part of a word is not checked against the next template
            t.add("la", "re-");
            t.add("la", "pullus");
            let re = t.add("gl", "re-");
            t.record(
                re,
                r#"[{"name": "inh", "args": {"1": "gl", "2": "la", "3": "re-"}}]"#,
            );
            let repolo = t.add("gl", "repolo");
            t.process(
                repolo,
                r#"[{"name": "affix", "args": {"1": "gl", "2": "re-"}},
                {"name": "inh", "args": {"1": "gl", "2": "la", "3": "pullus"}}]"#,
            );
            assert_eq!(t.parents(repolo), ["gl:re-"]);
            // a spelling variant of the source is not a contradiction
            let mut t = Setup::new("variant");
            let freo = t.add("ang", "frēo");
            let freoh = t.add("ang", "frēoh");
            t.items.graph.add_ety(
                freoh,
                EtyMode::Form,
                Some(0),
                EdgeBasis::FormOf,
                &[freo],
                &[1.0],
            );
            let fre = t.add("enm", "fre");
            t.record(
                fre,
                r#"[{"name": "inh", "args": {"1": "enm", "2": "ang", "3": "frēoh"}}]"#,
            );
            let free = t.add("en", "free");
            t.process(
                free,
                r#"[{"name": "inh", "args": {"1": "en", "2": "enm", "3": "fre"}},
                {"name": "inh", "args": {"1": "en", "2": "ang", "3": "frēo"}}]"#,
            );
            assert_eq!(t.parent(free), fre);
        }

        #[test]
        fn candidates_are_the_homographs_on_the_page() {
            // Middle Dutch "dië" (thigh) has its headword on the page "die"
            let mut t = Setup::new("page");
            t.add("odt", "thie");
            t.add("odt", "thio");
            let that = t.add_glossed("dum", "die", "that, those");
            t.record(
                that,
                r#"[{"name": "inh", "args": {"1": "dum", "2": "odt", "3": "thie"}}]"#,
            );
            let thigh = t.add_on_page("dum", "dië", Some("die"));
            t.record(
                thigh,
                r#"[{"name": "inh", "args": {"1": "dum", "2": "odt", "3": "thio", "4": "thīo"}}]"#,
            );
            let dij = t.add_glossed("nl", "dij", "thigh");
            t.process(
                dij,
                r#"[{"name": "inh", "args": {"1": "nl", "2": "dum", "3": "die"}},
                {"name": "inh", "args": {"1": "nl", "2": "odt", "3": "*thio"}}]"#,
            );
            assert_eq!(t.parent(dij), thigh);
        }

        #[test]
        fn only_an_entry_s_own_etymology_contradicts() {
            let mut t = Setup::new("contradicts");
            // a link from another entry's Descendants list
            t.add("trk-pro", "*bȫrü");
            let flea = t.add("trk-pro", "*bürgä");
            let wolf = t.add("tt", "бүре");
            t.items.graph.add_ety(
                wolf,
                EtyMode::Inherited,
                Some(0),
                EdgeBasis::Template,
                &[flea],
                &[1.0],
            );
            let mari = t.add("mhr", "пире");
            t.process(
                mari,
                r#"[{"name": "bor", "args": {"1": "mhr", "2": "tt", "3": "бүре"}},
                {"name": "der", "args": {"1": "mhr", "2": "trk-pro", "3": "*bȫrü"}}]"#,
            );
            assert_eq!(t.parent(mari), wolf);
            // spellings that differ only in diacritics or hyphens
            t.add("goh", "selah");
            t.add("goh", "sëlah");
            let sele = t.add("gmh", "sële");
            t.record(
                sele,
                r#"[{"name": "inh", "args": {"1": "gmh", "2": "goh", "3": "sëlah"}}]"#,
            );
            let seehund = t.add("de", "Seehund");
            t.process(
                seehund,
                r#"[{"name": "der", "args": {"1": "de", "2": "gmh", "3": "sële"}},
                {"name": "der", "args": {"1": "de", "2": "goh", "3": "selah"}}]"#,
            );
            assert_eq!(t.parents(seehund), ["gmh:sële"]);
            t.add("la", "ad-");
            t.add("la", "ad");
            let a = t.add("fro", "a-");
            t.record(
                a,
                r#"[{"name": "inh", "args": {"1": "fro", "2": "la", "3": "ad-"}}]"#,
            );
            let amolir = t.add("enm", "amolisshen");
            t.process(
                amolir,
                r#"[{"name": "der", "args": {"1": "enm", "2": "fro", "3": "a-"}},
                {"name": "der", "args": {"1": "enm", "2": "la", "3": "ad"}}]"#,
            );
            assert_eq!(t.parents(amolir), ["fro:a-"]);
            // a root names a family, not the next step
            t.add("ine-pro", "*bʰer-");
            t.add("ine-pro", "*bʰerǵ-");
            let frigo = t.add("la", "frīgō");
            t.record(
                frigo,
                r#"[{"name": "der", "args": {"1": "la", "2": "ine-pro", "3": "*bʰerǵ-"}}]"#,
            );
            let frighere = t.add("sc", "frighere");
            t.process(
                frighere,
                r#"[{"name": "inh", "args": {"1": "sc", "2": "la", "3": "frīgō"}},
                {"name": "der", "args": {"1": "sc", "2": "ine-pro", "3": "*bʰer-"}}]"#,
            );
            assert_eq!(t.parents(frighere), ["la:frīgō"]);
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
            t.process(
                quick,
                r#"[{"name": "af", "args": {"1": "en", "2": "lime<id:quicklime>", "3": "wash"}}]"#,
            );
            assert_eq!(t.parent(quick), mineral);
        }

        #[test]
        fn an_alternative_form_cites_its_lemma_with_the_gloss() {
            // the gloss decides between enm barn "child" and barn "barn"
            let mut sp = StringPool::new();
            let json = r#"{"pos": [{"senses": [{"gloss": "alternative form of barn (“child”)",
                "form_of": [{"lang": "enm", "term": "barn", "gloss": "child"}]}]}]}"#;
            let lang = Lang::from_str("enm").unwrap();
            let ety = test_entry::with(json, |e| {
                e.get_etymology(&mut sp, lang, &References::default())
            })
            .unwrap();
            let ParsedRawEtyTemplate::Parsed(template) = &ety.alternatives[0][0] else {
                panic!("not parsed");
            };
            assert_eq!(template.mode, EtyMode::Form);
            assert_eq!(template.langterms[0].term.resolve(&sp), "barn");
            assert_eq!(template.parts[0].gloss.as_deref(), Some("child"));
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
            t.process(
                limekiln,
                r#"[{"name": "af", "args": {"1": "en", "2": "lime", "3": "kiln"}}]"#,
            );
            assert_eq!(t.parent(limekiln), mineral);
        }

        #[test]
        fn descendants_lists_settle_homographs() {
            let mut t = Setup::new("homographs");
            let _lime1 = t.add("en", "lime");
            let lime2 = t.add("en", "lime");
            let limey = t.add("en", "limey");
            t.items.graph.add_ety(
                limey,
                EtyMode::MorphologicalDerivation,
                Some(0),
                EdgeBasis::Template,
                &[lime2],
                &[0.1],
            );
            t.process(
                limey,
                r#"[{"name": "af", "args": {"1": "en", "2": "lime", "3": "-y"}}]"#,
            );
            assert_eq!(t.items.graph.immediate_ety(limey).unwrap().items[0], lime2);
        }
    }
}
