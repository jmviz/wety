use std::{env, fs, path::PathBuf};
use sha2::{Digest, Sha256};

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo = crate_dir.join("../..");
    let selection_path = repo.join("data/registry/inputs.json");
    let selection: serde_json::Value = serde_json::from_slice(&fs::read(&selection_path).unwrap()).unwrap();
    let relative = selection["wiktionary_languages"].as_str().unwrap();
    assert!(!PathBuf::from(relative).is_absolute() && !relative.split('/').any(|s| s == ".."));
    let language_path = repo.join(relative).canonicalize().unwrap();
    let pos_path = crate_dir.join("src/pos_phf.rs");
    let mut hash = Sha256::new();
    hash.update(b"wety-reference-tables-v1\0");
    hash.update(fs::read(&language_path).unwrap());
    hash.update(b"\0");
    hash.update(fs::read(&pos_path).unwrap());
    println!("cargo:rustc-env=WETY_REFERENCE_FINGERPRINT={:x}", hash.finalize());
    println!("cargo:rustc-env=WETY_LANGUAGES_JSON={}", language_path.display());
    for path in [&selection_path, &language_path, &pos_path] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
