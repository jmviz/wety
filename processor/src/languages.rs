use crate::HashMap;

use std::{collections::BTreeMap, str::FromStr};

use anyhow::{anyhow, Ok, Result};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
enum LangKind {
    #[default]
    Regular,
    Reconstructed,
    EtymologyOnly,
    AppendixConstructed,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLangData {
    // aliases: Vec<&'static str>,
    ancestors: Vec<&'static str>,
    canonical_name: &'static str,
    // family: Option<&'static str>,
    kind: LangKind,
    // For regular languages, the mainCode should be the same as the code. For
    // etymology-only languages, it may not be the same. For example, Vulgar
    // Latin codes "VL" and "VL." both have mainCode "la-vul".
    main_code: &'static str,
    non_etymology_only: &'static str,
    // other_names: Vec<&'static str>,
    // parents: Vec<&'static str>,
    // scripts: Vec<&'static str>,
    // varieties: Vec<&'static str>,
    // wikidata_item: &'static str,
    // wikipedia_article: &'static str,
}

type LangId = u16;

#[derive(Default, Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Lang(LangId);

impl From<LangId> for Lang {
    fn from(id: LangId) -> Self {
        Self(id)
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct LangData {
    code: &'static str,
    name: &'static str,
    url_name: String,
    kind: LangKind,
    non_ety: Lang,
    ancestors: Vec<Lang>,
}

struct Languages {
    data: Vec<LangData>,
    code2id: HashMap<&'static str, Lang>,
    name2id: HashMap<&'static str, Lang>,
}

impl Languages {
    fn new() -> Self {
        // It's important to use an ordered map here. If a HashMap were used,
        // the iteration order and thus the sequence of ids would be
        // non-deterministic. This could cause discrepancies in LangId between
        // runs, particularly when running on different machines (e.g. local dev
        // vs. prod machines).
        let code2raw_data: BTreeMap<&'static str, RawLangData> = serde_json::from_str(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/languages.json")),
        )
        .expect("well-formed languages.json");

        let mut main_code2id = HashMap::default();
        let mut next_id: LangId = 0;

        for raw_data in code2raw_data.values() {
            if !main_code2id.contains_key(raw_data.main_code) {
                main_code2id.insert(raw_data.main_code, next_id);
                next_id += 1;
            }
        }

        let mut data = vec![LangData::default(); next_id.into()];
        let mut code2id = HashMap::default();
        let mut name2id = HashMap::default();

        for (&code, raw_data) in &code2raw_data {
            let id = *main_code2id.get(raw_data.main_code).expect("added above");

            let lang = Lang(id);
            code2id.insert(code, lang);
            name2id.insert(raw_data.canonical_name, lang);

            if data[id as usize] != LangData::default() {
                continue;
            }

            let mut ancestors = raw_data
                .ancestors
                .iter()
                .map(|&code| {
                    main_code2id
                        .get(code)
                        .map(|&id| Lang(id))
                        .expect("ancestor code should be a main code")
                })
                .collect::<Vec<_>>();
            ancestors.push(lang);

            let lang_data = LangData {
                code: raw_data.main_code,
                name: raw_data.canonical_name,
                url_name: urlencoding::encode(&raw_data.canonical_name.replace(' ', "_"))
                    .to_string(),
                kind: raw_data.kind,
                non_ety: main_code2id
                    .get(raw_data.non_etymology_only)
                    .map(|&id| Lang(id))
                    .expect("non etymology code should be a main code"),
                ancestors,
            };

            data[id as usize] = lang_data;
        }

        Self {
            data,
            code2id,
            name2id,
        }
    }

    fn data(&self, lang: Lang) -> &LangData {
        &self.data[lang.id() as usize]
    }

    fn code2lang(&self, code: &str) -> Option<Lang> {
        self.code2id.get(code).copied()
    }

    fn name2lang(&self, name: &str) -> Option<Lang> {
        self.name2id.get(name).copied()
    }
}

lazy_static! {
    static ref LANGUAGES: Languages = Languages::new();
}

impl FromStr for Lang {
    type Err = anyhow::Error;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        if let Some(lang) = LANGUAGES.code2lang(code) {
            return Ok(lang);
        }
        Err(anyhow!("Unknown lang code \"{code}\""))
    }
}

impl Lang {
    pub(crate) fn from_name(name: &str) -> Result<Self> {
        if let Some(lang) = LANGUAGES.name2lang(name) {
            return Ok(lang);
        }
        Err(anyhow!("Unknown lang canonical name \"{name}\""))
    }

