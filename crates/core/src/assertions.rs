//! Who asserts each etymology in the graph, and on what grounds.
//!
//! An etymology's asserter follows from its edges' basis: Wiktionary's editors,
//! or the wety rule that reads the basis (`rule`). What the basis does not say is
//! kept here: the page whose section states the etymology, the works it credits,
//! and what a rule read. Most etymologies are stated on their own page with no
//! credit; only the others are stored. `graph-assertions` writes the assertions
//! out in the assertion schema (packages/etymology/schema).

use crate::{ety_graph::EdgeBasis, items::ItemId};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<ItemId>,
    #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
    pub credits: Box<[Credit]>,
    /// How many inputs a rule read: the derivatives voting for a root.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub inputs: u32,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(n: &u32) -> bool {
    *n == 0
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

/// A cited work, as the source renders its citation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Work {
    /// Its id in the assertion schema: a bibliography's (`work:wiktionary:R:...`),
    /// or the digest of a footnote's citation.
    pub id: Box<str>,
    pub citation: Box<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
        EdgeBasis::Etymon | EdgeBasis::Template | EdgeBasis::Descendants => return None,
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
