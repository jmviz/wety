# wety
Digest etymologies from [`wiktextract`](https://github.com/tatuylonen/wiktextract) data. This is a work-in-progress project to which I make indiscriminate commits. Current `main` may not work as described below, or at all.

## Usage
If you do not have rust installed, [install it](https://www.rust-lang.org/tools/install). Clone this repo and `cd` into it. Download the latest `wiktextract` raw data from [https://kaikki.org/dictionary/rawdata.html](https://kaikki.org/dictionary/rawdata.html), namely [this file](https://kaikki.org/dictionary/raw-wiktextract-data.json.gz), into `/data`. (To get the data including wiktionary `Descendants` sections, see below.) This uses `rust-bert`, which uses `pytorch`, for word sense disambiguations. So follow the [`rust-bert` installation instructions](https://github.com/guillaume-be/rust-bert#manual-installation-recommended) if you have a CUDA-enabled GPU you want to use instead of the default CPU backend. Finally, Run the rust program:

```bash
cargo run --release
```

It will take a while to compile, and much longer to run :). By default, it will process the raw wiktextract data and creat three outputs: a Turtle file; an Oxigraph store; a gz-compressed JSON serialization of the finalized internal data structure used by `wety` (for loading into the server binary (TBD) on its startup). See `cargo run --release -- --help` for all options. Note that the first time this is run, the relevant `rust-bert` files will be downloaded from Hugging Face and placed in `~/.cache/.rustbert`. On subsequent runs, the files will be read from this cache rather than redownloaded.

The `wiktextract` raw data file will be automatically decompressed if the `--wiktextract-path` argument has the extension `.gz`, otherwise it will be read as a standard JSON Lines file. Similarly, if the `--serialization-path` argument ends in `.gz`, the output will be gzipped JSON; otherwise, it will be plain JSON.

If you get a CUDA out of memory error, or if you are using the CPU backend and the process gets killed due to RAM usage, try setting `--embeddings-batch-size` lower. The default value was set as the nearest round number that worked on a card with 10GB VRAM. Alternatively, if you have a better card, you could try setting the batch size higher to speed up the embeddings processing.

## Using local `wiktextract` data
You can run my [fork](https://github.com/jmviz/wiktextract/tree/descendants) of `wiktextract` to get data for wiktionary `Descendants` sections which can then be used with `wety`. Here is the invocation I use to run it (refer to the `wiktextract` README for explanations of the arguments):

```bash
python wiktwords --num-threads 16 --all-languages --descendants --etymologies --redirects --cache ../wiktextract_data/cache --pages-dir ../wiktextract_data/pages --out ../wiktextract_data/data.json ../wiktextract_data/enwiktionary-20230201-pages-articles.xml.bz2
```

You will have to adjust `num-threads` to fit your machine's CPU and RAM (each thread requires about 4 GB). `wiktextract` takes a very long time to run. The above command takes about 1.5 days to run on my machine. The time scales about linearly with the number of threads. 

To use the resulting `data.json` with `wety`, specify the appropiate path to the file:

```bash
cargo run --release -- --wiktextract-path path/to/data.json
```