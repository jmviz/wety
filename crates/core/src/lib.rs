#![allow(clippy::redundant_closure_for_method_calls)]

pub mod ety_graph;
pub mod etymology_templates;
pub mod gloss;
pub mod items;
pub mod langterm;
pub mod languages;
pub mod pos;
mod pos_phf;
pub mod processed;
pub mod string_pool;

pub use items::ItemId;
pub use languages::Lang;
pub use processed::{Data, Search};

use xxhash_rust::xxh3::Xxh3Builder;

pub type HashMap<K, V> = std::collections::HashMap<K, V, Xxh3Builder>;
pub type HashSet<T> = std::collections::HashSet<T, Xxh3Builder>;
