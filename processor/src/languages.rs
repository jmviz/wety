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
    non_etymology_only: &'static str,
    // other_names: Vec<&'static str>,
    // parents: Vec<&'static str>,
    // scripts: Vec<&'static str>,
    // varieties: Vec<&'static str>,
    // wikidata_item: &'static str,
    // wikipedia_article: &'static str,
}

type Code2Language = HashMap<&'static str, Language>;

struct Languages {
    code2language: Code2Language,
    name2code: HashMap<&'static str, &'static str>,
}

impl Languages {
    fn new(code2language: Code2Language) -> Self {
        let mut name2code = HashMap::default();
        for language in code2language.values() {
            // importantly, this maps canonical names to mainCodes
            name2code.insert(language.canonical_name, language.code);
        }
        Self {
            code2language,
            name2code,
        }
    }

    fn get(&self, code: &str) -> Option<&Language> {
        self.code2language.get(code)
    }

    fn get_known(&self, code: &str) -> &Language {
        self.get(code).expect("known lang code")
    }

    fn code2main(&self, code: &str) -> Option<&'static str> {
        self.get(code).map(|language| language.code)
    }

    fn name2code(&self, name: &str) -> Option<&'static str> {
        self.name2code.get(name).copied()
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
pub struct Lang(&'static str); // the inner value is the lang (main) code

impl FromStr for Lang {
    type Err = anyhow::Error;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        // get the main code
        if let Some(code) = LANGUAGES.code2main(code) {
            return Ok(Lang(code));
        }
        Err(anyhow!("Unknown lang code \"{code}\""))
    }
}

impl Lang {
    pub(crate) fn from_name(name: &str) -> Result<Self> {
        if let Some(code) = LANGUAGES.name2code(name) {
            return Ok(Lang(code));
        }
        Err(anyhow!("Unknown lang name \"{name}\""))
    }

    pub(crate) fn code(&self) -> &'static str {
        self.0
    }

    pub(crate) fn name(&self) -> &'static str {
        LANGUAGES.get_known(self.code()).canonical_name
    }

    pub(crate) fn ety2non(&self) -> Self {
        let code = LANGUAGES.get_known(self.code()).non_etymology_only;
        Lang(code)
    }

    pub(crate) fn is_reconstructed(&self) -> bool {
        LANGUAGES.get_known(self.code()).kind == LangKind::Reconstructed
    }

    pub(crate) fn ancestors(&self) -> Vec<Lang> {
        LANGUAGES
            .get_known(self.code())
            .ancestors
            .iter()
            .map(|&code| Lang(code))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_from_code() {
        let lang_en = Lang::from_str("en").unwrap();
        assert_eq!(lang_en.code(), "en");
        let lang_vl1 = Lang::from_str("VL.").unwrap();
        assert_eq!(lang_vl1.code(), "la-vul");
        let lang_vl2 = Lang::from_str("VL").unwrap();
        assert_eq!(lang_vl2.code(), "la-vul");
        let lang_vl3 = Lang::from_str("la-vul").unwrap();
        assert_eq!(lang_vl3.code(), "la-vul");
    }

    #[test]
    fn lang_from_name() {
        let lang_en = Lang::from_name("English").unwrap();
        assert_eq!(lang_en.code(), "en");
        let lang_vl = Lang::from_name("Vulgar Latin").unwrap();
        assert_eq!(lang_vl.code(), "la-vul");
    }

    #[test]
    fn lang_kind() {
        let lang_en = Lang::from_str("en").unwrap();
        assert!(!lang_en.is_reconstructed());
        let lang_ine_pro = Lang::from_str("ine-pro").unwrap();
        assert!(lang_ine_pro.is_reconstructed());
    }

    #[test]
    fn lang_ancestors() {
        let lang_en = Lang::from_str("en").unwrap();
        let known_ancestors = ["ine-pro", "gem-pro", "gmw-pro", "ang", "enm"];
        assert_eq!(
            lang_en.ancestors(),
            known_ancestors
                .iter()
                .map(|&code| Lang(code))
                .collect::<Vec<_>>()
        );
    }
}