    pub(crate) fn id(self) -> LangId {
        self.0
    }

    fn data(self) -> &'static LangData {
        LANGUAGES.data(self)
    }

    pub(crate) fn code(self) -> &'static str {
        self.data().code
    }

    pub(crate) fn name(self) -> &'static str {
        self.data().name
    }

    pub(crate) fn url_name(self) -> &'static str {
        &self.data().url_name
    }

    pub(crate) fn ety2non(self) -> Self {
        self.data().non_ety
    }

    pub(crate) fn is_reconstructed(self) -> bool {
        self.data().kind == LangKind::Reconstructed
    }

    pub(crate) fn ancestors(self) -> &'static [Lang] {
        &self.data().ancestors
    }

    pub(crate) fn descends_from(self, lang: Lang) -> bool {
        self.ancestors().contains(&lang)
    }

    pub(crate) fn strictly_descends_from(self, lang: Lang) -> bool {
        self != lang && self.descends_from(lang)
    }

    pub(crate) fn distance_from(self, lang: Lang) -> Option<usize> {
        if self == lang {
            return Some(0);
        }

        let ancestors_self = self.ancestors();
        let ancestors_lang = lang.ancestors();

        if ancestors_self.first() != ancestors_lang.first() {
            return None;
        }

        let (longer, shorter) = if ancestors_self.len() >= ancestors_lang.len() {
            (ancestors_self, ancestors_lang)
        } else {
            (ancestors_lang, ancestors_self)
        };

        let mut distance = ancestors_self.len() + ancestors_lang.len();

        for (shorter_ancestor, longer_ancestor) in shorter.iter().zip(longer.iter()) {
            if shorter_ancestor != longer_ancestor {
                return Some(distance);
            }
            distance -= 2;
        }

        Some(distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_from_code() {
        let en = Lang::from_str("en").unwrap();
        assert_eq!(en.code(), "en");
        let vl1 = Lang::from_str("VL.").unwrap();
        assert_eq!(vl1.code(), "la-vul");
        let vl2 = Lang::from_str("VL").unwrap();
        assert_eq!(vl2.code(), "la-vul");
        let vl3 = Lang::from_str("la-vul").unwrap();
        assert_eq!(vl3.code(), "la-vul");
        assert_eq!(vl1, vl2);
        assert_eq!(vl2, vl3);
        let nl = Lang::from_str("nl").unwrap();
        assert_eq!(nl.code(), "nl");
        assert_eq!(nl.name(), "Dutch");
    }

    #[test]
    fn lang_from_name() {
        let en = Lang::from_name("English").unwrap();
        assert_eq!(en.code(), "en");
        let vl = Lang::from_name("Vulgar Latin").unwrap();
        assert_eq!(vl.code(), "la-vul");
    }

    #[test]
    fn lang_non_ety() {
        let vulgar_latin = Lang::from_str("la-vul").unwrap();
        let old_latin = Lang::from_str("itc-ola").unwrap();
        let latin = Lang::from_str("la").unwrap();
        assert_eq!(vulgar_latin.ety2non(), latin);
        assert_eq!(old_latin.ety2non(), latin);
    }

    #[test]
    fn lang_kind() {
        let en = Lang::from_str("en").unwrap();
        assert!(!en.is_reconstructed());
        let ine_pro = Lang::from_str("ine-pro").unwrap();
        assert!(ine_pro.is_reconstructed());
    }

    #[test]
    fn lang_ancestors() {
        let en = Lang::from_str("en").unwrap();
        let known_ancestors = ["ine-pro", "gem-pro", "gmw-pro", "ang", "enm", "en"];
        assert_eq!(
            en.ancestors(),
            known_ancestors
                .iter()
                .map(|&code| code.parse().unwrap())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn lang_strictly_descends_from() {
        let vulgar_latin = Lang::from_str("la-vul").unwrap();
        let classical_latin = Lang::from_str("la-cla").unwrap();
        let old_latin = Lang::from_str("itc-ola").unwrap();
        let latin = Lang::from_str("la").unwrap();
        let proto_italic = Lang::from_str("itc-pro").unwrap();
        let pie = Lang::from_str("ine-pro").unwrap();

        assert!(!vulgar_latin.strictly_descends_from(vulgar_latin));
        assert!(!vulgar_latin.strictly_descends_from(latin));
        assert!(vulgar_latin.strictly_descends_from(classical_latin));
        assert!(vulgar_latin.strictly_descends_from(old_latin));
        assert!(vulgar_latin.strictly_descends_from(proto_italic));
        assert!(vulgar_latin.strictly_descends_from(pie));

        assert!(!classical_latin.strictly_descends_from(classical_latin));
        assert!(!classical_latin.strictly_descends_from(latin));
        assert!(classical_latin.strictly_descends_from(old_latin));
        assert!(classical_latin.strictly_descends_from(proto_italic));
        assert!(classical_latin.strictly_descends_from(pie));

        assert!(!old_latin.strictly_descends_from(old_latin));
        assert!(!old_latin.strictly_descends_from(latin));
        assert!(old_latin.strictly_descends_from(proto_italic));
        assert!(old_latin.strictly_descends_from(pie));

        assert!(!latin.strictly_descends_from(latin));
        assert!(latin.strictly_descends_from(proto_italic));
        assert!(latin.strictly_descends_from(pie));

        assert!(!proto_italic.strictly_descends_from(proto_italic));
        assert!(proto_italic.strictly_descends_from(pie));

        assert!(!pie.strictly_descends_from(pie));
    }

    #[test]
    fn lang_distance() {
        // la-vul -> la-cla -> itc-ola -> itc-pro -> ine-pro
        let vulgar_latin = Lang::from_str("la-vul").unwrap();
        let classical_latin = Lang::from_str("la-cla").unwrap();
        let old_latin = Lang::from_str("itc-ola").unwrap();
        // la -> itc-ola -> itc-pro -> ine-pro
        let latin = Lang::from_str("la").unwrap();
        let proto_italic = Lang::from_str("itc-pro").unwrap();
        let pie = Lang::from_str("ine-pro").unwrap();

        assert_eq!(vulgar_latin.distance_from(vulgar_latin), Some(0));
        assert_eq!(vulgar_latin.distance_from(classical_latin), Some(1));
        assert_eq!(classical_latin.distance_from(vulgar_latin), Some(1));
        assert_eq!(vulgar_latin.distance_from(latin), Some(3));
        assert_eq!(latin.distance_from(vulgar_latin), Some(3));
        assert_eq!(vulgar_latin.distance_from(old_latin), Some(2));
        assert_eq!(old_latin.distance_from(vulgar_latin), Some(2));
        assert_eq!(proto_italic.distance_from(pie), Some(1));

        // fr -> frm -> fro -> la -> itc-ola -> itc-pro -> ine-pro
        let fr = Lang::from_str("fr").unwrap();
        assert_eq!(fr.distance_from(latin), Some(3));
        assert_eq!(latin.distance_from(fr), Some(3));

        // en -> enm -> ang -> gmw-pro -> gem-pro -> ine-pro
        let en = Lang::from_str("en").unwrap();
        let enm = Lang::from_str("enm").unwrap();
        assert_eq!(en.distance_from(enm), Some(1));
        assert_eq!(enm.distance_from(en), Some(1));
        assert_eq!(en.distance_from(latin), Some(8));
        assert_eq!(latin.distance_from(en), Some(8));
        assert_eq!(en.distance_from(fr), Some(11));

        let ar = Lang::from_str("ar").unwrap();
        assert_eq!(ar.distance_from(latin), None);
        assert_eq!(latin.distance_from(ar), None);
    }
}
