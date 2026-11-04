//! How a word's assertions become its reading: which etymology walks follow.
//!
//! The claims model reads a word by tiers, the first that settles it deciding:
//! (1) an override for the word, (2) one lineage, (3) consensus, (4) a clear
//! preference, (5) a model's synthesis, else no primary. With Wiktionary the
//! only source in the graph, every assertion has one origin, so every word is
//! settled by tier 2: the source's own preference among its alternatives is
//! primary, and with none stated there is none. No word has an override yet,
//! so tier 1 has nothing to apply.

use crate::ety_graph::Certainty;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, IntoStaticStr};

/// The version of these rules. A graph records the rules it was read with.
pub const SYNTHESIS_RULES: u32 = 2;

/// The tier that settled a word's reading.
#[derive(
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Tier {
    /// a reviewer's override for the word
    Override,
    /// every endorsing assertion has one origin
    #[default]
    OneLineage,
    /// every origin endorses one etymology
    Consensus,
    /// origins disagree, and only one endorses its etymology firmly
    Preference,
    /// origins disagree and none is preferred: the model tier's (syn-2), and
    /// until it runs, no primary
    Unsettled,
}

/// How a word's primary was chosen, and whether origins dispute it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reading {
    pub tier: Tier,
    /// Independent origins endorse competing etymologies at probable or
    /// better, or one rejects the primary.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disputed: bool,
}

/// What a word's assertions say for one of its etymologies.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Evidence {
    /// The origins (agents) whose assertions endorse it.
    pub origins: Vec<u32>,
    /// The origins that reject it.
    pub rejected_by: Vec<u32>,
    /// Its weakest link as each endorsement states it, firmest first.
    pub firmest: Certainty,
    /// Whether only wety's rules assert it.
    pub rule: bool,
}

/// A word's reading from its etymologies' evidence, by tiers 2 to 4: the
/// primary (None: walks follow every alternative) and how it was chosen.
/// Rules count only where no source asserts anything.
#[must_use]
pub fn read(etys: &[Evidence]) -> (Option<u8>, Reading) {
    let sourced = etys
        .iter()
        .positions(|e| !e.rule && !e.origins.is_empty())
        .collect_vec();
    let considered = if sourced.is_empty() {
        (0..etys.len()).collect_vec()
    } else {
        sourced
    };
    let origins = considered
        .iter()
        .flat_map(|&i| etys[i].origins.iter().copied())
        .unique()
        .collect_vec();
    let index = |i: usize| u8::try_from(i).ok();
    if origins.len() <= 1 {
        let weakest = considered.iter().map(|&i| etys[i].firmest).collect_vec();
        let primary = primary(&weakest).and_then(|p| index(considered[usize::from(p)]));
        return (primary, Reading::default());
    }
    let endorsed_by_all = considered
        .iter()
        .copied()
        .filter(|&i| {
            origins.iter().all(|o| etys[i].origins.contains(o)) && etys[i].rejected_by.is_empty()
        })
        .collect_vec();
    if let [only] = endorsed_by_all[..] {
        let reading = Reading {
            tier: Tier::Consensus,
            disputed: false,
        };
        return (index(only), reading);
    }
    let firm = |i: usize| etys[i].firmest <= Certainty::Probable && etys[i].rejected_by.is_empty();
    let firm_ones = considered
        .iter()
        .copied()
        .filter(|&i| firm(i))
        .collect_vec();
    if let [only] = firm_ones[..] {
        let reading = Reading {
            tier: Tier::Preference,
            disputed: true,
        };
        return (index(only), reading);
    }
    let reading = Reading {
        tier: Tier::Unsettled,
        disputed: firm_ones.len() > 1 || etys.iter().any(|e| !e.rejected_by.is_empty()),
    };
    (None, reading)
}

/// Tier 2, one lineage: of one source's alternatives with these weakest
/// links, the one it prefers. A source prefers the alternative it states more
/// firmly than every other ("from A, or possibly from B"); the order in which
/// it mentions them is no preference.
#[must_use]
pub fn primary(weakest: &[Certainty]) -> Option<u8> {
    let firmest = weakest.iter().min()?;
    match weakest.iter().positions(|w| w == firmest).collect_vec()[..] {
        [only] => u8::try_from(only).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(origins: &[u32], firmest: Certainty) -> Evidence {
        Evidence {
            origins: origins.to_vec(),
            firmest,
            ..Evidence::default()
        }
    }

    #[test]
    fn readings_by_tier() {
        use Certainty::{Possible, Stated};
        // Wiktionary alone, two alternatives, one firmer: one lineage
        let (p, r) = read(&[ev(&[0], Stated), ev(&[0], Possible)]);
        assert_eq!((p, r.tier), (Some(0), Tier::OneLineage));
        // Wiktionary following de Vaan (one origin) and de Vaan
        let (p, r) = read(&[ev(&[1], Stated)]);
        assert_eq!((p, r.tier), (Some(0), Tier::OneLineage));
        // two origins on one etymology, Wiktionary also offering another
        let (p, r) = read(&[ev(&[0, 1], Stated), ev(&[0], Stated)]);
        assert_eq!((p, r.tier, r.disputed), (Some(0), Tier::Consensus, false));
        // disagreement, one firm: preferred, disputed
        let (p, r) = read(&[ev(&[0], Possible), ev(&[1], Stated)]);
        assert_eq!((p, r.tier, r.disputed), (Some(1), Tier::Preference, true));
        // disagreement, both firm: unsettled
        let (p, r) = read(&[ev(&[0], Stated), ev(&[1], Stated)]);
        assert_eq!((p, r.tier, r.disputed), (None, Tier::Unsettled, true));
        // a rule's etymology counts only where no source asserts anything
        let rule = Evidence {
            origins: vec![9],
            rule: true,
            ..Evidence::default()
        };
        let (p, r) = read(&[rule, ev(&[1], Possible)]);
        assert_eq!((p, r.tier), (Some(1), Tier::OneLineage));
    }

    #[test]
    fn the_firmest_alternative_alone_is_primary() {
        use Certainty::{Possible, Stated};
        assert_eq!(primary(&[Stated, Possible]), Some(0));
        assert_eq!(primary(&[Possible, Stated]), Some(1));
        assert_eq!(primary(&[Stated, Stated]), None);
        assert_eq!(primary(&[]), None);
    }
}
