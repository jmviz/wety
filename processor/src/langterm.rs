use std::str::FromStr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::{
    lang_phf::{LANG_CODE2NAME, LANG_ETYCODE2CODE, LANG_NAME2CODE, LANG_RECONSTRUCTED},
    phf_ext::OrderedMapExt,
    string_pool::{StringPool, Symbol},
};

// See data/phf/lang.py for more on these, but in summary:
// LANG_CODE2NAME: A bijection from all lang codes to their canonical names,
// e.g. "en" -> "English".
// LANG_NAME2CODE: Not merely the inverse of CODE2NAME. Many languages have
// multiple names, each of which maps to the same lang code (i.e. the map is
// surjective but not injective).
// LANG_ETYCODE2CODE: Maps every etymology-only lang code to the lang code of
// its nearest "main" parent (i.e. lang code for which a wiktionary page can
// exist), e.g. "VL." -> "la"
// LANG_RECONSTRUCTED: A set of all reconstructed lang codes.

// LangId refers to an index in the LANG_CODE2NAME OrderedMap
pub type LangId = u16; // The map has ~10k elements

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Lang {
    id: LangId,
}

impl From<LangId> for Lang {
    fn from(lang_id: LangId) -> Self {
        Self { id: lang_id }
    }
}

impl FromStr for Lang {
    type Err = anyhow::Error;

    fn from_str(lang_code: &str) -> Result<Self, Self::Err> {
        if let Some(id) = LANG_CODE2NAME.get_index(lang_code) {
            return Ok(LangId::try_from(id)?.into());
        }
        Err(anyhow!(
            "The key \"{lang_code}\" does not exist LANG_CODE2NAME"
        ))
    }
}

impl Lang {
    // pub(crate) fn id(self) -> LangId {
    //     self.id
    // }

    pub(crate) fn code(self) -> &'static str {
        LANG_CODE2NAME
            .get_index_key(self.id as usize)
            .expect("id cannot have been created without being a valid index")
    }

    pub(crate) fn name(self) -> &'static str {
        LANG_CODE2NAME
            .get_index_value(self.id as usize)
            .expect("id cannot have been created without being a valid index")
    }

    // If lang is an etymology-only language, we will not find any entries
    // for it in Items lang map, since such a language definitionally does
    // not have any entries itself. So we look for the main lang that the
    // ety lang is associated with.
    pub(crate) fn ety2main(self) -> Self {
        LANG_ETYCODE2CODE
            .get(self.code())
            .and_then(|code| LANG_CODE2NAME.get_index(code))
            .map_or(self.id, |i| {
                LangId::try_from(i).expect("less than LangId::MAX elements in LANG_CODE2NAME")
            })
            .into()
    }

    pub(crate) fn is_reconstructed(self) -> bool {
        LANG_RECONSTRUCTED.contains(self.code())
    }

    pub(crate) fn new_langterm(self, string_pool: &mut StringPool, term: &str) -> LangTerm {
        let term = Term::new(string_pool, term);
        LangTerm::new(self, term)
    }
}

// LanguageId refers to an index in the LANG_NAME2CODE OrderedMap.
pub(crate) type LanguageId = u16; // map has ~15k elements

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) struct Language {
    id: LanguageId,
}

impl Language {
    fn code(self) -> &'static str {
        LANG_NAME2CODE
            .get_index_value(self.id as usize)
            .expect("id cannot have been created without being a valid index")
    }
}

impl From<LanguageId> for Language {
    fn from(language_id: LanguageId) -> Self {
        Self { id: language_id }
    }
}

impl From<Lang> for Language {
    fn from(lang: Lang) -> Self {
        LANG_NAME2CODE
            .get_index(lang.name())
            .map(|i| {
                LanguageId::try_from(i)
                    .expect("less than LanguageId::MAX elements in LANG_NAME2CODE")
            })
            .expect("all name values in LANG_CODE2NAME should be keys in LANG_NAME2CODE")
            .into()
    }
}

// For converting from a language name, as in a reconstruction redirect (e.g.
// "Proto-Indo-European")
impl FromStr for Language {
    type Err = anyhow::Error;

    fn from_str(language_name: &str) -> Result<Self, Self::Err> {
        if let Some(id) = LANG_NAME2CODE.get_index(language_name) {
            return Ok(LanguageId::try_from(id)?.into());
        }
        Err(anyhow!(
            "The key \"{language_name}\" does not exist LANG_NAME2CODE"
        ))
    }
}

impl From<Language> for Lang {
    fn from(language: Language) -> Self {
        let id = LANG_CODE2NAME
            .get_index(language.code())
            .map(|i| LangId::try_from(i).expect("less than LangId::MAX elements in LANG_CODE2NAME"))
            .expect("all code values in LANG_NAME2CODE should be keys in LANG_CODE2NAME");
        Self { id }
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct Term {
    symbol: Symbol,
}

impl From<Symbol> for Term {
    fn from(symbol: Symbol) -> Self {
        Self { symbol }
    }
}

impl<'a> Term {
    pub(crate) fn new(string_pool: &mut StringPool, term: &str) -> Self {
        let symbol = string_pool.get_or_intern(term);
        Self { symbol }
    }

    pub(crate) fn resolve(self, string_pool: &'a StringPool) -> &'a str {
        string_pool.resolve(self.symbol)
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) struct LangTerm {
    pub(crate) lang: Lang,
    pub(crate) term: Term,
}

impl LangTerm {
    pub(crate) fn new(lang: Lang, term: Term) -> Self {
        Self { lang, term }
    }
}

// Used in redirects
#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) struct LanguageTerm {
    pub(crate) language: Language,
    pub(crate) term: Term,
}

impl From<LangTerm> for LanguageTerm {
    fn from(langterm: LangTerm) -> Self {
        Self {
            language: langterm.lang.into(),
            term: langterm.term,
        }
    }
}

impl From<LanguageTerm> for LangTerm {
    fn from(language_term: LanguageTerm) -> Self {
        Self {
            lang: language_term.language.into(),
            term: language_term.term,
        }
    }
}
