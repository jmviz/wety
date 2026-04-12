# [wety](https://www.wety.org/)

(w)iktionary (ety)mologies. `processor` uses [`wiktextract`](https://github.com/tatuylonen/wiktextract) data to generate an etymological graph of all words on wiktionary. `server` serves the data. [...]

## Installation

Clone this repo.

If you do not have Rust installed, [install it](https://www.rust-lang.org/tools/install). This project uses `nightly` Rust and assumes you have a recent version. When you build the project, the `ni[...] 

## `processor` usage

Make sure you are in the root directory of this repo. Download the latest `wiktextract` raw data from [https://kaikki.org/dictionary/rawdata.html](https://kaikki.org/dictionary/rawdata.html), name[...] 

```bash
cargo run --release --bin processor
```

It will take a while to compile, and even longer to run :). By default, it will process the raw wiktextract data and produce a gz-compressed JSON serialization of the data structure used by `serve[...] 

`processor` uses an embeddings model for word sense disambiguation. Note that the first time this is run, the model files will be downloaded from Hugging Face and placed in `~/.cache/huggingface/h[...] 

The `wiktextract` raw data file will be automatically decompressed if the `--wiktextract-path` argument has the extension `.gz`, otherwise it will be read as a standard JSON Lines file. Similarly,[...] 

If you get a CUDA out of memory error, or if you are using CPU and the process gets killed due to RAM usage, try setting `--embeddings-batch-size` lower. The default value was set as the nearest r[...] 

**N.B.** If you have previously run `processor` and on a subsequent run decide to change the `embeddings-model`, be sure to delete `data/embeddings_cache` to remove the incompatible previously-cac[...] 

## `server` usage

You must have run `processor` first, with the serialized processed data having been written to `data/wety.json.gz` or `data/wety.json` (the latter will load faster on server startup).

Alternatively, if you don't want to spend time running `processor` yourself, you can download the current processed data that [wety.org](https://www.wety.org) is using from [data.wety.org](http://[...] 

To run the server:

```bash
cargo run --release --bin server
```

Requests to the server can be made at `127.0.0.1:3000`. For development in conjuction with the frontend, see the README in the `client` subdirectory for instructions on setting up and running the [...]