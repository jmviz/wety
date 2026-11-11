#![allow(clippy::redundant_closure_for_method_calls)]

pub mod assertions;
pub mod descendant_langs;
pub mod ety_graph;
pub mod etymology_templates;
pub mod gloss;
pub mod items;
pub mod keys;
pub mod langterm;
pub mod languages;
pub mod pos;
mod pos_phf;
pub mod processed;
pub mod response;
pub mod string_pool;
pub mod synthesis;

pub use items::ItemId;
pub use languages::Lang;
pub use processed::{Data, Search, TreeView};

use std::hash::{BuildHasher, Hasher};

use xxhash_rust::xxh3::{Xxh3, xxh3_64};

pub type HashMap<K, V> = std::collections::HashMap<K, V, Xxh3Keys>;
pub type HashSet<T> = std::collections::HashSet<T, Xxh3Keys>;

/// The xxh3 hash (seed 0) of the bytes a key writes, as `xxh3::Xxh3Builder` gives it,
/// without building its ~540-byte streaming state for every key: a short key is
/// gathered inline and hashed in one shot. The values are the same, and so is the
/// iteration order of every map, on which some outputs depend.
#[derive(Clone, Copy, Default)]
pub struct Xxh3Keys;

impl BuildHasher for Xxh3Keys {
    type Hasher = Xxh3KeyHasher;

    fn build_hasher(&self) -> Xxh3KeyHasher {
        Xxh3KeyHasher::Short {
            buf: [0; SHORT_KEY],
            len: 0,
        }
    }
}

const SHORT_KEY: usize = 64;

pub enum Xxh3KeyHasher {
    Short { buf: [u8; SHORT_KEY], len: usize },
    Long(Box<Xxh3>),
}

impl Hasher for Xxh3KeyHasher {
    fn write(&mut self, bytes: &[u8]) {
        match self {
            Self::Short { buf, len } if *len + bytes.len() <= SHORT_KEY => {
                buf[*len..*len + bytes.len()].copy_from_slice(bytes);
                *len += bytes.len();
            }
            Self::Short { buf, len } => {
                let mut long = Box::new(Xxh3::new());
                long.update(&buf[..*len]);
                long.update(bytes);
                *self = Self::Long(long);
            }
            Self::Long(long) => long.update(bytes),
        }
    }

    fn finish(&self) -> u64 {
        match self {
            Self::Short { buf, len } => xxh3_64(&buf[..*len]),
            Self::Long(long) => long.digest(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::hash::BuildHasher;

    #[test]
    fn hashes_as_streaming_xxh3() {
        let streaming = xxhash_rust::xxh3::Xxh3Builder::new();
        let long = "x".repeat(300);
        let keys: Vec<(u32, u16, &str)> = vec![
            (0, 0, ""),
            (7, 3, "minþiją"),
            (
                u32::MAX,
                9,
                "a term of sixty bytes or so, to cross the inline limit",
            ),
            (1, 1, &long),
        ];
        for key in &keys {
            assert_eq!(Xxh3Keys.hash_one(key), streaming.hash_one(key));
        }
        for n in 0u64..1000 {
            assert_eq!(Xxh3Keys.hash_one(n), streaming.hash_one(n));
            assert_eq!(
                Xxh3Keys.hash_one((n, n as u32)),
                streaming.hash_one((n, n as u32))
            );
        }
    }
}
