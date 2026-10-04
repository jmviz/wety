//! After selecting a new Wiktionary language export, record the codes it
//! retired so that links naming them keep resolving:
//!
//!     cargo run --locked -p wety-core --example retired_language_codes -- PREVIOUS/languages.json [--dry-run]
//!
//! A main code of the previous export that the current one (compiled into
//! this binary) no longer has is mapped to the current main code whose name,
//! alias or other name is its old canonical name, when exactly one does.
//! Entries already in `data/registry/retired-language-codes.json` are kept,
//! re-pointed if their successor was itself retired.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

type Table = BTreeMap<String, Value>;

fn names(entry: &Value) -> impl Iterator<Item = &str> {
    let list = |key| entry[key].as_array().into_iter().flatten().filter_map(Value::as_str);
    entry["canonicalName"].as_str().into_iter().chain(list("aliases")).chain(list("otherNames"))
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let previous_path = args.next().expect("path to the previous languages.json");
    let dry_run = args.any(|a| a == "--dry-run");
    let previous: Table = serde_json::from_slice(&std::fs::read(previous_path)?)?;
    let current: Table = serde_json::from_str(include_str!(env!("WETY_LANGUAGES_JSON")))?;
    let registry_path = env!("WETY_RETIRED_LANGUAGE_CODES_JSON");
    let mut retired: BTreeMap<String, String> =
        serde_json::from_slice(&std::fs::read(registry_path)?)?;

    let main_code = |table: &Table, code: &str| -> Option<String> {
        table.get(code)?["mainCode"].as_str().map(str::to_owned)
    };
    let mut by_name: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for entry in current.values() {
        let main = entry["mainCode"].as_str().unwrap_or_default().to_owned();
        for name in names(entry) {
            by_name.entry(name).or_default().insert(main.clone());
        }
    }

    // Earlier entries whose successor is gone too follow it to its own.
    let mut dropped = vec![];
    for (old, new) in retired.clone() {
        if current.contains_key(&old) {
            retired.remove(&old);
        } else if !current.contains_key(&new) {
            match retired.get(&new).cloned() {
                Some(next) => {
                    retired.insert(old, next);
                }
                None => dropped.push(old),
            }
        }
    }

    let (mut added, mut unresolved) = (0, vec![]);
    for (code, entry) in &previous {
        if main_code(&previous, code).as_deref() != Some(code) || current.contains_key(code) {
            continue;
        }
        let name = entry["canonicalName"].as_str().unwrap_or_default();
        match by_name.get(name).map(|codes| codes.iter().collect::<Vec<_>>()).as_deref() {
            Some([successor]) => {
                retired.insert(code.clone(), (*successor).clone());
                added += 1;
            }
            _ => unresolved.push(format!("{code} ({name})")),
        }
    }
    for old in &dropped {
        retired.remove(old);
    }

    println!("{added} retired codes mapped; {} without a unique successor:", unresolved.len());
    for code in &unresolved {
        println!("  {code}");
    }
    if !dropped.is_empty() {
        println!("dropped (successor retired without a successor): {dropped:?}");
    }
    if !dry_run {
        std::fs::write(registry_path, serde_json::to_string_pretty(&retired)? + "\n")?;
        println!("wrote {registry_path}");
    }
    Ok(())
}
