# wety
WIP project to digest etymologies from [wiktextract](https://github.com/tatuylonen/wiktextract) data.

## Usage
If you do not have rust installed, [install it](https://www.rust-lang.org/tools/install). Clone this repo and `cd` into it. Run the rust program:
```bash
cargo run --release
```
This will begin downloading the most recent raw compressed wiktextract data from [https://kaikki.org/dictionary/rawdata.html](https://kaikki.org/dictionary/rawdata.html), namely [this file](https://kaikki.org/dictionary/raw-wiktextract-data.json.gz). It will be saved as `data/raw-wiktextract-data.json.gz`. If the program sees that `data/raw-wiktextract-data.json.gz` already exists, it will skip the download and just process the local file. This file will then be processed to digest etymologies.