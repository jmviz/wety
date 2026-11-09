# Building and serving the graph

## Build

```sh
uv run --frozen wety-data run etymology_graph
```

This invokes the Rust processor with the selected Wiktextract dump, writes to a run
staging directory, validates the graph with matching compiled language/POS tables,
then publishes an immutable artifact and `data/products/etymology_graph/CURRENT.json`.
Besides the dump it reads products of other workflows, pinned in its `workflow.json`:
`wiktionary_references` (each entry's References, from the pages dump of the same
date), `acd_claims` and `lila_claims` (other sources' claims, matched against
`etymology_assertions`, the previous graph's assertion export), and the reviewers'
overrides in `data/workspaces/etymology/sources/wiktionary/etc/overrides.json`. For a
new dump, rebuild in that order: References, the graph, its export, the claims, the
graph again ([packages/etymology](../packages/etymology/README.md)).

With the embedding cache warm it takes about 30 minutes on an M1 Pro; a cold cache for
the 2026-09-01 Kaikki data (10.9M entries) took about six hours, almost all of it
embedding. Model files are cached by Hugging Face; the model revision and expected
local file hashes are recorded in `data/registry/embedding-model.json`. On macOS the
embedding model runs on the GPU (Metal) automatically; elsewhere it runs on the CPU
unless the workflow command adds the `cuda`, `accelerate` or `mkl` feature (that change
becomes part of the run receipt). Texts are embedded in length-sorted sub-batches with
masked mean pooling, so an embedding does not depend on the other texts in its batch.

For direct development use, `cargo run --locked --release --bin processor -- --help`
shows explicit input/output/model options. Direct CLI runs do not create a workflow
receipt. Embedding revisions must be immutable commits; computed caches are separated
by model revision and embedding implementation. The old unkeyed cache is preserved but
is not reused automatically. A lower embedding batch size can reduce memory use.

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
