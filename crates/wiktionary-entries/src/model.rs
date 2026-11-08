//! The entries file: one JSON line per [`Entry`].
//!
//! Text offsets (`start`, `end`) count Unicode scalar values (Python `str` indices) in
//! the `text` of the block they belong to.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

/// One language section of a page, or one numbered etymology within it.
#[derive(Serialize, Default, Debug)]
pub struct Entry {
    pub page: String,
    pub page_id: u64,
    pub rev: u64,
    pub ns: i64,
    /// Wiktionary's code for the heading's language; `None` if the name is unknown.
    pub lang: Option<&'static str>,
    pub lang_heading: String,
    /// "2" for "Etymology 2"; `None` for an unnumbered or absent etymology heading.
    pub ety_label: Option<String>,
    pub etymology: Option<Section>,
    pub pos: Vec<PosBlock>,
    /// Descendants given beside the parts of speech, for the whole etymology.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub descendants: Vec<Descendant>,
    /// The References section of this etymology, or of the language when unnumbered.
    pub references: Option<Section>,
    /// A References section beside numbered etymologies: it covers every one of them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_references: Option<Section>,
    pub further_reading: Option<Section>,
    pub categories: Vec<String>,
}

#[derive(Serialize, Default, Debug)]
pub struct PosBlock {
    pub heading: String,
    /// The heading's POS code (wiktextract's table, `pos_headings.json`).
    pub pos: Option<&'static str>,
    /// Headwords as rendered (`.headword`).
    pub headword: Vec<String>,
    /// Template calls on the headword line.
    pub head: Vec<Call>,
    pub senses: Vec<Sense>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub descendants: Vec<Descendant>,
    /// Template calls of the Derived terms section.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub derived: Vec<Call>,
}

#[derive(Serialize, Debug)]
pub struct Sense {
    pub depth: u8,
    pub gloss: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub senseids: Vec<String>,
    pub calls: Vec<Call>,
}

/// One template call, as Parsoid recorded it in `data-mw`.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Call {
    pub name: String,
    pub args: BTreeMap<String, String>,
}

#[derive(Serialize, Debug, Default, Clone)]
pub struct Section {
    /// Digest of the section's content: unchanged sections keep it across dumps.
    pub digest: String,
    pub blocks: Vec<Block>,
    /// Trees `{{ety}}` and `{{etymon}}` render (`data-ety-tree-json`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trees: Vec<Value>,
    /// Bodies of the footnotes the blocks cite.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

/// A paragraph, a list item or another run of text, with its templates and footnotes.
#[derive(Serialize, Debug, Clone)]
pub struct Block {
    pub kind: &'static str,
    pub depth: u8,
    pub text: String,
    pub spans: Vec<Span>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Span {
    /// Text one transclusion rendered; it may hold several calls.
    Template {
        start: usize,
        end: usize,
        calls: Vec<Call>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        links: Vec<Link>,
        /// Output of a transclusion an earlier span already opened (its calls are
        /// counted there).
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        continued: bool,
    },
    /// A footnote marker; `note` names a [`Note`].
    Ref { start: usize, note: String },
}

/// A wiki link's target page: the entry name Lua computed for a term.
#[derive(Serialize, Debug, Clone)]
pub struct Link {
    pub page: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    /// The page does not exist (a red link).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub calls: Vec<Call>,
}

#[derive(Serialize, Debug)]
pub struct Descendant {
    pub text: String,
    pub calls: Vec<Call>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Descendant>,
}

/// Counts of what the extraction met, written beside the entries.
#[derive(Default, Debug)]
pub struct Counts(pub BTreeMap<String, u64>);

impl Counts {
    pub fn add(&mut self, key: &str, n: u64) {
        *self.0.entry(key.to_string()).or_default() += n;
    }

    pub fn merge(&mut self, other: Counts) {
        for (k, v) in other.0 {
            *self.0.entry(k).or_default() += v;
        }
    }
}
