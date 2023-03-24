use itertools::Itertools;

use crate::string_pool::{StringPool, Symbol};

#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) struct Gloss {
    symbols: Box<[Symbol]>,
}

impl Gloss {
    pub(crate) fn new(string_pool: &mut StringPool, gloss: &str) -> Self {
        let symbols: Box<[Symbol]> = gloss
            .split(' ')
            .map(|g| string_pool.get_or_intern(g))
            .collect();
        Self { symbols }
    }

    pub(crate) fn to_string(&self, string_pool: &StringPool) -> String {
        self.symbols
            .iter()
            .map(|&symbol| string_pool.resolve(symbol))
            .join(" ")
    }
}
