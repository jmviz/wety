//! Parsing of https://en.wiktionary.org/wiki/Template:etymon (shortcut {{ety}}),
//! following https://en.wiktionary.org/wiki/Module:etymon.
//!
//! "1" is the entry's language and "id" names the entry's own etymology. The
//! positional args "2", "3", ... are a sequence of derivation keywords (":inh",
//! ":bor", ":af", ...) and etymons. A keyword applies to the etymons after it
//! until the next keyword; etymons before any keyword are ":from" (same-language
//! derivation). An etymon is `lang:term<mod:value>...`, where the lang prefix
//! defaults to "1" and the inline modifiers include `<id:...>` (the target
//! etymology's id), `<aftype:...>` (affix type under ":af") and
//! `<ety:keyword<etymon>...>` (the etymon's own etymology, given inline when
//! its page is missing). A keyword takes modifiers too: `<unc>` (the derivation
//! is uncertain) and `<conj:...>` (how its terms are joined).
//!
//! An entry's {{etymon}} gives one or more etymologies, alternatives the entry
//! offers (see parse_params). Each becomes a chain of `ParsedRawEtyTemplate`s,
//! as the classic templates do: the first is the entry's parent(s), and the
//! rest come from the inline etymology of a single parent, to be used if that
//! parent has to be imputed.

use crate::{
    entries::{
        Affix, Json, JsonStr, affix_by_hyphens, affix_type, clean_template_term, modifier,
        parse_pos, split_lang_prefix, split_modifiers,
    },
    etymology::{ParsedRawEtyTemplate, Part, RawEtyTemplate, affixation_kind},
};
use wety_core::{
    ety_graph::{Certainty, LanguageOnly, SourceLang},
    etymology_templates::EtyMode,
    languages::{Family, Lang},
    string_pool::StringPool,
};

use std::str::FromStr;

// Inline etymologies can nest; bound the recursion.
const MAX_INLINE_DEPTH: usize = 8;

#[derive(Debug, PartialEq, Clone, Copy)]
enum Keyword {
    // The keyword's etymons are alternatives (e.g. "borrowed from Spanish or
    // Portuguese ..."); like the classic templates, we take the first.
    Single(EtyMode),
    // The keyword's etymons are all components, in order. A mode of None
    // means it is to be refined from the affix types, as for {{af}}.
    Components(Option<EtyMode>),
    // Categorization-only or non-genealogical keywords: ":root" (handled
    // through the root categories it generates), ":afeq" (a hidden surface
    // analysis) and ":influence".
    Ignored,
}

// The keywords and aliases of Module:etymon/data.
fn keyword(name: &str) -> Option<Keyword> {
    use EtyMode as M;
    use Keyword::{Components, Ignored, Single};
    Some(match name {
        // ":from" is the default: an unspecified derivation within the language.
        "from" | "denominal" | "denom" | "metathesis" | "meta" => {
            Single(M::MorphologicalDerivation)
        }
        "inherited" | "inh" => Single(M::Inherited),
        "derived" | "der" => Single(M::Derived),
        "uder" => Single(M::UndefinedDerivation),
        "bor" | "borrowed" => Single(M::Borrowed),
        "lbor" => Single(M::LearnedBorrowing),
        "slbor" | "slb" => Single(M::SemiLearnedBorrowing),
        "obor" => Single(M::OrthographicBorrowing),
        "ubor" => Single(M::UnadaptedBorrowing),
        "calque" | "cal" | "clq" => Single(M::Calque),
        "partial calque" | "pcal" | "pclq" => Single(M::PartialCalque),
        "semantic loan" | "sl" => Single(M::SemanticLoan),
        "psm" | "phono-semantic matching" => Single(M::PhonoSemanticMatching),
        "transliteration" | "translit" => Single(M::Transliteration),
        "clipping" | "clip" => Single(M::Clipping),
        "ellipsis" | "ellip" => Single(M::Ellipsis),
        "back-formation" | "bf" => Single(M::BackFormation),
        "nominalization" | "nom" => Single(M::Nominalization),
        "deverbal" => Single(M::Deverbal),
        "apheretic" | "apheresis" | "aphetic" => Single(M::ApheticForm),
        "reduplication" | "rdp" | "redup" => Single(M::Reduplication),
        "abbreviation" | "abbr" | "abbrev" | "syllabic abbreviation" | "sylabbr" | "sylabbrev" => {
            Single(M::Abbreviation)
        }
        "acronym" | "acro" => Single(M::Acronym),
        "initialism" | "init" => Single(M::Initialism),
        "vrd" => Single(M::Vrddhi),
        "affix" | "af" | "sa-af" | "vrd-af" => Components(None),
        "blend" => Components(Some(M::Blend)),
        "univerbation" | "univ" => Components(Some(M::Univerbation)),
        "root" | "afeq" | "influence" => Ignored,
        _ => return None,
    })
}

