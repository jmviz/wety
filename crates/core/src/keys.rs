//! Names for items that outlive a build, for shared links: language code,
//! term and, where several items share these, Wiktionary's etymology number
//! (`ety`). Numeric item ids are positions in one build's graph.
//!
//! A name is resolved against whatever graph is loaded. It never resolves to a
//! different word; when Wiktionary renumbers a page's etymologies it can land
//! on another homograph of the same word, so the client offers the others.

use crate::{
    items::{Item, ItemId},
    languages::Lang,
    processed::Data,
};

use std::borrow::Cow;

use string_interner::Symbol as _;
use unicode_normalization::UnicodeNormalization;

/// (lang id, term symbol, item index), sorted for exact lookup.
type Index = Vec<(u16, u32, u32)>;

pub struct Keys {
    terms: Index,
    page_terms: Index,
}

pub enum Resolution {
    /// The item named, and every item sharing its name (itself included).
    Found(ItemId, Vec<ItemId>),
    /// Several items share the name and nothing says which one is meant.
    Ambiguous(Vec<ItemId>),
    NotFound,
}

impl Item {
    /// The `ety` that tells this item apart in a link: Wiktionary's etymology
    /// number where the page has one, else our own number when it is not 1.
    #[must_use]
    pub fn link_ety(&self) -> Option<Cow<'_, str>> {
        self.ety_label()
            .map(Cow::Borrowed)
            .or_else(|| (self.ety_num() > 1).then(|| Cow::Owned(self.ety_num().to_string())))
    }
}

fn symbol(term: crate::langterm::Term) -> u32 {
    u32::try_from(term.symbol().to_usize()).expect("symbols are u32")
}

fn lookup(index: &Index, lang: Lang, term: u32) -> impl Iterator<Item = ItemId> + '_ {
    let start = index.partition_point(|&(l, t, _)| (l, t) < (lang.id(), term));
    index[start..]
        .iter()
        .take_while(move |&&(l, t, _)| (l, t) == (lang.id(), term))
        .map(|&(_, _, i)| ItemId::new(i as usize))
}

impl Data {
    #[must_use]
    pub fn build_keys(&self) -> Keys {
        let mut terms = Vec::with_capacity(self.graph.len());
        let mut page_terms = vec![];
        for (id, item) in self.graph.iter() {
            let lang = item.lang().id();
            let index = u32::try_from(id.index()).expect("item indices are u32");
            terms.push((lang, symbol(item.term()), index));
            if let Some(page_term) = item.page_term() {
                page_terms.push((lang, symbol(page_term), index));
            }
        }
        terms.sort_unstable();
        page_terms.sort_unstable();
        Keys { terms, page_terms }
    }

