use string_interner::{backend::StringBackend, symbol::SymbolU32, StringInterner};

pub(crate) type Symbol = SymbolU32;

#[derive(Default)]
pub(crate) struct StringPool {
    pool: StringInterner<StringBackend<Symbol>>,
}

impl StringPool {
    pub(crate) fn resolve(&self, symbol: Symbol) -> &str {
        self.pool
            .resolve(symbol)
            .expect("Resolve interned string from symbol")
    }
    pub(crate) fn get_or_intern(&mut self, s: &str) -> Symbol {
        self.pool.get_or_intern(s)
    }
}