// A positional arg: a keyword (without its colon) and its modifiers, or an
// etymon.
#[derive(Debug, PartialEq)]
enum Param<'a> {
    Keyword(&'a str, Vec<&'a str>),
    Etymon(&'a str),
}

fn top_level_param(arg: &str) -> Param<'_> {
    match arg.strip_prefix(':') {
        Some(keyword) => {
            let (name, modifiers) = split_modifiers(keyword);
            Param::Keyword(name.trim(), modifiers)
        }
        None => Param::Etymon(arg),
    }
}

// Whether a boolean modifier is set: `<unc>` or `<unc:1>`.
fn flag(modifiers: &[&str], name: &str) -> bool {
    modifiers.iter().any(|m| {
        *m == name
            || m.strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(':'))
                .is_some_and(|v| !matches!(v.trim(), "" | "0" | "no" | "false"))
    })
}

/// The params of an inline etymology `keyword<<kwmod>><etymon><etymon>:keyword2<etymon>`.
fn inline_params(s: &str) -> Vec<Param<'_>> {
    let (keyword, _) = split_modifiers(s);
    let mut params = vec![Param::Keyword(
        keyword.trim().trim_start_matches(':'),
        vec![],
    )];
    let mut rest = &s[keyword.len()..];
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<<") {
            // a keyword modifier, e.g. <<unc>>
            let (modifier, r) = after.split_once(">>").unwrap_or((after, ""));
            if let Some(Param::Keyword(_, modifiers)) = params
                .iter_mut()
                .rev()
                .find(|p| matches!(p, Param::Keyword(..)))
            {
                modifiers.push(modifier);
            }
            rest = r;
        } else if rest.starts_with('<') {
            let mut depth = 0usize;
            let mut end = rest.len();
            for (i, c) in rest.char_indices() {
                match c {
                    '<' => depth += 1,
                    '>' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let cell = &rest[1..end];
            if !cell.is_empty() {
                params.push(Param::Etymon(cell));
            }
            rest = rest.get(end + 1..).unwrap_or("");
        } else {
            let bare_end = rest.find('<').unwrap_or(rest.len());
            let bare = rest[..bare_end].trim();
            if let Some(keyword) = bare.strip_prefix(':') {
                params.push(Param::Keyword(keyword.trim(), vec![]));
            }
            rest = &rest[bare_end..];
        }
    }
    params
}

#[derive(Debug, PartialEq)]
pub(crate) struct Etymon<'a> {
    pub(crate) lang: Lang,
    // the language or family its prefix names, if it names one ("cy:",
    // "trk:-"); a family only with no term
    source: Option<SourceLang>,
    // None for an unknown ("", "+") or suppressed ("-") term
    pub(crate) term: Option<&'a str>,
    pub(crate) id: Option<&'a str>,
    gloss: Option<&'a str>,
    pos: Option<&'a str>,
    aftype: Option<&'a str>,
    ety: Option<&'a str>,
    // <unc>: the source is unsure of this term's part in the derivation
    uncertain: bool,
}

