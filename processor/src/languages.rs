use crate::HashMap;

use std::hash::Hash;
use std::str::FromStr;

use anyhow::anyhow;
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
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
    kind: &'static str,
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

    fn code2main(&self, code: &str) -> Option<&str> {
        self.get(code).map(|language| language.code)
    }

    fn name2code(&self, name: &str) -> Option<&str> {
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

struct Lang {
    code: &'static str,
}

impl FromStr for Lang {
    type Err = anyhow::Error;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        // get the main code
        if let Some(code) = LANGUAGES.get(code).map(|language| language.code) {
            return Ok(Lang { code });
        }
        Err(anyhow!("Unknown lang code \"{code}\""))
    }
}
