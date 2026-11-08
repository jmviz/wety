//! Wiktionary's dumps to wety's entries: one record per page × language × etymology,
//! with each etymology's prose, templates and footnotes in place.

pub mod dump;
pub mod model;
pub mod page;
pub mod records;
pub mod render;