/// Parse `lang:term<mod:value>...`. Returns None if the prefix is not a
/// language code we know, unless it is a family's and names no term ("trk:",
/// "from some Turkic language").
pub(crate) fn parse_etymon(param: &str, context_lang: Lang) -> Option<Etymon<'_>> {
    fn clean(term: &str) -> Option<&str> {
        let term = clean_template_term(term.trim());
        (!matches!(term, "" | "+" | "-")).then_some(term)
    }
    let (head, modifiers) = split_modifiers(param);
    let (lang, source, term) = match split_lang_prefix(head) {
        Some((code, term)) => match Lang::from_str(code) {
            Ok(lang) => (lang, Some(SourceLang::Lang(lang)), clean(term)),
            Err(_) => {
                let family = Family::from_str(code).ok()?;
                let None = clean(term) else {
                    return None;
                };
                (context_lang, Some(SourceLang::Family(family)), None)
            }
        },
        None => (context_lang, None, clean(head)),
    };
    // a term shown but not linked (<alt:...>) is a word, not only a language
    let source = source.filter(|_| modifier(&modifiers, "alt").is_none());
    Some(Etymon {
        lang,
        source,
        term,
        id: modifier(&modifiers, "id")
            .map(|id| id.trim_start_matches('!'))
            .filter(|id| !id.is_empty()),
        gloss: modifier(&modifiers, "t").filter(|t| !t.is_empty()),
        pos: modifier(&modifiers, "pos"),
        aftype: modifier(&modifiers, "aftype"),
        ety: modifier(&modifiers, "ety").filter(|ety| !ety.is_empty()),
        uncertain: flag(&modifiers, "unc"),
    })
}

impl Etymon<'_> {
    fn part(&self) -> Part {
        Part {
            id: self.id.map(Box::from),
            gloss: self.gloss.map(Box::from),
            pos: self.pos.and_then(parse_pos),
            uncertain: self.uncertain,
        }
    }
}

fn affix_kind(etymon: &Etymon, term: &str) -> Affix {
    etymon
        .aftype
        .and_then(affix_type)
        .unwrap_or_else(|| affix_by_hyphens(term))
}

fn components_template(
    string_pool: &mut StringPool,
    etymons: &[Etymon],
    mode: Option<EtyMode>,
    certainty: Certainty,
) -> Option<RawEtyTemplate> {
    let mut langterms = Vec::with_capacity(etymons.len());
    let mut parts = Vec::with_capacity(etymons.len());
    let mut affixes = Vec::with_capacity(etymons.len());
    let mut head = 0;
    let mut n_base_terms = 0;
    for (i, etymon) in etymons.iter().enumerate() {
        let term = etymon.term?;
        let affix = affix_kind(etymon, term);
        if affix == Affix::Base {
            n_base_terms += 1;
            head = i;
        }
        affixes.push(affix);
        langterms.push(etymon.lang.new_langterm(string_pool, term));
        parts.push(etymon.part());
    }
    (!langterms.is_empty()).then(|| RawEtyTemplate {
        langterms: langterms.into_boxed_slice(),
        parts: parts.into_boxed_slice(),
        continues: false,
        refine_mode: mode.is_none(),
        affixes: affixes.clone().into_boxed_slice(),
        mode: mode.unwrap_or_else(|| affixation_kind(&affixes, n_base_terms)),
        // as for {{af}}, a head only when there is a single base term
        head: (n_base_terms == 1)
            .then(|| u8::try_from(head).ok())
            .flatten(),
        certainty,
    })
}

// One etymon after a keyword that takes one, as a template.
// An etymon naming a language or family but no term, as an etymology.
fn language_only(etymon: &Etymon, mode: EtyMode, certainty: Certainty) -> Option<LanguageOnly> {
    let source = etymon.source.filter(|_| etymon.term.is_none())?;
    let certainty = if etymon.uncertain {
        certainty.max(Certainty::Possible)
    } else {
        certainty
    };
    Some(LanguageOnly {
        source,
        mode,
        certainty,
    })
}

fn single_template(
    string_pool: &mut StringPool,
    etymon: &Etymon,
    mode: EtyMode,
    certainty: Certainty,
) -> Option<RawEtyTemplate> {
    etymon.term.map(|term| RawEtyTemplate {
        langterms: Box::new([etymon.lang.new_langterm(string_pool, term)]),
        parts: Box::new([etymon.part()]),
        continues: false,
        affixes: Box::default(),
        refine_mode: false,
        mode,
        head: Some(0),
        certainty,
    })
}

