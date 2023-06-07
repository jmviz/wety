# [wety](https://www.wety.org/)
(w)iktionary (ety)mologies. `processor` uses [`wiktextract`](https://github.com/tatuylonen/wiktextract) data to generate an etymological graph of all words on wiktionary. `server` serves the data. For the frontend, see [`wety-client`](https://github.com/jmviz/wety-client).

## `processor` usage
If you do not have rust installed, [install it](https://www.rust-lang.org/tools/install). Clone this repo and `cd` into it. Download the latest `wiktextract` raw data from [https://kaikki.org/dictionary/rawdata.html](https://kaikki.org/dictionary/rawdata.html), namely [this file](https://kaikki.org/dictionary/raw-wiktextract-data.json.gz), into `/data`. This uses `rust-bert`, which uses `pytorch`, for word sense disambiguations. So follow the [`rust-bert` installation instructions](https://github.com/guillaume-be/rust-bert#manual-installation-recommended) if you have a CUDA-enabled GPU you want to use instead of the default CPU backend. Some platforms (e.g. ARM-based ones) may have to compile `pytorch` from source. Finally, Run the rust program:

```bash
cargo run --release --bin processor
```

It will take a while to compile, and even longer to run :). By default, it will process the raw wiktextract data and produce a gz-compressed JSON serialization of the data structure used by `server`. It also can generate a [Turtle](https://www.w3.org/TR/turtle/) file for loading into a graph database, e.g. [Oxigraph](https://github.com/oxigraph/oxigraph). See `cargo run --release --bin processor -- --help` for all options. Note that the first time this is run, the relevant `rust-bert` files will be downloaded from Hugging Face and placed in `~/.cache/.rustbert`. On subsequent runs, the files will be read from this cache rather than redownloaded. Similarly, on the first run, embeddings will be generated for all items determined to need them. This will take the lion's share of processing time. On subsequent runs, embeddings will be read from the embeddings cache if previously embedded text is encountered, which will significantly speed up processing. (If you have previously run `processor` and on a subsequent run decide to change the `embeddings-model`, be sure to delete `data/embeddings_cache` to remove the incompatible previously-cached embeddings.)

The `wiktextract` raw data file will be automatically decompressed if the `--wiktextract-path` argument has the extension `.gz`, otherwise it will be read as a standard JSON Lines file. Similarly, if the `--serialization-path` argument ends in `.gz`, the output will be gzipped JSON; otherwise, it will be plain JSON.

If you get a CUDA out of memory error, or if you are using the CPU backend and the process gets killed due to RAM usage, try setting `--embeddings-batch-size` lower. The default value was set as the nearest round number that worked on a card with 10GB VRAM. Alternatively, if you have a better card, you could try setting the batch size higher to speed up the embeddings processing. 

## `server` usage
You must have run `processor` first, with the serialized processed data having been written to `data/wety.json.gz` or `data/wety.json` (the latter will load faster on server startup). Then:

```bash
cargo run --release --bin server
```

This will start the server at `127.0.0.1:3000`. The server will only accept requests from `127.0.0.1:8000`, the port used by [`wety-client`](https://github.com/jmviz/wety-client). See that repo for instructions on setting up and running the client locally.