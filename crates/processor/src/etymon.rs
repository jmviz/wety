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
//! its page is missing).
//!
//! An entry's {{etymon}} becomes a chain of `ParsedRawEtyTemplate`s, as the
//! classic templates do: the first is the entry's parent(s), and the rest come
//! from the inline etymology of a single parent, to be used if that parent has
//! to be imputed. Of several keyword groups we take the first structural one,
//! just as we take the first classic template.

use crate::{
    etymology::{ParsedRawEtyTemplate, RawEtyTemplate, affixation_kind},
    wiktextract_json::{
        Affix, WiktextractJson, WiktextractJsonValidStr, affix_by_hyphens, affix_type,
        clean_template_term, modifier, split_lang_prefix, split_modifiers,
    },
};
use wety_core::{etymology_templates::EtyMode, languages::Lang, string_pool::StringPool};

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
        "abbreviation" | "abbr" | "abbrev" | "syllabic abbreviation" | "sylabbr"
        | "sylabbrev" => Single(M::Abbreviation),
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

// A positional arg: a keyword (without its colon and modifiers) or an etymon.
#[derive(Debug, PartialEq)]
enum Param<'a> {
    Keyword(&'a str),
    Etymon(&'a str),
}

fn top_level_param(arg: &str) -> Param<'_> {
    match arg.strip_prefix(':') {
        Some(keyword) => Param::Keyword(split_modifiers(keyword).0.trim()),
        None => Param::Etymon(arg),
    }
}

/// The params of an inline etymology `keyword<<kwmod>><etymon><etymon>:keyword2<etymon>`.
fn inline_params(s: &str) -> Vec<Param<'_>> {
    let (keyword, _) = split_modifiers(s);
    let mut params = vec![Param::Keyword(keyword.trim().trim_start_matches(':'))];
    let mut rest = &s[keyword.len()..];
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<<") {
            // a keyword modifier, e.g. <<unc>>
            rest = after.split_once(">>").map_or("", |(_, r)| r);
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
                params.push(Param::Keyword(keyword.trim()));
            }
            rest = &rest[bare_end..];
        }
    }
    params
}

#[derive(Debug, PartialEq)]
pub(crate) struct Etymon<'a> {
    pub(crate) lang: Lang,
    // None for an unknown ("", "+") or suppressed ("-") term
    pub(crate) term: Option<&'a str>,
    pub(crate) id: Option<&'a str>,
    aftype: Option<&'a str>,
    ety: Option<&'a str>,
}

/// Parse `lang:term<mod:value>...`. Returns None if the prefix is not a
/// language code we know (e.g. a family code).
pub(crate) fn parse_etymon(param: &str, context_lang: Lang) -> Option<Etymon<'_>> {
    let (head, modifiers) = split_modifiers(param);
    let (lang, term) = match split_lang_prefix(head) {
        Some((code, term)) => (Lang::from_str(code).ok()?, term),
        None => (context_lang, head),
    };
    let term = clean_template_term(term.trim());
    let term = (!matches!(term, "" | "+" | "-")).then_some(term);
    Some(Etymon {
        lang,
        term,
        id: modifier(&modifiers, "id")
            .map(|id| id.trim_start_matches('!'))
            .filter(|id| !id.is_empty()),
        aftype: modifier(&modifiers, "aftype"),
        ety: modifier(&modifiers, "ety").filter(|ety| !ety.is_empty()),
    })
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
) -> Option<RawEtyTemplate> {
    let mut langterms = Vec::with_capacity(etymons.len());
    let mut ids = Vec::with_capacity(etymons.len());
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
        ids.push(etymon.id.map(Box::from));
    }
    (!langterms.is_empty()).then(|| RawEtyTemplate {
        langterms: langterms.into_boxed_slice(),
        ids: ids.into_boxed_slice(),
        continues: false,
        mode: mode.unwrap_or_else(|| affixation_kind(&affixes, n_base_terms)),
        // as for {{af}}, a head only when there is a single base term
        head: (n_base_terms == 1).then(|| u8::try_from(head).ok()).flatten(),
    })
}