// Terms joined by "and" after one keyword, as one template: several parents,
// none of them the head.
fn joined_template(
    string_pool: &mut StringPool,
    etymons: &[&Etymon],
    mode: EtyMode,
    certainty: Certainty,
) -> Option<RawEtyTemplate> {
    let mut langterms = Vec::with_capacity(etymons.len());
    for etymon in etymons {
        langterms.push(etymon.lang.new_langterm(string_pool, etymon.term?));
    }
    (!langterms.is_empty()).then(|| RawEtyTemplate {
        langterms: langterms.into_boxed_slice(),
        parts: etymons.iter().map(|e| e.part()).collect(),
        continues: false,
        refine_mode: false,
        affixes: Box::default(),
        mode,
        head: None,
        certainty,
    })
}

// A keyword group: its keyword, its modifiers and its etymons.
struct Group<'a> {
    keyword: Keyword,
    modifiers: Vec<&'a str>,
    etymons: Vec<Etymon<'a>>,
}

impl Group<'_> {
    // How Module:etymon/text joins this group's terms, and the group to the
    // next: "or" unless the keyword says otherwise (<conj:and>).
    fn joins_by_and(&self) -> bool {
        modifier(&self.modifiers, "conj").is_some_and(|conj| conj == "and")
    }

    // Whether the group starts a sentence of its own, adding to the
    // derivation rather than being an alternative to it: a calque-like
    // keyword (Module:etymon/data's new_sentence), or a surface analysis.
    fn own_sentence(&self) -> bool {
        matches!(
            self.keyword,
            Keyword::Single(EtyMode::Calque | EtyMode::PartialCalque | EtyMode::SemanticLoan)
        ) || flag(&self.modifiers, "surf")
    }

    fn certainty(&self) -> Certainty {
        if flag(&self.modifiers, "unc") {
            Certainty::Possible
        } else {
            Certainty::Stated
        }
    }
}

