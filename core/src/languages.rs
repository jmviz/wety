use crate::HashMap;

use std::{collections::BTreeMap, str::FromStr};

use anyhow::{anyhow, Ok, Result};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    ancestors: Vec<&'static str>,
    canonical_name: &'static str,
    kind: LangKind,
    main_code: &'static str,
    non_etymology_only: &'static str,
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

static LANGUAGES: LazyLock<Languages> = LazyLock::new(Languages::new);

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
    /// # Errors
    ///
    /// Returns an error if `name` is not a known canonical language name.
    pub fn from_name(name: &str) -> Result<Self> {
        if let Some(lang) = LANGUAGES.name2lang(name) {
            return Ok(lang);
        }
        Err(anyhow!("Unknown lang canonical name \"{name}\""))
    }

    #[must_use]
    pub fn id(self) -> LangId {
        self.0
    }

    fn data(self) -> &'static LangData {
        LANGUAGES.data(self)
    }

    #[must_use]
    pub fn code(self) -> &'static str {
        self.data().code
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        self.data().name
    }

    #[must_use]
    pub fn url_name(self) -> &'static str {
        &self.data().url_name
    }

    #[must_use]
    pub fn ety2non(self) -> Self {
        self.data().non_ety
    }

    #[must_use]
    pub fn is_reconstructed(self) -> bool {
        self.data().kind == LangKind::Reconstructed
    }

    #[must_use]
    pub fn ancestors(self) -> &'static [Lang] {
        &self.data().ancestors
    }

    #[must_use]
    pub fn descends_from(self, lang: Lang) -> bool {
        self.ancestors().contains(&lang)
    }

    #[must_use]
    pub fn strictly_descends_from(self, lang: Lang) -> bool {
        self != lang && self.descends_from(lang)
    }

    #[must_use]
    pub fn distance_from(self, lang: Lang) -> Option<usize> {
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

    #[must_use]
    pub fn json(self) -> Value {
        json!({
            "id": self.id(),
            "name": self.name(),
        })
    }
}
