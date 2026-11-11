# Building and serving the graph

## What an edge means

An edge says that one word descends from or is formed from another: inherited,
borrowed, derived, compounded, calqued. The graph holds derivation only. Lists of forms
that share something else are not descent and must not become edges, however
tree-like they look on Wiktionary: `{{CJKV}}` lists how a Chinese word's characters are
read in Japanese, Korean and Vietnamese, and a listed word may be a native word written
with the same characters, or a Japanese coinage that Chinese borrowed. Treat any
template that lists forms across languages without asserting derivation the same way
(`READING_LISTS` in `crates/processor/src/descendants.rs`).
Such correspondences are not shown elsewhere either, for now (decided 2026-11-08).

## Build

```sh
uv run --frozen wety-data run etymology_graph
```

This invokes the Rust processor with wety's entries of Wiktionary, writes to a run
staging directory, validates the graph with matching compiled language/POS tables,
then publishes an immutable artifact and `data/products/etymology_graph/CURRENT.json`.
The graph is one file, `wety.bin.gz`: `wety_core`'s `Data` in postcard behind a format
number, gzipped (`crates/core/src/processed.rs`); graphs written as JSON before it
still load. Tools that need it without `wety_core` get counts from its examples
(`lang_counts`, `graph_stats`).
It reads products of other workflows, pinned in its `workflow.json`:
`wiktionary_entries` (the entries, the dump's redirects and the entry names mentions
link to), `wiktionary_references` (each entry's References, from the pages dump of the
same date), the `<source>_claims` products (other sources' claims: the ACD, LiLa, the
MCD, TLOPO, the CSD, the Comparative Cariban Database, WOLD and IE-CoR, each matched
against `etymology_assertions`, the previous graph's assertion export), and the reviewers'
overrides in `data/workspaces/etymology/sources/wiktionary/etc/overrides.json`. For a
new dump, rebuild in that order: entries, References, the graph, its export, the
claims, the graph again ([packages/etymology](../packages/etymology/README.md)).

`uv run --frozen wety-data run wiktionary_entries` runs `crates/wiktionary-entries` on
one date's sources: the Wikimedia Enterprise snapshot of en.wiktionary
(`python -m source_tools.enterprise snapshot enwiktionary_namespace_0`, a free account;
only the current month can be downloaded), the Reconstruction pages' HTML at the
pages-articles dump's revisions (`python -m source_tools.wiktionary_html`, which
Enterprise does not serve), the pages-articles dump itself (redirects) and the language
export made from it. It takes about 15 minutes; the entries format is documented in
`crates/wiktionary-entries/src/model.rs`.

With the embedding cache warm the graph build takes about 2.5 minutes on an M1 Pro; a
cold cache for 10.9M entries took about six hours, almost all of it embedding. The
cache (`data/cache/embeddings/current/<namespace>/`) holds each embedding once, by the
hash of its text: the keys, read into memory, and the vectors (int8, 384 bytes each),
read in place from a memory-mapped file. Its namespace is a fingerprint of what the model computes: the
model, its revision and the exact embeddings it gives a few fixed texts. A change that
alters any embedding (candle, tokenizers, the device, the encoding or pooling in
`crates/processor/src/embeddings/model.rs`) starts an empty cache; one that does not,
such as most library upgrades, keeps it. Model
files are cached by Hugging Face; the model revision and expected local file hashes are
recorded in `data/registry/embedding-model.json`. On macOS the
embedding model runs on the GPU (Metal) automatically; elsewhere it runs on the CPU
unless the workflow command adds the `cuda` or `accelerate` feature (that change
becomes part of the run receipt). Texts are embedded in length-sorted sub-batches with
masked mean pooling, so an embedding does not depend on the other texts in its batch.

For direct development use, `cargo run --locked --release --bin processor -- --help`
shows explicit input/output/model options. Direct CLI runs do not create a workflow
receipt. Embedding revisions must be immutable commits. A lower embedding batch size
can reduce memory use.

## Serve

```sh
cargo run --locked --release --bin server
```

This serves the graph selected by `data/products/etymology_graph/CURRENT.json`; set
`WETY_GRAPH_PATH` to serve another graph file. A graph whose language/POS fingerprint
differs from the server binary's is rejected. The server listens on port 3000 (set
`WETY_PORT` to change it). See [client development](../client/README.md) and
[deployment](../deploy/README.md).

## Links

Shared links name a word by language code, term and, when Wiktionary numbers the
page's etymologies, that number: `/etymology/en/lime?ety=3`,
`/cognates/en/lime?ety=3&lang=la&lang=grc`. A cognates tree starts by default at the
oldest form that is the same word (inherited or borrowed), or at the root that form is
built on (Latin pater starts at PIE \*peh₂-); `root=ine-pro:ph₂tḗr` (with `rootEty`
when needed) names another of the word's ancestors to start from. Without `lang`,
trees show the reader's languages; `all=1` shows every language, and `within=1` adds
words formed from a word of the same language (limelight under lime). The client
resolves links with `/resolve?lang=en&term=lime&ety=3`, which also lists the word's
homographs. Numeric item and language IDs are positions in one build and never appear
in shared links; the client sends the graph version (`graph=`) with numeric item IDs,
and the server answers 409 if they belong to another graph. When a new Wiktionary
language export is selected, record the codes it retired so links naming them keep
working:

```sh
cargo run --locked -p wety-core --example retired_language_codes -- PREVIOUS/files/languages.json
```

This updates `data/registry/retired-language-codes.json`.