/// Parse a sequence of params into an etymology chain (empty if no keyword
/// group is usable).
fn parse_params(
    string_pool: &mut StringPool,
    params: &[Param],
    lang: Lang,
    depth: usize,
) -> Vec<ParsedRawEtyTemplate> {
    let mut groups: Vec<(&str, Vec<&str>)> = vec![];
    for param in params {
        match *param {
            Param::Keyword(name) => groups.push((name, vec![])),
            Param::Etymon(etymon) => match groups.last_mut() {
                Some((_, etymons)) => etymons.push(etymon),
                None => groups.push(("from", vec![etymon])),
            },
        }
    }
    for (name, etymons) in groups {
        let Some(keyword) = keyword(name) else {
            continue; // an error on Wiktionary
        };
        if keyword == Keyword::Ignored || etymons.is_empty() {
            continue;
        }
        let etymons: Option<Vec<Etymon>> =
            etymons.iter().map(|e| parse_etymon(e, lang)).collect();
        let Some(etymons) = etymons else {
            return vec![ParsedRawEtyTemplate::Skipped];
        };
        let parsed = match keyword {
            Keyword::Single(mode) => etymons[0].term.map(|term| RawEtyTemplate {
                langterms: Box::new([etymons[0].lang.new_langterm(string_pool, term)]),
                ids: Box::new([etymons[0].id.map(Box::from)]),
                continues: false,
                mode,
                head: Some(0),
            }),
            Keyword::Components(mode) => components_template(string_pool, &etymons, mode),
            Keyword::Ignored => unreachable!(),
        };
        let mut chain = vec![parsed.map_or(ParsedRawEtyTemplate::Skipped, ParsedRawEtyTemplate::Parsed)];
        // A single parent's inline etymology continues the chain.
        let single = match keyword {
            Keyword::Single(_) => Some(&etymons[0]),
            _ => (etymons.len() == 1).then(|| &etymons[0]),
        };
        if let Some(etymon) = single
            && let Some(ety) = etymon.ety
            && depth < MAX_INLINE_DEPTH
        {
            let inline = inline_params(ety);
            chain.extend(
                parse_params(string_pool, &inline, etymon.lang, depth + 1)
                    .into_iter()
                    .map(|template| match template {
                        ParsedRawEtyTemplate::Parsed(mut template) => {
                            template.continues = true;
                            ParsedRawEtyTemplate::Parsed(template)
                        }
                        skipped @ ParsedRawEtyTemplate::Skipped => skipped,
                    }),
            );
        }
        return chain;
    }
    vec![]
}

/// The etymology chain given by an {{etymon}} template's args.
pub(crate) fn parse_etymon_args(
    string_pool: &mut StringPool,
    args: &WiktextractJson,
    lang: Lang,
) -> Vec<ParsedRawEtyTemplate> {
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

    fn parse(args: &str, entry_lang: &str) -> Got {
        let mut bytes = args.as_bytes().to_vec();
        let args = to_borrowed_value(&mut bytes).unwrap();
        let mut string_pool = StringPool::new();
        parse_etymon_args(&mut string_pool, &args, lang(entry_lang))
            .into_iter()
            .map(|t| match t {
                ParsedRawEtyTemplate::Parsed(t) => Some((
                    t.mode,
                    t.head,
                    t.langterms
                        .iter()
                        .zip(t.ids.iter())
                        .map(|(lt, id)| {
                            (
                                lt.lang.code().to_owned(),
                                lt.term.resolve(&string_pool).to_owned(),
                                id.as_deref().map(str::to_owned),
                            )
                        })
                        .collect(),
                )),
                ParsedRawEtyTemplate::Skipped => None,
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
    fn alternatives_take_the_first_and_ignored_groups_are_skipped() {
        let got = parse(
            r#"{"1": "en", "2": ":root", "3": "ine-pro:*bʰer-<id:carry>",
                "4": ":bor<conj:and/or>", "5": "es:cruzado", "6": "pt:cruzado"}"#,
            "en",
        );
        assert_eq!(
            got,
            [Some((EtyMode::Borrowed, Some(0), vec![lt("es", "cruzado", None)]))]
        );
    }

    #[test]
    fn compounds_and_explicit_affix_types() {
        let got = parse(r#"{"1": "en", "2": ":af", "3": "tooth", "4": "brush"}"#, "en");
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
        assert_eq!(parse(r#"{"1": "en", "2": ":der", "3": "gem:foo"}"#, "en"), [None]);
        // only categorization
        assert!(parse(r#"{"1": "en", "2": ":root", "3": "ine-pro:*bʰer-"}"#, "en").is_empty());
        assert!(parse(r#"{"1": "en", "id": "example", "tree": "1"}"#, "en").is_empty());
    }

    #[test]
    fn modifier_splitting() {
        assert_eq!(
            crate::wiktextract_json::split_modifiers("a<id:x><ety:inh<b<t:c>>>"),
            ("a", vec!["id:x", "ety:inh<b<t:c>>"])
        );
        assert_eq!(
            inline_params("inh<<unc>><fro:mot<id:x>>:bor<la:y>"),
            [
                Param::Keyword("inh"),
                Param::Etymon("fro:mot<id:x>"),
                Param::Keyword("bor"),
                Param::Etymon("la:y"),
            ]
        );
    }
}