/// Parse a sequence of params into the etymologies they give, each a chain of
/// templates whose first gives the parents (empty if no keyword group is
/// usable).
///
/// Module:etymon/text joins the terms after a keyword, and the keyword groups
/// of the derivation's sentence, with "or" unless the keyword says `<conj:and>`:
/// "From French docteur or English doctor", "Possibly borrowed from German
/// Tiger, or a learned borrowing from Latin tigris". These are alternatives,
/// each its own etymology. Terms joined by "and" ("Borrowed from French de and
/// Latin dē") are one etymology with several parents. A calque-like keyword or
/// a surface analysis starts a sentence of its own and adds to the derivation,
/// so it is no alternative; it is read only when it comes first, as before.
/// An inline etymology continues a single parent's chain: with its one
/// etymology, or with its alternatives, each a chain of its own.
fn parse_params(
    string_pool: &mut StringPool,
    params: &[Param],
    lang: Lang,
    depth: usize,
) -> Vec<Vec<ParsedRawEtyTemplate>> {
    let mut raw_groups: Vec<(&str, Vec<&str>, Vec<&str>)> = vec![];
    for param in params {
        match param {
            Param::Keyword(name, modifiers) => raw_groups.push((name, modifiers.clone(), vec![])),
            Param::Etymon(etymon) => match raw_groups.last_mut() {
                Some((_, _, etymons)) => etymons.push(etymon),
                None => raw_groups.push(("from", vec![], vec![etymon])),
            },
        }
    }
    let mut groups = vec![];
    for (name, modifiers, etymons) in raw_groups {
        let Some(keyword) = keyword(name) else {
            continue; // an error on Wiktionary
        };
        if keyword == Keyword::Ignored || etymons.is_empty() {
            continue;
        }
        let etymons: Option<Vec<Etymon>> = etymons.iter().map(|e| parse_etymon(e, lang)).collect();
        let Some(etymons) = etymons else {
            if groups.is_empty() {
                return vec![vec![ParsedRawEtyTemplate::Skipped]];
            }
            break;
        };
        groups.push(Group {
            keyword,
            modifiers,
            etymons,
        });
    }
    let Some(first) = groups.first() else {
        return vec![];
    };
    // the first group, and the groups after it in its sentence joined by "or"
    let mut chosen = vec![first];
    if !first.own_sentence() {
        let mut joined_by_and = first.joins_by_and();
        for group in groups.iter().skip(1).filter(|g| !g.own_sentence()) {
            if joined_by_and {
                break;
            }
            chosen.push(group);
            joined_by_and = group.joins_by_and();
        }
    }
    let mut alternatives = vec![];
    for group in chosen {
        let certainty = group.certainty();
        // each alternative: a template, and the parent whose inline etymology
        // may continue it
        let mut found: Vec<(ParsedRawEtyTemplate, Option<&Etymon>)> = vec![];
        let parsed = |t: Option<RawEtyTemplate>| {
            t.map_or(ParsedRawEtyTemplate::Skipped, ParsedRawEtyTemplate::Parsed)
        };
        match group.keyword {
            Keyword::Single(mode) if group.joins_by_and() => {
                // an etymon naming only a language ("de:", "fr:-") gives no
                // parent; with none naming a term, the first is the origin
                let named: Vec<&Etymon> =
                    group.etymons.iter().filter(|e| e.term.is_some()).collect();
                found.push(match named[..] {
                    [] => (
                        group
                            .etymons
                            .iter()
                            .find_map(|e| language_only(e, mode, certainty))
                            .map_or(ParsedRawEtyTemplate::Skipped, |l| {
                                ParsedRawEtyTemplate::LanguageOnly(l, false)
                            }),
                        None,
                    ),
                    [single] => (
                        parsed(single_template(string_pool, single, mode, certainty)),
                        Some(single),
                    ),
                    _ => (
                        parsed(joined_template(string_pool, &named, mode, certainty)),
                        None,
                    ),
                });
            }
            Keyword::Single(mode) => {
                for etymon in &group.etymons {
                    let template = match language_only(etymon, mode, certainty) {
                        Some(l) => ParsedRawEtyTemplate::LanguageOnly(l, false),
                        None => parsed(single_template(string_pool, etymon, mode, certainty)),
                    };
                    found.push((template, Some(etymon)));
                }
            }
            Keyword::Components(mode) => {
                let template = components_template(string_pool, &group.etymons, mode, certainty);
                found.push((
                    parsed(template),
                    (group.etymons.len() == 1).then(|| &group.etymons[0]),
                ));
            }
            Keyword::Ignored => unreachable!(),
        }
        for (template, single) in found {
            let mut chain = vec![template];
            // A single parent's inline etymology continues the chain.
            if let Some(etymon) = single
                && let Some(ety) = etymon.ety
                && depth < MAX_INLINE_DEPTH
            {
                let inline = inline_params(ety);
                let mut inline_chains: Vec<Vec<ParsedRawEtyTemplate>> =
                    parse_params(string_pool, &inline, etymon.lang, depth + 1)
                        .into_iter()
                        .map(|chain| {
                            chain
                                .into_iter()
                                .map(|template| match template {
                                    ParsedRawEtyTemplate::Parsed(mut template) => {
                                        template.continues = true;
                                        ParsedRawEtyTemplate::Parsed(template)
                                    }
                                    ParsedRawEtyTemplate::LanguageOnly(l, _) => {
                                        ParsedRawEtyTemplate::LanguageOnly(l, true)
                                    }
                                    other => other,
                                })
                                .collect()
                        })
                        .collect();
                // alternatives are the parent's own; with no parent named, the
                // first goes on as the chain
                if inline_chains.len() > 1 && matches!(chain[0], ParsedRawEtyTemplate::Parsed(_)) {
                    chain.push(ParsedRawEtyTemplate::Alternatives(
                        inline_chains
                            .into_iter()
                            .map(Vec::into_boxed_slice)
                            .collect(),
                    ));
                } else if !inline_chains.is_empty() {
                    chain.extend(inline_chains.swap_remove(0));
                }
            }
            alternatives.push(chain);
        }
    }
    alternatives
}

