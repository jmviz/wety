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

/// The version of these rules. A graph records the rules it was read with.
pub const SYNTHESIS_RULES: u32 = 1;

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

    #[test]
    fn the_firmest_alternative_alone_is_primary() {
        use Certainty::{Possible, Stated};
        assert_eq!(primary(&[Stated, Possible]), Some(0));
        assert_eq!(primary(&[Possible, Stated]), Some(1));
        assert_eq!(primary(&[Stated, Stated]), None);
        assert_eq!(primary(&[]), None);
    }
}
