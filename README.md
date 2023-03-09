# wety
Digest etymologies from [wiktextract](https://github.com/tatuylonen/wiktextract) data. This is a work-in-progress project to which I make indiscriminate commits. Current `main` may not run as described below, or may fail. Sometimes it may not even compile!

## Usage
If you do not have rust installed, [install it](https://www.rust-lang.org/tools/install). Clone this repo and `cd` into it. Download the latest wiktextract raw data from [https://kaikki.org/dictionary/rawdata.html](https://kaikki.org/dictionary/rawdata.html), namely [this file](https://kaikki.org/dictionary/raw-wiktextract-data.json.gz), into `/data`. (To get the data including wiktionary `Descendants` sections, see below.) Run the rust program:

```bash
cargo run --release
```

This will parse the raw data, process etymologies, and output a Turtle file and Oxigraph store. 

## Using local `wiktextract` data
You can run my [fork](https://github.com/jmviz/wiktextract/tree/descendants) of wiktextract to get data for wiktionary `Descendants` sections which can then be used with `wety`. Here is the invocation I use to run it (refer to the `wiktextract` README for explanations of the arguments):

```bash
python wiktwords --num-threads 16 --all-languages --descendants --etymologies --redirects --cache ../wiktextract_data/cache --pages-dir ../wiktextract_data/pages --out ../wiktextract_data/data.json ../wiktextract_data/enwiktionary-20230201-pages-articles.xml.bz2
```

You will have to adjust `num-threads` to fit your machine's CPU and RAM (each thread requires about 4 GB). `wiktextract` takes a very long time to run. The above command takes about 1.5 days to run on my machine. The time scales about linearly with the number of threads. 

To use the resulting `data.json` with `wety`, specify the appropiate path to the file:

```bash
cargo run --release -- --wiktextract-path path/to/data.json
```