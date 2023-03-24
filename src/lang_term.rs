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
pub(crate) type LangId = usize;

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct Lang {
    id: LangId,
}

impl From<LangId> for Lang {
    fn from(lang_id: LangId) -> Self {
        Self { id: lang_id }
    }
}

// For converting from a lang code string, as in an ety template (e.g. "en")
impl TryFrom<&str> for Lang {
    type Error = anyhow::Error;

    fn try_from(lang_code: &str) -> Result<Self, Self::Error> {
        if let Some(id) = LANG_CODE2NAME.get_index(lang_code) {
            return Ok(id.into());
        }
        Err(anyhow!(
            "The key \"{lang_code}\" does not exist LANG_CODE2NAME"
        ))
    }
}

impl Lang {
    pub(crate) fn id(&self) -> LangId {
        self.id
    }

    pub(crate) fn code(&self) -> &'static str {
        LANG_CODE2NAME
            .get_index_key(self.id)
            .expect("id cannot have been created without being a valid index")
    }

    pub(crate) fn name(&self) -> &'static str {
        LANG_CODE2NAME
            .get_index_value(self.id)
            .expect("id cannot have been created without being a valid index")
    }

    // If lang is an etymology-only language, we will not find any entries
    // for it in Items lang map, since such a language definitionally does
    // not have any entries itself. So we look for the main lang that the
    // ety lang is associated with.
    pub(crate) fn ety2main(&self) -> Self {
        let id = LANG_ETYCODE2CODE
            .get(self.code())
            .and_then(|code| LANG_CODE2NAME.get_index(code))
            .unwrap_or(self.id);
        Self { id }
    }

    pub(crate) fn is_reconstructed(&self) -> bool {
        LANG_RECONSTRUCTED.contains(self.code())
    }
}

// LanguageId refers to an index in the LANG_NAME2CODE OrderedMap.
pub(crate) type LanguageId = usize;

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct Language {
    pub(crate) id: LanguageId,
}

impl Language {
    fn code(&self) -> &'static str {
        LANG_NAME2CODE
            .get_index_value(self.id)
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
        let id = LANG_NAME2CODE
            .get_index(lang.name())
            .expect("all name values in LANG_CODE2NAME should be keys in LANG_NAME2CODE");
        Self { id }
    }
}

// For converting from a language name, as in a reconstruction redirect (e.g.
// "Proto-Indo-European")
impl TryFrom<&str> for Language {
    type Error = anyhow::Error;

    fn try_from(language_name: &str) -> Result<Self, Self::Error> {
        if let Some(id) = LANG_NAME2CODE.get_index(language_name) {
            return Ok(id.into());
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

    pub(crate) fn resolve(&self, string_pool: &'a StringPool) -> &'a str {
        string_pool.resolve(self.symbol)
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
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
#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct LanguageTerm {
    pub(crate) language: Language,
    pub(crate) term: Term,
}

impl LanguageTerm {
    pub(crate) fn new(language: Language, term: Term) -> Self {
        Self { language, term }
    }
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
