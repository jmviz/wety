//! Who asserts each etymology in the graph, and on what grounds.
//!
//! An etymology's asserter follows from its edges' basis: Wiktionary's editors,
//! or the wety rule that reads the basis (`rule`). What the basis does not say is
//! kept here: the page whose section states the etymology, the works it credits,
//! and what a rule read. Most etymologies are stated on their own page with no
//! credit; only the others are stored. `graph-assertions` writes the assertions
//! out in the assertion schema (packages/etymology/schema).

use crate::{
    ety_graph::{Certainty, EdgeBasis},
    etymology_templates::EtyMode,
    items::{Item, ItemId},
    string_pool::StringPool,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use strum::{AsRefStr, IntoStaticStr};

/// What one etymology's assertion adds to its edges.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Asserted {
    /// The page whose section states the etymology, when it is not the item's
    /// own: another page's Descendants list, or the section that cites an
    /// imputed item and gives its etymology.
    #[serde(default)]
    pub page: Option<ItemId>,
    #[serde(default)]
    pub credits: Box<[Credit]>,
    /// How many inputs a rule read: the derivatives voting for a root.
    #[serde(default)]
    pub inputs: u32,
    /// The agent (index into the graph's agents) whose own assertion of the
    /// same etymology this one relays, when a relay found that they agree:
    /// the assertion's origin. None: it is its own origin.
    #[serde(default)]
    pub origin: Option<u32>,
}

impl Asserted {
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// A work an assertion relies on, with the role and scope of the citation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Credit {
    /// Index into the graph's works.
    pub work: u32,
    pub role: Role,
    pub scope: Scope,
    /// Where in the work: a page, an entry.
    #[serde(default)]
    pub locator: Option<Box<str>>,
}

#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, AsRefStr, IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Role {
    /// the asserter adopts the work's view, or cites it after a claim
    Follows,
    /// the asserter relays the work's view without taking a side
    Reports,
    /// the asserter cites the work to reject it
    Against,
    /// the work is cited for a form, a gloss or a date
    Data,
}

#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, AsRefStr, IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Scope {
    /// named in the claim
    Inline,
    /// a footnote on the claim's section
    Note,
    /// listed in the entry's References
    Entry,
}

/// Who asserts what another source says about an item's etymology: a source
/// other than Wiktionary (its editors, a scholar) or a wety rule reading that
/// source's structure.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Agent {
    /// Its id in the assertion schema (`agent:de-vaan`).
    pub id: Box<str>,
    pub name: Box<str>,
    /// "person", "editors", "rule", ...
    pub kind: Box<str>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, AsRefStr, IntoStaticStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Stance {
    Endorses,
    Reports,
    Rejects,
}

/// How a claim stands to the etymology it is attached to.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, AsRefStr, IntoStaticStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Relation {
    /// the same parents
    Same,
    /// a parent that is the other's ancestor or descendant: compatible, the
    /// fuller kept, both credited
    Refines,
    /// other parents: the claim is an etymology of its own
    Competes,
}

/// Another source's claim about one of an item's etymologies: one link of a
/// hypothesis it asserts, its records matched to items.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Claim {
    /// The etymology it is about.
    pub ety: u8,
    /// Index into the graph's agents: the asserter.
    pub agent: u32,
    /// Index into the graph's agents: where the claim originates. The
    /// asserter, unless a relay finds it follows another's assertion.
    pub origin: u32,
    pub stance: Stance,
    pub relation: Relation,
    /// The claim's own certainty of the link (its hedges).
    pub certainty: Certainty,
    /// The claim's mode, when it is not the etymology's.
    #[serde(default)]
    pub mode: Option<EtyMode>,
    /// Whether a wety rule compiled it from the source's structure.
    #[serde(default)]
    pub compiled: bool,
    /// Its assertion's id in the assertion schema.
    pub assertion: Box<str>,
    /// The source it is from, by name (`acd`, `lila-edl`, `wold`): its
    /// assertion's source without the release.
    pub source: Box<str>,
    /// Where in the source.
    pub locator: Box<str>,
}

/// The id `graph-assertions` gives an item's record:
/// `wiktionary:<code>:<term>#<n>`, with `?` for an item imputed from citations.
#[must_use]
pub fn record_id(item: &Item, string_pool: &StringPool) -> String {
    format!(
        "wiktionary:{}:{}#{}{}",
        item.lang().code(),
        item.term().resolve(string_pool),
        item.ety_num(),
        if item.is_imputed() { "?" } else { "" }
    )
}

/// A cited work, as the source renders its citation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Work {
    /// Its id in the assertion schema: a bibliography's (`work:wiktionary:R:...`),
    /// or the digest of a footnote's citation.
    pub id: Box<str>,
    pub citation: Box<str>,
    #[serde(default)]
    pub year: Option<u16>,
}

/// An id as `etymology.assertions.digest` computes it: the sha256 of the value
/// as compact JSON with sorted keys (serde_json's maps are sorted), first 16 hex
/// digits.
#[must_use]
pub fn digest(value: &Value) -> String {
    let hash = Sha256::digest(value.to_string().as_bytes());
    hash.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// A wety rule that asserts etymologies, at its version. Bumping a version
/// marks every assertion it made as made by another rule.
#[must_use]
pub fn rule(basis: EdgeBasis) -> Option<&'static str> {
    Some(match basis {
        EdgeBasis::Chain => "wety-chain@1",
        EdgeBasis::Mention => "wety-mention@1",
        EdgeBasis::FormOf => "wety-form-of@1",
        EdgeBasis::Root => "wety-root@1",
        EdgeBasis::RootVote => "wety-root-vote@1",
        EdgeBasis::Etymon | EdgeBasis::Template | EdgeBasis::Descendants | EdgeBasis::Source => {
            return None;
        }
    })
}

/// What a rule reads, for readers.
#[must_use]
pub fn rule_name(rule: &str) -> &'static str {
    match rule.split('@').next() {
        Some("wety-chain") => {
            "wety's chain rule: a later template read as the etymology of a missing parent before it"
        }
        Some("wety-mention") => {
            "wety's mention rule: a lone {{m}}, in a section with no other etymology template"
        }
        Some("wety-form-of") => "wety's form-of rule: an entry's form-of or alt-of sense",
        Some("wety-root") => {
            "wety's root rule: an entry's {{root}} or root category, for an entry with no other etymology"
        }
        _ => {
            "wety's root vote: the root most of a progenitor's derivatives name, for a progenitor with no etymology"
        }
    }
}

/// How a rule's reading is described: what it inferred the etymology from.
#[must_use]
pub fn compiled_words(inputs: u32) -> String {
    match inputs {
        0 => "inferred by wety".to_owned(),
        1 => "inferred by wety from 1 derivative's root categories".to_owned(),
        n => format!("inferred by wety from {n} derivatives' root categories"),
    }
}

/// The part of a page that states an etymology of this basis.
#[must_use]
pub fn section(basis: EdgeBasis) -> &'static str {
    match basis {
        EdgeBasis::Etymon | EdgeBasis::Template | EdgeBasis::Chain | EdgeBasis::Mention => {
            "Etymology"
        }
        EdgeBasis::Descendants => "Descendants",
        EdgeBasis::Source => "the source's entry",
        EdgeBasis::FormOf => "senses",
        EdgeBasis::Root | EdgeBasis::RootVote => "categories",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn digests_are_those_of_the_python_package() {
        // etymology.assertions.digest(["wiktionary", "x", "ü"])
        assert_eq!(digest(&json!(["wiktionary", "x", "ü"])), "1f3d79a50b5e6bbb");
    }
}
