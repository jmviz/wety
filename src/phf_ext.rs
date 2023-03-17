use anyhow::{anyhow, Result};
use phf::{OrderedMap, OrderedSet};

pub(crate) trait OrderedMapExt {
    fn get_index_key(&self, index: usize) -> Option<&str>;
    fn get_expected_index_key(&self, index: usize) -> Result<&str>;
    fn get_index_value(&self, index: usize) -> Option<&str>;
    fn get_expected_index_value(&self, index: usize) -> Result<&str>;
    fn get_expected_index(&self, key: &str) -> Result<usize>;
}

impl OrderedMapExt for OrderedMap<&str, &str> {
    fn get_index_key(&self, index: usize) -> Option<&str> {
        self.index(index).map(|(&key, _)| key)
    }
    fn get_expected_index_key(&self, index: usize) -> Result<&str> {
        self.get_index_key(index)
            .ok_or_else(|| anyhow!("The index {index} does not exist."))
    }
    fn get_index_value(&self, index: usize) -> Option<&str> {
        self.index(index).map(|(_, &value)| value)
    }
    fn get_expected_index_value(&self, index: usize) -> Result<&str> {
        self.get_index_value(index)
            .ok_or_else(|| anyhow!("The index {index} does not exist."))
    }
    fn get_expected_index(&self, key: &str) -> Result<usize> {
        self.get_index(key)
            .ok_or_else(|| anyhow!("The key '{key}' does not exist."))
    }
}

pub(crate) trait OrderedSetExt {
    fn get_expected_index_key(&self, index: usize) -> Result<&str>;
    fn get_expected_index(&self, key: &str) -> Result<usize>;
}

impl OrderedSetExt for OrderedSet<&str> {
    fn get_expected_index_key(&self, index: usize) -> Result<&str> {
        self.index(index)
            .copied()
            .ok_or_else(|| anyhow!("The index {index} does not exist."))
    }
    fn get_expected_index(&self, key: &str) -> Result<usize> {
        self.get_index(key)
            .ok_or_else(|| anyhow!("The key '{key}' does not exist."))
    }
}
