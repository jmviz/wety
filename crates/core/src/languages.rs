use crate::HashMap;

use std::{collections::BTreeMap, str::FromStr};

use anyhow::{Ok, Result, anyhow};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
    /// retired code -> its successor, for links made before a refresh
    retired2id: HashMap<&'static str, Lang>,
}

impl Languages {
    fn new() -> Self {
        let code2raw_data: BTreeMap<&'static str, RawLangData> = serde_json::from_str(
            include_str!(env!("WETY_LANGUAGES_JSON")),
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

        let retired: BTreeMap<&'static str, &'static str> = serde_json::from_str(include_str!(
            env!("WETY_RETIRED_LANGUAGE_CODES_JSON")
        ))
        .expect("well-formed retired-language-codes.json");
        let retired2id = retired
            .into_iter()
            .filter(|(old, _)| !code2id.contains_key(old))
            .filter_map(|(old, new)| Some((old, *code2id.get(new)?)))
            .collect();

        Self {
            data,
            code2id,
            name2id,
            retired2id,
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

    fn retired2lang(&self, code: &str) -> Option<Lang> {
        self.retired2id.get(code).copied()
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
    /// Like `from_str`, but also accepts codes Wiktionary has retired (see
    /// `data/registry/retired-language-codes.json`). For codes from links.
    ///
    /// # Errors
    ///
    /// Returns an error if `code` is neither a current nor a retired code.
    pub fn from_code(code: &str) -> Result<Self> {
        Self::from_str(code).or_else(|e| LANGUAGES.retired2lang(code).ok_or(e))
    }

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

    /// Whether this id names a language in the compiled table.
    #[must_use]
    pub fn exists(self) -> bool {
        usize::from(self.id()) < LANGUAGES.data.len()
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
        json!(self.info())
    }
}

/// What the client sees for a lang (`Lang` in `search/types.ts`).
#[derive(Serialize)]
pub struct LangInfo {
    pub id: LangId,
    pub code: &'static str,
    pub name: &'static str,
}
