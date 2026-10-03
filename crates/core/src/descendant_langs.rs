use crate::{items::ItemId, languages::Lang};

use std::fmt;

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, Unexpected, Visitor},
};

/// For each item, the langs that have at least one item descended from it.
/// CSR layout: item `i`'s langs are `langs[offsets[i]..offsets[i + 1]]`,
/// sorted so membership is a binary search.
///
/// Deserializes from either this layout or the older map from item id to
/// langs, so graphs built before the CSR layout still load.
#[derive(Default, Serialize)]
pub struct DescendantLangs {
    offsets: Vec<u32>,
    langs: Vec<Lang>,
}

impl DescendantLangs {
    /// # Panics
    ///
    /// Panics if an item id appears more than once in `pairs`.
    #[must_use]
    pub fn new(mut pairs: Vec<(ItemId, Vec<Lang>)>) -> Self {
        pairs.sort_unstable_by_key(|(id, _)| *id);
        let n = pairs.last().map_or(0, |(id, _)| id.index() + 1);
        let mut offsets = Vec::with_capacity(n + 1);
        let mut langs = Vec::with_capacity(pairs.iter().map(|(_, l)| l.len()).sum());
        let mut next = 0;
        for (id, mut item_langs) in pairs {
            while offsets.len() < id.index() {
                offsets.push(next);
            }
            assert_eq!(offsets.len(), id.index(), "duplicate item id {id:?}");
            offsets.push(next);
            item_langs.sort_unstable_by_key(|l| l.id());
            item_langs.dedup();
            langs.extend_from_slice(&item_langs);
            next = u32::try_from(langs.len()).expect("fewer than 2^32 (item, lang) pairs");
        }
        offsets.push(next);
        Self { offsets, langs }
    }

    #[must_use]
    pub fn get(&self, item: ItemId) -> &[Lang] {
        let i = item.index();
        if i + 1 >= self.offsets.len() {
            return &[];
        }
        &self.langs[self.offsets[i] as usize..self.offsets[i + 1] as usize]
    }

    #[must_use]
    pub fn contains(&self, item: ItemId, lang: Lang) -> bool {
        self.get(item)
            .binary_search_by_key(&lang.id(), |l| l.id())
            .is_ok()
    }
}

impl<'de> Deserialize<'de> for DescendantLangs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(DescendantLangsVisitor)
    }
}

struct DescendantLangsVisitor;

impl<'de> Visitor<'de> for DescendantLangsVisitor {
    type Value = DescendantLangs;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("descendant langs as {offsets, langs} or a map from item id to langs")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut offsets = None;
        let mut langs = None;
        let mut pairs = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "offsets" => offsets = Some(map.next_value()?),
                "langs" => langs = Some(map.next_value()?),
                id => {
                    let id: usize = id
                        .parse()
                        .map_err(|_| de::Error::invalid_value(Unexpected::Str(id), &self))?;
                    pairs.push((ItemId::new(id), map.next_value::<Vec<Lang>>()?));
                }
            }
        }
        match (offsets, langs) {
            (Some(offsets), Some(langs)) if pairs.is_empty() => Ok(DescendantLangs { offsets, langs }),
            (None, None) => Ok(DescendantLangs::new(pairs)),
            _ => Err(de::Error::custom("descendant langs mix the CSR and map formats")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup() {
        let dl = DescendantLangs::new(vec![
            (
                ItemId::new(2),
                vec![Lang::from(5), Lang::from(3), Lang::from(5)],
            ),
            (ItemId::new(0), vec![Lang::from(7)]),
            (ItemId::new(1), vec![]),
        ]);
        assert_eq!(dl.get(ItemId::new(0)), &[Lang::from(7)]);
        assert_eq!(dl.get(ItemId::new(1)), &[]);
        assert_eq!(dl.get(ItemId::new(2)), &[Lang::from(3), Lang::from(5)]);
        assert_eq!(dl.get(ItemId::new(3)), &[]);
        assert_eq!(dl.get(ItemId::new(1000)), &[]);
        assert!(dl.contains(ItemId::new(2), Lang::from(5)));
        assert!(!dl.contains(ItemId::new(2), Lang::from(4)));
        assert!(!dl.contains(ItemId::new(1), Lang::from(5)));
    }

    #[test]
    fn deserializes_both_formats() {
        let legacy: DescendantLangs =
            serde_json::from_str(r#"{"2": [5, 3], "0": [7], "1": []}"#).unwrap();
        assert_eq!(legacy.get(ItemId::new(0)), &[Lang::from(7)]);
        assert_eq!(legacy.get(ItemId::new(2)), &[Lang::from(3), Lang::from(5)]);

        let csr: DescendantLangs =
            serde_json::from_str(&serde_json::to_string(&legacy).unwrap()).unwrap();
        assert_eq!(csr.get(ItemId::new(2)), &[Lang::from(3), Lang::from(5)]);
        assert!(serde_json::from_str::<DescendantLangs>(r#"{"offsets": [0], "1": []}"#).is_err());
    }
}
