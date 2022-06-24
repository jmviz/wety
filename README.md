# wety
WIP project to digest etymologies from [wiktextract](https://github.com/tatuylonen/wiktextract) data.

## Envisioned usage
If you do not have rust installed, [install it](https://www.rust-lang.org/tools/install). Clone this repo and `cd` into it. Download the [raw wiktextract data](https://kaikki.org/dictionary/raw-wiktextract-data.json.gz) to `/data/download`. Unzip it. Then run the rust program.
```bash
wget -O data/download/raw-wiktextract-data.json.gz https://kaikki.org/dictionary/raw-wiktextract-data.json.gz
gunzip data/download/raw-wiktextract-data.json.gz
cargo run --release
``` 

