use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo = crate_dir.join("../..");
    let selection_path = repo.join("data/registry/inputs.json");
    let selection: serde_json::Value =
        serde_json::from_slice(&fs::read(&selection_path).unwrap()).unwrap();
    let relative = selection["wiktionary_languages"].as_str().unwrap();
    assert!(!PathBuf::from(relative).is_absolute() && !relative.split('/').any(|s| s == ".."));
    let language_path = repo.join(relative).canonicalize().unwrap();
    let relative = selection["wiktionary_families"].as_str().unwrap();
    assert!(!PathBuf::from(relative).is_absolute() && !relative.split('/').any(|s| s == ".."));
    let family_path = repo.join(relative).canonicalize().unwrap();
    let pos_path = crate_dir.join("src/pos_phf.rs");
    let mut hash = Sha256::new();
    hash.update(b"wety-reference-tables-v1\0");
    hash.update(fs::read(&language_path).unwrap());
    hash.update(b"\0");
    hash.update(fs::read(&family_path).unwrap());
    hash.update(b"\0");
    hash.update(fs::read(&pos_path).unwrap());
    let fingerprint: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    println!("cargo:rustc-env=WETY_REFERENCE_FINGERPRINT={fingerprint}");
    println!(
        "cargo:rustc-env=WETY_LANGUAGES_JSON={}",
        language_path.display()
    );
    println!(
        "cargo:rustc-env=WETY_FAMILIES_JSON={}",
        family_path.display()
    );
    // Codes Wiktionary has retired, mapped to their successors, so that links
    // naming them keep working. Not part of the fingerprint: it does not
    // change any numeric id.
    let retired_path = repo
        .join("data/registry/retired-language-codes.json")
        .canonicalize()
        .unwrap();
    println!(
        "cargo:rustc-env=WETY_RETIRED_LANGUAGE_CODES_JSON={}",
        retired_path.display()
    );
    for path in [
        &selection_path,
        &language_path,
        &family_path,
        &pos_path,
        &retired_path,
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
