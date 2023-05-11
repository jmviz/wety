use serde::{Deserialize, Serialize};

use crate::{
    languages::Lang,
    string_pool::{StringPool, Symbol},
};

impl Lang {
    pub(crate) fn new_langterm(self, string_pool: &mut StringPool, term: &str) -> LangTerm {
        let term = Term::new(string_pool, term);
        LangTerm::new(self, term)
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
