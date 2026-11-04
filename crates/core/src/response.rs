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

/// One step of a word's trail: the item and the mode of its own etymology.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrailStepJson<'a> {
    pub item: ItemJson<'a>,
    pub ety_mode: Option<&'static str>,
}

/// The descendants of one ancestor of the requested item (`root`), with the
/// item's trail (itself back to its oldest form that is the same word) and
/// the sources of that oldest form, which the reader may start from instead.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognatesJson<'a> {
    pub trail: Vec<TrailStepJson<'a>>,
    pub sources: Vec<ItemJson<'a>>,
    /// The item's ultimate ancestors not already on the trail or among its
    /// sources, in etymology order: root entries, then the rest (affixes,
    /// words with no recorded source).
    pub ultimate_roots: Vec<ItemJson<'a>>,
    pub ultimate_others: Vec<ItemJson<'a>>,
    pub root: ItemId,
    /// Where the tree starts without a `root` request: the trail's last step,
    /// or the root entry it is formed from.
    pub default_root: ItemId,
    /// Words in the root's unfiltered tree (up to `TOTAL_CAP`).
    pub total: usize,
    /// Whether `tree` covers every language rather than the requested ones.
    pub all_langs: bool,
    pub tree: DescendantsJson<'a>,
}

/// A descendants tree with the size of its unfiltered version, as for
/// cognates.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DescendantsViewJson<'a> {
    pub total: usize,
    pub all_langs: bool,
    pub tree: DescendantsJson<'a>,
}

/// A word in an etymology chart, with every etymology it has.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtymologyJson<'a> {
    pub item: ItemJson<'a>,
    /// Its place among its child's parents.
    pub ety_order: u8,
    pub lang_distance: Option<usize>,
    /// How firmly its child is said to come from it, when not stated outright.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certainty: Option<&'static str>,
    /// One etymology, or alternatives one source gives ("from A or B");
    /// none when nothing is recorded.
    pub etymologies: Vec<EtyJson<'a>>,
    /// The etymology walks follow; null when it has several and none is
    /// preferred, and walks follow each.
    pub primary: Option<u8>,
    /// How the primary was chosen, when not by one lineage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading: Option<ReadingJson>,
}

/// How a word's primary was chosen: the tier (`consensus`, `preference`,
/// `unsettled`, `override`) and whether independent origins dispute it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingJson {
    pub tier: &'static str,
    pub disputed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtyJson<'a> {
    /// "derivation", or "unknown": the source says no origin is known.
    pub kind: &'static str,
    pub mode: Option<&'static str>,
    /// Whether a wety rule reads it rather than a source stating it.
    pub inferred: bool,
    /// Whether only another source than Wiktionary gives it.
    pub source: bool,
    pub parents: Vec<EtymologyJson<'a>>,
}

/// A word's etymologies with who asserts each and on what grounds, as served
/// at `/origin/:id`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginJson<'a> {
    pub primary: Option<u8>,
    pub reading: ReadingJson,
    pub hypotheses: Vec<HypothesisJson<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisJson<'a> {
    pub kind: &'static str,
    pub mode: Option<&'static str>,
    pub parents: Vec<LinkJson<'a>>,
    /// Wiktionary's assertion of it (its editors', or a wety rule's reading
    /// of Wiktionary); none when only another source gives it.
    pub asserted: Option<AssertionJson<'a>>,
    /// Other sources' claims about it.
    pub claims: Vec<ClaimJson<'a>>,
}

/// Another source's claim about one of a word's etymologies.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimJson<'a> {
    /// The asserter's id (`agent:de-vaan`) and name.
    pub by: &'a str,
    pub by_name: &'a str,
    /// endorses, reports or rejects
    pub stance: &'static str,
    /// same, refines or competes: how it stands to the etymology
    pub relation: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certainty: Option<&'static str>,
    /// The claim's own mode, when it is not the etymology's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<&'static str>,
    /// Whether a wety rule compiled it from the source's structure.
    pub compiled: bool,
    pub locator: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkJson<'a> {
    pub item: ItemJson<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certainty: Option<&'static str>,
    /// The part the word is built on, when its parents are parts.
    pub head: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertionJson<'a> {
    /// "wiktionary-editors", or the wety rule that reads it (`wety-form-of@1`).
    pub by: &'static str,
    /// Who that is, for readers.
    pub by_name: &'static str,
    /// The source a relay found it follows, by name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<&'a str>,
    /// For a rule: what it inferred the etymology from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    /// The page that states it: the word's own, or another's (its Descendants
    /// list, the entry citing a missing word).
    pub page: ItemJson<'a>,
    pub section: &'static str,
    pub credits: Vec<CreditJson<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditJson<'a> {
    pub citation: &'a str,
    pub year: Option<u16>,
    pub locator: Option<&'a str>,
    /// follows, reports, against or data
    pub role: &'static str,
    /// inline, note (a footnote) or entry (the entry's References)
    pub scope: &'static str,
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
