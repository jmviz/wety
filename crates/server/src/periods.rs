//! When each language was spoken, from the dating registry's export
//! (`language_periods.json`), keyed for the client by lang id.
//!
//! The file is optional: without it the server serves an empty list, so a
//! host that has not received an export yet still runs. Codes the compiled
//! language table doesn't know are dropped (and logged) rather than guessed.

use std::{io::ErrorKind, path::Path, str::FromStr};

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use serde_json::{Value, json};
use wety_core::Lang;

pub const DEFAULT_PERIODS_PATH: &str = "data/products/dating/language_periods.json";

pub struct Periods {
    /// The serialized `/periods` response.
    pub bytes: Bytes,
    pub languages: usize,
    pub unknown_codes: Vec<String>,
}

impl Periods {
    #[must_use]
    pub fn empty() -> Self {
        Self::from_value(&json!({})).expect("an empty export is valid")
    }

    /// # Errors
    ///
    /// Returns `Err` if the file exists but can't be read or isn't a valid export.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read(path) {
            Ok(raw) => {
                let value: Value = serde_json::from_slice(&raw)
                    .with_context(|| format!("parsing {}", path.display()))?;
                Self::from_value(&value).with_context(|| format!("reading {}", path.display()))
            }
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Self::empty()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    /// # Errors
    ///
    /// Returns `Err` if a language record has no string `code`.
    pub fn from_value(export: &Value) -> Result<Self> {
        let mut languages = Vec::new();
        let mut unknown_codes = Vec::new();
        for record in export["languages"].as_array().into_iter().flatten() {
            let code = record["code"]
                .as_str()
                .ok_or_else(|| anyhow!("language period without a code"))?;
            let Ok(lang) = Lang::from_str(code) else {
                unknown_codes.push(code.to_string());
                continue;
            };
            let mut record = record.clone();
            record["lang"] = lang.json();
            languages.push(record);
        }
        let body = json!({
            "generation": export["generation"],
            "languages": languages,
            "family_splits": export.get("family_splits").cloned().unwrap_or_else(|| json!([])),
        });
        Ok(Self {
            bytes: Bytes::from(serde_json::to_vec(&body)?),
            languages: languages.len(),
            unknown_codes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(periods: &Periods) -> Value {
        serde_json::from_slice(&periods.bytes).unwrap()
    }

    #[test]
    fn missing_file_serves_empty_list() {
        let periods = Periods::load(Path::new("/nonexistent/language_periods.json")).unwrap();
        assert_eq!(periods.languages, 0);
        assert_eq!(body(&periods)["languages"], json!([]));
        assert_eq!(body(&periods)["family_splits"], json!([]));
    }

    #[test]
    fn codes_map_to_lang_ids_and_unknown_codes_are_dropped() {
        let english = Lang::from_str("en").unwrap();
        let export = json!({
            "generation": "abc",
            "languages": [
                {"code": "en", "label": "c. 1500 CE–present", "basis": "conventional"},
                {"code": "not-a-code", "label": "x"},
            ],
            "family_splits": [{"node": "Indo-European"}],
        });
        let periods = Periods::from_value(&export).unwrap();
        assert_eq!(periods.languages, 1);
        assert_eq!(periods.unknown_codes, vec!["not-a-code".to_string()]);
        let served = body(&periods);
        assert_eq!(served["generation"], "abc");
        assert_eq!(served["languages"][0]["lang"]["id"], json!(english.id()));
        assert_eq!(served["languages"][0]["lang"]["name"], json!(english.name()));
        assert_eq!(served["languages"][0]["label"], "c. 1500 CE–present");
        assert_eq!(served["family_splits"][0]["node"], "Indo-European");
    }

    #[test]
    fn record_without_code_is_an_error() {
        assert!(Periods::from_value(&json!({"languages": [{"label": "x"}]})).is_err());
    }

    #[test]
    fn invalid_file_is_an_error() {
        let path = std::env::temp_dir().join(format!("wety-periods-{}.json", std::process::id()));
        std::fs::write(&path, b"not json").unwrap();
        assert!(Periods::load(&path).is_err());
        std::fs::remove_file(&path).unwrap();
    }
}
