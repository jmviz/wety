# wety
WIP project to digest etymologies from [wiktextract](https://github.com/tatuylonen/wiktextract) data.

## Envisioned usage
If you do not have rust installed, [install it](https://www.rust-lang.org/tools/install). Clone this repo and `cd` into it. Run the rust program:
```bash
cargo run --release
```
This will begin downloading the most recent raw compressed wiktextract data from [https://kaikki.org/dictionary/rawdata.html](https://kaikki.org/dictionary/rawdata.html), namely [this file](https://kaikki.org/dictionary/raw-wiktextract-data.json.gz). It will decompress and process the data on the fly while it is downloading. The remote file will not be saved locally. If you want a local copy, download it manually from the website and save it in `/data`. If the program sees a file called `raw-wiktextract-data.json.gz` in `/data`, it will skip the download and just process the local file $$needs implemented$$.