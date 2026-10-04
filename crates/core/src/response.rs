//! The shapes the server sends to the client; field names match the
//! interfaces in the client's `search/types.ts`.

use crate::{
    gloss::Gloss,
    items::ItemId,
    languages::{Lang, LangInfo},
    string_pool::StringPool,
};

use std::{borrow::Cow, fmt};

use serde::{Serialize, Serializer};

/// A gloss written straight into the output without joining its interned
/// words into a `String` first.
pub struct GlossStr<'a> {
    pub gloss: &'a Gloss,
    pub string_pool: &'a StringPool,
}

impl fmt::Display for GlossStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, word) in self.gloss.words(self.string_pool).enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            f.write_str(word)?;
        }
        Ok(())
    }
}

impl Serialize for GlossStr<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemJson<'a> {
    pub id: ItemId,
    pub ety_num: u8,
    /// What tells this item apart from others of the same lang and term in a
    /// link (see `keys`); absent when nothing needs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ety: Option<Cow<'a, str>>,
    pub lang: LangInfo,
    pub term: &'a str,
    pub imputed: bool,
    pub reconstructed: bool,
    pub url: Option<String>,
    pub pos: Option<Vec<&'static str>>,
    /// Outer `None` omits the field (lean trees; the client fetches the item
    /// lazily); `Some(None)` is an item without glosses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gloss: Option<Option<Vec<GlossStr<'a>>>>,
    pub romanization: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OtherParentJson<'a> {
    pub item: ItemJson<'a>,
    pub ety_order: u8,
    pub lang_distance: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DescendantsJson<'a> {
    pub item: ItemJson<'a>,
    pub children: Vec<DescendantsJson<'a>>,
    /// Nodes strictly below this one in the returned (filtered) tree, so the
    /// client can size collapsed subtrees without walking them.
    pub descendant_count: u64,
    pub lang_distance: Option<usize>,
    pub ety_mode: Option<&'static str>,
    pub other_parents: Vec<OtherParentJson<'a>>,
    pub parent_ety_order: Option<u8>,
}

/// One descendants tree per progenitor of the requested item.
pub type CognatesJson<'a> = Vec<DescendantsJson<'a>>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtymologyJson<'a> {
    pub item: ItemJson<'a>,
    pub ety_mode: Option<&'static str>,
    pub ety_order: u8,
    pub parents: Vec<EtymologyJson<'a>>,
    pub lang_distance: Option<usize>,
}

impl Lang {
    #[must_use]
    pub fn info(self) -> LangInfo {
        LangInfo {
            id: self.id(),
            code: self.code(),
            name: self.name(),
        }
    }
}