    /// Find the item a link names. `term` is matched exactly (after Unicode
    /// normalization), first against displayed terms, then page titles (e.g.
    /// "voco" for "vocō"); a leading `*` is ignored. Real items outrank
    /// imputed ones of the same name. Among several, `ety` picks one; without
    /// it, the one Wiktionary does not number, if any.
    #[must_use]
    pub fn resolve(&self, keys: &Keys, lang: Lang, term: &str, ety: Option<&str>) -> Resolution {
        let term: String = term.nfc().collect();
        let term = term.strip_prefix('*').unwrap_or(&term);
        let Some(term) = self.string_pool.get(term) else {
            return Resolution::NotFound;
        };
        let term = u32::try_from(term.to_usize()).expect("symbols are u32");
        let mut items: Vec<ItemId> = lookup(&keys.terms, lang, term).collect();
        if items.is_empty() {
            items = lookup(&keys.page_terms, lang, term).collect();
        }
        if items.iter().any(|&id| !self.graph.item(id).is_imputed()) {
            items.retain(|&id| !self.graph.item(id).is_imputed());
        }
        items.sort_by_key(|&id| (self.graph.item(id).ety_num(), id.index()));
        let chosen = match (items.as_slice(), ety) {
            ([], _) => return Resolution::NotFound,
            ([only], _) => Some(*only),
            (_, Some(ety)) => items
                .iter()
                .find(|&&id| self.graph.item(id).ety_label() == Some(ety))
                .or_else(|| {
                    items.iter().find(|&&id| {
                        let item = self.graph.item(id);
                        item.ety_label().is_none() && item.ety_num().to_string() == ety
                    })
                })
                .copied(),
            (_, None) => items
                .iter()
                .find(|&&id| {
                    let item = self.graph.item(id);
                    item.ety_label().is_none() && item.ety_num() == 1
                })
                .copied(),
        };
        match chosen {
            Some(id) => Resolution::Found(id, items),
            None => Resolution::Ambiguous(items),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ety_graph::EtyGraph,
        items::{ImputedItem, RealItem},
        langterm::Term,
        string_pool::StringPool,
    };
    use std::str::FromStr;

    fn real(sp: &mut StringPool, lang: &str, term: &str, page: Option<&str>, label: Option<&str>, ety_num: u8) -> Item {
        Item::Real(RealItem {
            ety_num,
            ety_label: label.map(Into::into),
            lang: Lang::from_str(lang).unwrap(),
            term: Term::new(sp, term),
            pos: vec![],
            gloss: vec![],
            page_term: page.map(|p| Term::new(sp, p)),
            romanization: None,
            is_reconstructed: false,
        })
    }

    fn data() -> (Data, Vec<ItemId>) {
        let mut sp = StringPool::new();
        let mut graph = EtyGraph::default();
        let mut ids = vec![];
        for n in 1..=3u8 {
            let label = n.to_string();
            ids.push(graph.add(real(&mut sp, "en", "lime", None, Some(&label), n)));
        }
        ids.push(graph.add(real(&mut sp, "en", "kid", None, None, 1))); // 3
        ids.push(graph.add(real(&mut sp, "la", "levis", None, Some("1"), 1))); // 4
        ids.push(graph.add(real(&mut sp, "la", "lēvis", Some("levis"), Some("2"), 2))); // 5
        ids.push(graph.add(real(&mut sp, "ine-pro", "men-", None, None, 1))); // 6
        ids.push(graph.add(real(&mut sp, "ine-pro", "men-", None, None, 2))); // 7
        for n in 1..=2u8 {
            ids.push(graph.add(Item::Imputed(ImputedItem {
                ety_num: n,
                lang: Lang::from_str("gem-pro").unwrap(),
                term: Term::new(&mut sp, "kidą"),
                romanization: None,
                from: ids[3],
            }))); // 8, 9
        }
        (Data::new(sp, graph), ids)
    }

    fn found(r: Resolution) -> Option<ItemId> {
        match r {
            Resolution::Found(id, _) => Some(id),
            _ => None,
        }
    }

    #[test]
    fn resolves_names() {
        let (data, ids) = data();
        let keys = data.build_keys();
        let lang = |c| Lang::from_str(c).unwrap();
        let r = |l, t, e| found(data.resolve(&keys, lang(l), t, e));
        assert_eq!(r("en", "lime", Some("2")), Some(ids[1]));
        // several etymologies and no number: ask
        assert!(matches!(data.resolve(&keys, lang("en"), "lime", None), Resolution::Ambiguous(v) if v.len() == 3));
        // a number the page no longer has: ask
        assert_eq!(r("en", "lime", Some("9")), None);
        // only one item: the number does not matter
        assert_eq!(r("en", "kid", Some("2")), Some(ids[3]));
        // displayed form, then page title
        assert_eq!(r("la", "lēvis", None), Some(ids[5]));
        // an exact displayed form wins over page titles
        assert_eq!(r("la", "levis", Some("2")), Some(ids[4]));
        // unnumbered sections: ours
        assert_eq!(r("ine-pro", "*men-", None), Some(ids[6]));
        assert_eq!(r("ine-pro", "men-", Some("2")), Some(ids[7]));
        // imputed items, by their own number
        assert_eq!(r("gem-pro", "kidą", Some("2")), Some(ids[9]));
        // Unicode normalization: decomposed ē
        assert_eq!(r("la", "le\u{304}vis", None), Some(ids[5]));
        assert!(matches!(data.resolve(&keys, lang("en"), "nonesuch", None), Resolution::NotFound));
        assert!(matches!(data.resolve(&keys, lang("la"), "lime", None), Resolution::NotFound));
    }

    #[test]
    fn link_etys() {
        let (data, ids) = data();
        let ety = |i: usize| data.graph.item(ids[i]).link_ety().map(|e| e.into_owned());
        assert_eq!(ety(1).as_deref(), Some("2"));
        assert_eq!(ety(3), None);
        assert_eq!(ety(7).as_deref(), Some("2"));
        assert_eq!(ety(8), None);
        assert_eq!(ety(9).as_deref(), Some("2"));
    }
}
