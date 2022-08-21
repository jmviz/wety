# wety
WIP project to digest etymologies from [wiktextract](https://github.com/tatuylonen/wiktextract) data.

## Usage
If you do not have rust installed, [install it](https://www.rust-lang.org/tools/install). Clone this repo and `cd` into it. Download the latest wiktextract raw data from [https://kaikki.org/dictionary/rawdata.html](https://kaikki.org/dictionary/rawdata.html), namely [this file](https://kaikki.org/dictionary/raw-wiktextract-data.json.gz), into `/data`. Run the rust program:
```bash
cargo run --release
```
This will parse the raw data, process etymologies, and output a Turtle file and Oxigraph store. 
