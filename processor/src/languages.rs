use crate::HashMap;

use std::str::FromStr;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LangKind {
    Regular,
    Reconstructed,
    EtymologyOnly,
    AppendixConstructed,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Language {
    // aliases: Vec<&'static str>,
    ancestors: Vec<&'static str>,
    canonical_name: &'static str,
    // For etymology-only languages, this is the mainCode; it may not be the
    // same as the code that maps to the Language in Code2Language. For example,
    // Vulgar Latin codes "VL" and "VL." both have mainCode "la-vul".
    code: &'static str,
    // family: Option<&'static str>,
    kind: LangKind,
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

struct Languages {
    languages: Vec<Language>,
    code2id: HashMap<&'static str, LangId>,
    name2id: HashMap<&'static str, LangId>,
}

impl Languages {
    fn new(languages: Vec<Language>) -> Self {
        let mut code2id = HashMap::default();
        let mut name2id = HashMap::default();
        for (idx, language) in languages.iter().enumerate() {
            let id = LangId::try_from(idx).expect("much fewer than 65,535 languages");
            code2id.insert(language.code, id);
            if language.code == language.main_code {
                name2id.insert(language.canonical_name, id);
            }
        }

        let me = Self {
            languages,
            code2id,
            name2id,
        };
        me.validate();
        me
    }

    fn validate(&self) {
        for language in &self.languages {
            for ancestor in &language.ancestors {
                assert!(
                    self.code2id.contains_key(ancestor),
                    "ancestor {} of {} not in languages.json",
                    ancestor,
                    language.code
                );
            }
            assert!(
                self.code2id.contains_key(language.code),
                "code {} not in languages.json",
                language.code
            );
            assert!(
                self.code2id.contains_key(language.main_code),
                "main code {} of {} not in languages.json",
                language.main_code,
                language.code
            );
            assert!(
                self.code2id.contains_key(language.non_etymology_only),
                "non-etymology-only code {} of {} not in languages.json",
                language.non_etymology_only,
                language.code
            );
            assert!(
                self.name2id.contains_key(language.canonical_name),
                "canonical name {} not in languages.json",
                language.canonical_name
            );
        }
    }

    fn index(&self, id: LangId) -> &Language {
        &self.languages[id as usize]
    }

    fn code2id(&self, code: &str) -> Option<LangId> {
        self.code2id.get(code).copied()
    }

    fn code2language(&self, code: &str) -> Option<&Language> {
        self.code2id(code).map(|id| self.index(id))
    }

    fn code2main_id(&self, code: &str) -> Option<LangId> {
        let language = self.code2language(code)?;
        self.code2id(language.main_code)
    }

    // the id returned is guaranteed to be the index of the language whose code
    // == main_code due to the construction in Languages::new()
    fn name2id(&self, name: &str) -> Option<LangId> {
        self.name2id.get(name).copied()
    }
}

lazy_static! {
    static ref LANGUAGES: Languages = Languages::new(
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data/languages.json"
        )))
        .expect("well-formed languages.json")
    );
}

#[derive(Default, Hash, Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Lang(LangId);

impl FromStr for Lang {
    type Err = anyhow::Error;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        if let Some(id) = LANGUAGES.code2main_id(code) {
            return Ok(Lang(id));
        }
        Err(anyhow!("Unknown lang code \"{code}\""))
    }
}

impl Lang {
    pub(crate) fn from_name(name: &str) -> Result<Self> {
        if let Some(id) = LANGUAGES.name2id(name) {
            return Ok(Lang(id));
        }
        Err(anyhow!("Unknown lang canonical name \"{name}\""))
    }

    pub(crate) fn id(self) -> LangId {
        self.0
    }

    fn data(&self) -> &Language {
        LANGUAGES.index(self.id())
    }

    #[allow(clippy::misnamed_getters)]
    pub(crate) fn code(self) -> &'static str {
        self.data().main_code
    }

    pub(crate) fn name(self) -> &'static str {
        self.data().canonical_name
    }

    pub(crate) fn ety2non(self) -> Self {
        self.data()
            .non_etymology_only
            .parse()
            .expect("validated lang code")
    }

    pub(crate) fn is_reconstructed(self) -> bool {
        self.data().kind == LangKind::Reconstructed
    }

    pub(crate) fn ancestors(self) -> Vec<Lang> {
        self.data()
            .ancestors
            .iter()
            .map(|&code| code.parse().expect("validated lang code"))
            .collect()
    }

    pub(crate) fn strictly_descends_from(self, lang: Lang) -> bool {
        self.ancestors().contains(&lang)
    }

    // pub(crate) fn loosely_descends_from(self, lang: Lang) -> bool {
    //     if self.strictly_descends_from(lang) || self == lang {
    //         return true;
    //     }
    //     let non_ety_lang = self.ety2non();
    //     if non_ety_lang == self {
    //         return false;
    //     }
    //     non_ety_lang == lang || non_ety_lang.ancestors().contains(&lang)
    // }
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
        let known_ancestors = ["ine-pro", "gem-pro", "gmw-pro", "ang", "enm"];
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

    // #[test]
    // fn lang_loosely_descends_from() {
    //     let vulgar_latin = Lang::from_str("la-vul").unwrap();
    //     let classical_latin = Lang::from_str("la-cla").unwrap();
    //     let old_latin = Lang::from_str("itc-ola").unwrap();
    //     let latin = Lang::from_str("la").unwrap();
    //     let proto_italic = Lang::from_str("itc-pro").unwrap();
    //     let pie = Lang::from_str("ine-pro").unwrap();

    //     assert!(vulgar_latin.loosely_descends_from(vulgar_latin));
    //     assert!(vulgar_latin.loosely_descends_from(classical_latin));
    //     assert!(vulgar_latin.loosely_descends_from(old_latin));
    //     assert!(vulgar_latin.loosely_descends_from(latin));
    //     assert!(vulgar_latin.loosely_descends_from(proto_italic));
    //     assert!(vulgar_latin.loosely_descends_from(pie));

    //     assert!(classical_latin.loosely_descends_from(classical_latin));
    //     assert!(classical_latin.loosely_descends_from(old_latin));
    //     assert!(classical_latin.loosely_descends_from(latin));
    //     assert!(classical_latin.loosely_descends_from(proto_italic));
    //     assert!(classical_latin.loosely_descends_from(pie));

    //     assert!(old_latin.loosely_descends_from(old_latin));
    //     assert!(old_latin.loosely_descends_from(latin));
    //     assert!(old_latin.loosely_descends_from(proto_italic));
    //     assert!(old_latin.loosely_descends_from(pie));

    //     assert!(latin.loosely_descends_from(latin));
    //     assert!(latin.loosely_descends_from(old_latin));
    //     assert!(latin.loosely_descends_from(proto_italic));
    //     assert!(latin.loosely_descends_from(pie));

    //     assert!(proto_italic.loosely_descends_from(proto_italic));
    //     assert!(proto_italic.loosely_descends_from(pie));

    //     assert!(pie.loosely_descends_from(pie));
    // }
}
