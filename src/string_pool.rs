use serde::{Deserialize, Serialize};
use string_interner::{
    backend::StringBackend, symbol::SymbolU32, StringInterner, Symbol as SymbolTrait,
};

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) struct Symbol(SymbolU32);

impl SymbolTrait for Symbol {
    fn try_from_usize(index: usize) -> Option<Self> {
        let symbol_u32 = SymbolU32::try_from_usize(index)?;
        Some(Self { 0: symbol_u32 })
    }
    fn to_usize(self) -> usize {
        self.0.to_usize()
    }
}

impl Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_usize().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Symbol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = usize::deserialize(deserializer)?;
        Ok(Self::try_from_usize(s)
            .expect("this was a SymbolU32 converted to a usize for serialize"))
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct StringPool {
    pool: StringInterner<StringBackend<Symbol>>,
}

impl StringPool {
    pub(crate) fn new() -> Self {
        let pool: StringInterner<StringBackend<Symbol>> = StringInterner::new();
        Self { pool }
    }

    pub(crate) fn resolve(&self, symbol: Symbol) -> &str {
        self.pool
            .resolve(symbol)
            .expect("Resolve interned string from symbol")
    }

    pub(crate) fn get_or_intern(&mut self, s: &str) -> Symbol {
        self.pool.get_or_intern(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_serialize() {
        let s = Symbol {
            0: SymbolU32::try_from_usize(1337).unwrap(),
        };
        assert_eq!("1337", serde_json::to_string(&s).unwrap());
    }

    #[test]
    fn symbol_deserialize() {
        let s: Symbol = serde_json::from_str("1337").unwrap();
        assert_eq!(1337, s.to_usize());
    }
}
