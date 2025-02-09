# [wety](https://www.wety.org/)

(w)iktionary (ety)mologies. `processor` uses [`wiktextract`](https://github.com/tatuylonen/wiktextract) data to generate an etymological graph of all words on wiktionary. `server` serves the data. `client` is the frontend.

## Installation

Clone this repo.

If you do not have Rust installed, [install it](https://www.rust-lang.org/tools/install). This project uses `nightly` Rust and assumes you have a recent version. When you build the project, the `nightly` channel should automatically be selected. If compilation fails complaining about unstable features, you should update to a more recent version with `rustup update nightly`.

## `processor` usage

Make sure you are in the root directory of this repo. Download the latest `wiktextract` raw data from [https://kaikki.org/dictionary/rawdata.html](https://kaikki.org/dictionary/rawdata.html), namely [this file](https://kaikki.org/dictionary/raw-wiktextract-data.jsonl.gz), into `data/`. Run `processor` with:

```bash
cargo run --release --bin processor
```

It will take a while to compile, and even longer to run :). By default, it will process the raw wiktextract data and produce a gz-compressed JSON serialization of the data structure used by `server`. It also can generate a [Turtle](https://www.w3.org/TR/turtle/) file for loading into a graph database, e.g. [Oxigraph](https://github.com/oxigraph/oxigraph). See `cargo run --release --bin processor -- --help` for all options.

`processor` uses an embeddings model for word sense disambiguation. Note that the first time this is run, the model files will be downloaded from Hugging Face and placed in `~/.cache/huggingface/hub`. On subsequent runs, the files will be read from this cache rather than redownloaded. Similarly, on the first run, embeddings will be generated for all items determined to need them. This will take the lion's share of processing time. On subsequent runs, embeddings will be read from the embeddings cache if previously embedded text is encountered, which will very significantly speed up processing. Depending on the beefiness of your machine and whether you are using GPU or CPU (much slower) for embeddings, an initial run generating all new embeddings may take anywhere from less than 10 minutes to more than 10 hours. Subsequent runs using cached embeddings should take about 1%-10% of that time. The CPU will be used by default. To utilize your GPU, run with `--features cuda` if you have a CUDA GPU or `--features metal` on an ARM-based Mac. For accelerated CPU processing, run with `--features mkl` or `--features accelerate` on macos.

The `wiktextract` raw data file will be automatically decompressed if the `--wiktextract-path` argument has the extension `.gz`, otherwise it will be read as a standard JSON Lines file. Similarly, if the `--serialization-path` argument ends in `.gz`, the output will be gzipped JSON; otherwise, it will be plain JSON.

If you get a CUDA out of memory error, or if you are using CPU and the process gets killed due to RAM usage, try setting `--embeddings-batch-size` lower. The default value was set as the nearest round number that worked on a card with 10GB VRAM. Alternatively, if you have a better card, you could try setting the batch size higher to speed up the embeddings processing.

**N.B.** If you have previously run `processor` and on a subsequent run decide to change the `embeddings-model`, be sure to delete `data/embeddings_cache` to remove the incompatible previously-cached embeddings.

## `server` usage

You must have run `processor` first, with the serialized processed data having been written to `data/wety.json.gz` or `data/wety.json` (the latter will load faster on server startup).

Alternatively, if you don't want to spend time running `processor` yourself, you can download the current processed data that [wety.org](https://www.wety.org) is using from [data.wety.org](http://data.wety.org/). Download the file into `data/`, and decompress it if you wish (do not rename it in either case). It's possible that the format of the processed data at this link may become out of sync with the format expected by latest `main`, either because production is using an older version or because I have neglected to update the link. If you get a deserialization error running the below, please [ping me](mailto:jmviz@jmviz.dev) to update the link.

To run the server:

```bash
cargo run --release --bin server
```

Requests to the server can be made at `127.0.0.1:3000`. For development in conjuction with the frontend, see the README in the `client` subdirectory for instructions on setting up and running the client locally.