/// The etymologies given by an {{etymon}} template's args: see parse_params.
pub(crate) fn parse_etymon_args(
    string_pool: &mut StringPool,
    args: &Json,
    lang: Lang,
) -> Vec<Vec<ParsedRawEtyTemplate>> {
    let mut params = vec![];
    let mut n = 2;
    while let Some(arg) = args.get_valid_str(n.to_string().as_str()) {
        params.push(top_level_param(arg));
        n += 1;
    }
    parse_params(string_pool, &params, lang, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::to_borrowed_value;

    fn lang(code: &str) -> Lang {
        Lang::from_str(code).unwrap()
    }

    // (mode, head, [(lang, term, id)]) for each template in the chain, or None
    // for a skipped one
    type Got = Vec<Option<(EtyMode, Option<u8>, Vec<(String, String, Option<String>)>)>>;

    // the first etymology's chain
    fn parse(args: &str, entry_lang: &str) -> Got {
        parse_all(args, entry_lang)
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    // every etymology's chain, and the certainty of each first template
    fn parse_all(args: &str, entry_lang: &str) -> Vec<Got> {
        parse_with_certainty(args, entry_lang)
            .into_iter()
            .map(|(got, _)| got)
            .collect()
    }

    fn parse_with_certainty(args: &str, entry_lang: &str) -> Vec<(Got, Vec<Certainty>)> {
        let mut bytes = args.as_bytes().to_vec();
        let args = to_borrowed_value(&mut bytes).unwrap();
        let mut string_pool = StringPool::new();
        parse_etymon_args(&mut string_pool, &args, lang(entry_lang))
            .into_iter()
            .map(|chain| {
                let certainties = match chain.first() {
                    Some(ParsedRawEtyTemplate::Parsed(t)) => t
                        .parts
                        .iter()
                        .map(|p| {
                            if p.uncertain {
                                t.certainty.max(Certainty::Possible)
                            } else {
                                t.certainty
                            }
                        })
                        .collect(),
                    _ => vec![],
                };
                (got(&string_pool, chain), certainties)
            })
            .collect()
    }

    fn got(string_pool: &StringPool, chain: Vec<ParsedRawEtyTemplate>) -> Got {
        chain
            .into_iter()
            .map(|t| match t {
                ParsedRawEtyTemplate::Parsed(t) => Some((
                    t.mode,
                    t.head,
                    t.langterms
                        .iter()
                        .zip(t.parts.iter())
                        .map(|(lt, part)| {
                            (
                                lt.lang.code().to_owned(),
                                lt.term.resolve(string_pool).to_owned(),
                                part.id.as_deref().map(str::to_owned),
                            )
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

    fn lt(lang: &str, term: &str, id: Option<&str>) -> (String, String, Option<String>) {
        (lang.to_owned(), term.to_owned(), id.map(str::to_owned))
    }

    #[test]
    fn latin_matrix_affix() {
        let got = parse(
            r#"{"1": "la", "title": "mātrīx", "2": ":af", "3": "māter",
                "4": "-ix<t:feminine noun ending>", "text": "+", "tree": "1"}"#,
            "la",
        );
        assert_eq!(
            got,
            [Some((
                EtyMode::Suffix,
                Some(0),
                vec![lt("la", "māter", None), lt("la", "-ix", None)]
            ))]
        );
    }

    #[test]
    fn english_mother_inherited_with_id() {
        let got = parse(
            r#"{"1": "en", "id": "female parent", "2": ":inh",
                "3": "enm:moder<id:mother>", "tree": "1"}"#,
            "en",
        );
        assert_eq!(
            got,
            [Some((
                EtyMode::Inherited,
                Some(0),
                vec![lt("enm", "moder", Some("mother"))]
            ))]
        );
    }

    #[test]
    fn default_keyword_is_from() {
        let got = parse(r#"{"1": "la", "2": "decem<id:ten>"}"#, "la");
        assert_eq!(
            got,
            [Some((
                EtyMode::MorphologicalDerivation,
                Some(0),
                vec![lt("la", "decem", Some("ten"))]
            ))]
        );
    }

    #[test]
    fn alternatives_are_etymologies_and_ignored_groups_are_skipped() {
        let single = |lang: &str, term: &str| {
            vec![Some((
                EtyMode::Borrowed,
                Some(0),
                vec![lt(lang, term, None)],
            ))]
        };
        // "Borrowed from Spanish and/or Portuguese cruzado"
        let got = parse_all(
            r#"{"1": "en", "2": ":root", "3": "ine-pro:*bʰer-<id:carry>",
                "4": ":bor<conj:and/or>", "5": "es:cruzado", "6": "pt:cruzado"}"#,
            "en",
        );
        assert_eq!(got, [single("es", "cruzado"), single("pt", "cruzado")]);
        // "From French docteur or English doctor": "or" by default
        let got = parse_all(
            r#"{"1": "mg", "2": ":bor", "3": "fr:docteur", "4": "en:doctor"}"#,
            "mg",
        );
        assert_eq!(got, [single("fr", "docteur"), single("en", "doctor")]);
    }

    #[test]
    fn terms_joined_by_and_are_one_etymology() {
        // "Borrowed from French de and Latin dē"
        let got = parse_all(
            r#"{"1": "eo", "2": ":bor<conj:and>", "3": "fr:de<id:of>", "4": "la:dē<id:of>"}"#,
            "eo",
        );
        assert_eq!(
            got,
            [vec![Some((
                EtyMode::Borrowed,
                None,
                vec![lt("fr", "de", Some("of")), lt("la", "dē", Some("of"))]
            ))]]
        );
        // "Borrowed from German inszenieren and French (mettre) en scène": an
        // etymon naming only a language gives no parent
        let got = parse_all(
            r#"{"1": "nl", "2": ":bor<conj:and>", "3": "de:inszenieren", "4": "fr:-"}"#,
            "nl",
        );
        assert_eq!(
            got,
            [vec![Some((
                EtyMode::Borrowed,
                Some(0),
                vec![lt("de", "inszenieren", None)]
            ))]]
        );
        let got = parse_all(
            r#"{"1": "ia", "2": ":bor<conj:and>", "3": "en:conquer", "4": "fr:conquérir", "5": "pt:"}"#,
            "ia",
        );
        assert_eq!(
            got,
            [vec![Some((
                EtyMode::Borrowed,
                None,
                vec![lt("en", "conquer", None), lt("fr", "conquérir", None)]
            ))]]
        );
    }

    #[test]
    fn groups_of_one_sentence_are_alternatives() {
        // "Possibly borrowed from German Tiger, or a learned borrowing from
        // Latin tigris"
        let got = parse_with_certainty(
            r#"{"1": "sk", "2": ":bor<unc>", "3": "de:Tiger", "4": ":lbor", "5": "la:tigris"}"#,
            "sk",
        );
        assert_eq!(
            got,
            [
                (
                    vec![Some((
                        EtyMode::Borrowed,
                        Some(0),
                        vec![lt("de", "Tiger", None)]
                    ))],
                    vec![Certainty::Possible]
                ),
                (
                    vec![Some((
                        EtyMode::LearnedBorrowing,
                        Some(0),
                        vec![lt("la", "tigris", None)]
                    ))],
                    vec![Certainty::Stated]
                ),
            ]
        );
        // "From cape + Portuguese verde. Partial calque of Portuguese Cabo
        // Verde": a calque starts a sentence of its own, adding to the
        // derivation
        let got = parse_all(
            r#"{"1": "en", "2": ":af", "3": "cape", "4": "pt:verde<t:green>",
                "5": ":pcal", "6": "pt:Cabo Verde"}"#,
            "en",
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0][0].as_ref().unwrap().0, EtyMode::Compound);
        // and so does a surface analysis
        let got = parse_all(
            r#"{"1": "en", "2": ":inh", "3": "enm:abetter", "4": ":af<surf>",
                "5": "abet", "6": "-er<id:agent noun>"}"#,
            "en",
        );
        assert_eq!(got.len(), 1);
        // groups joined by "and" add to each other: the first is read
        let got = parse_all(
            r#"{"1": "en", "2": ":bor<conj:and>", "3": "fr:a", "4": ":bor", "5": "de:b"}"#,
            "en",
        );
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn hedged_terms_are_possible() {
        let got = parse_with_certainty(
            r#"{"1": "xka", "id": "left", "2": ":inh", "3": "sa:खुर<t:hoof><unc>"}"#,
            "xka",
        );
        assert_eq!(got[0].1, [Certainty::Possible]);
        let got = parse_with_certainty(r#"{"1": "ar", "2": ":cal<unc>", "3": "fr:blanche"}"#, "ar");
        assert_eq!(got[0].1, [Certainty::Possible]);
        let got = parse_with_certainty(r#"{"1": "es", "2": ":inh", "3": "osp:onze"}"#, "es");
        assert_eq!(got[0].1, [Certainty::Stated]);
    }

    #[test]
    fn compounds_and_explicit_affix_types() {
        let got = parse(
            r#"{"1": "en", "2": ":af", "3": "tooth", "4": "brush"}"#,
            "en",
        );
        assert_eq!(
            got,
            [Some((
                EtyMode::Compound,
                None,
                vec![lt("en", "tooth", None), lt("en", "brush", None)]
            ))]
        );
        let got = parse(
            r#"{"1": "en", "2": ":af", "3": "un-<aftype:pre>", "4": "happy<aftype:naf>"}"#,
            "en",
        );
        assert_eq!(got[0].as_ref().unwrap().0, EtyMode::Prefix);
        assert_eq!(got[0].as_ref().unwrap().1, Some(1));
        // As with {{af}}, a root written with a final hyphen reads as a prefix
        // (Module:etymon instead looks up pos=root on the root's page).
        let got = parse(
            r#"{"1": "ine-pro", "id": "father", "2": ":af",
                "3": "*peh₂-<id:protect><unc>", "4": "*-tḗr<id:agent noun><unc>"}"#,
            "ine-pro",
        );
        assert_eq!(
            got,
            [Some((
                EtyMode::Confix,
                None,
                vec![
                    lt("ine-pro", "peh₂-", Some("protect")),
                    lt("ine-pro", "-tḗr", Some("agent noun"))
                ]
            ))]
        );
    }

    #[test]
    fn inline_etymology_continues_the_chain() {
        let got = parse(
            r#"{"1": "en", "2": ":der",
                "3": "xno:mot<ety:inh<fro:mot<ety:inh<la:muttum>>>>"}"#,
            "en",
        );
        assert_eq!(
            got,
            [
                Some((EtyMode::Derived, Some(0), vec![lt("xno", "mot", None)])),
                Some((EtyMode::Inherited, Some(0), vec![lt("fro", "mot", None)])),
                Some((EtyMode::Inherited, Some(0), vec![lt("la", "muttum", None)])),
            ]
        );
        // an unknown term is skipped, but its inline etymology is kept
        let got = parse(
            r#"{"1": "en", "2": ":inh", "3": "enm:<ety:bor<fro:octobre>>"}"#,
            "en",
        );
        assert_eq!(
            got,
            [
                None,
                Some((EtyMode::Borrowed, Some(0), vec![lt("fro", "octobre", None)])),
            ]
        );
        // inline affixation with keyword modifiers and references
        let got = parse(
            r#"{"1": "en", "2": ":der",
                "3": "enm:word<ref:{{R:MED}}<<name:MED>>><ety:af<<unc>><ang:word><-s>>"}"#,
            "en",
        );
        assert_eq!(
            got[1],
            Some((
                EtyMode::Suffix,
                Some(0),
                vec![lt("ang", "word", None), lt("enm", "-s", None)]
            ))
        );
    }

    #[test]
    fn unusable_etymons() {
        // a family code or unknown prefix
        assert_eq!(
            parse(r#"{"1": "en", "2": ":der", "3": "gem:foo"}"#, "en"),
            [None]
        );
        // only categorization
        assert!(parse(r#"{"1": "en", "2": ":root", "3": "ine-pro:*bʰer-"}"#, "en").is_empty());
        assert!(parse(r#"{"1": "en", "id": "example", "tree": "1"}"#, "en").is_empty());
    }

    #[test]
    fn modifier_splitting() {
        assert_eq!(
            crate::entries::split_modifiers("a<id:x><ety:inh<b<t:c>>>"),
            ("a", vec!["id:x", "ety:inh<b<t:c>>"])
        );
        assert_eq!(
            inline_params("inh<<unc>><fro:mot<id:x>>:bor<la:y>"),
            [
                Param::Keyword("inh", vec!["unc"]),
                Param::Etymon("fro:mot<id:x>"),
                Param::Keyword("bor", vec![]),
                Param::Etymon("la:y"),
            ]
        );
    }
}
