# [wety](https://www.wety.org/)

Wiktionary etymologies, language catalog linking, and a provenance-rich registry of
chronological claims. Rust builds and serves the etymological graph; Python manages
catalog and dating research. Workflows can invoke either stack through explicit CLIs.

## Layout

- `crates/{core,processor,server}`: Rust libraries, graph processor, API server.
- `packages/{catalogs,dating,source_tools,workflow_tools}`: Python domain code and shared tooling.
- `workflows/`: executable workflow definitions; algorithms stay in their packages/crates.
- `data/`: one home for artifacts, research workspaces, product selections, receipts, caches and archives.
- `client/`: frontend. `experiments/`: exploratory notebooks.

**Current state and next steps: [STATUS.md](STATUS.md).** Rules for agents:
[AGENTS.md](AGENTS.md). See also [organization and ownership](docs/architecture/reorganization.md)
and [source acquisition and recovery](docs/architecture/provenance.md).
The scientific contracts remain in [catalogs](packages/catalogs/README.md) and
[dating](packages/dating/README.md). Catalog correspondence never licenses automatic
chronological transfer.

## Setup and offline checks

Run from the repository root. Python 3.12+ and `uv` are required. The root `uv.lock`
is authoritative; Rust uses `Cargo.lock` and the dated `rust-toolchain.toml`.

```sh
uv sync --frozen
uv run --frozen wety-data run catalogs
uv run --frozen wety-data run dating
uv run --frozen python -m unittest discover -s packages/catalogs/tests
uv run --frozen python -m unittest discover -s packages/dating/tests
uv run --frozen python -m unittest discover -s packages/workflow_tools/tests
uv run --frozen python -m unittest discover -s packages/source_tools/tests
cargo check --locked --workspace
cargo test --locked -p wety-core --lib
```

Catalog/dating builds and tests use existing local source material and make no model
calls. A fresh checkout needs the ignored source artifacts restored first; manifests
and the migration inventory identify the required bytes. Run `wety-data verify-all`
to check adopted artifacts. Source refresh is always a separate, deliberate command.

## Build an etymological graph

```sh
uv run --frozen wety-data run etymology_graph
```

This invokes the Rust processor with the selected Wiktextract dump, writes to a run
staging directory, validates the graph with matching compiled language/POS tables,
then publishes an immutable artifact and `data/products/etymology_graph/CURRENT.json`.
It takes hours on a full dump: about six on an M1 Pro for the 2026-09-01 Kaikki data
(10.9M entries), almost all of it embedding. Model files are cached by Hugging Face; the
model revision and expected local file hashes are recorded in
`data/registry/embedding-model.json`. On macOS the embedding model runs on the GPU
(Metal) automatically; elsewhere it runs on the CPU unless the workflow command adds the
`cuda`, `accelerate` or `mkl` feature (that change becomes part of the run receipt).
Texts are embedded in length-sorted sub-batches with masked mean pooling, so an
embedding does not depend on the other texts in its batch.

For direct development use, `cargo run --locked --release --bin processor -- --help`
shows explicit input/output/model options. Direct CLI runs do not create a workflow
receipt. Embedding revisions must be immutable commits; computed caches are separated
by model revision and embedding implementation. The old unkeyed cache is preserved
but is not reused automatically. A lower embedding batch size can reduce memory use.

## Serve a graph

```sh
cargo run --locked --release --bin server
```

This serves the graph selected by `data/products/etymology_graph/CURRENT.json`; set
`WETY_GRAPH_PATH` to serve another graph file. A graph whose language/POS fingerprint
differs from the server binary's is rejected. The server listens on port 3000 (set
`WETY_PORT` to change it). See [client development](client/README.md) and
[deployment](deploy/README.md).

## Links

Shared links name a word by language code, term and, when Wiktionary numbers the
page's etymologies, that number: `/etymology/en/lime?ety=3`,
`/cognates/en/lime?ety=3&lang=la&lang=grc`. The client resolves them with
`/resolve?lang=en&term=lime&ety=3`, which also lists the word's homographs. Numeric
item and language IDs are positions in one build and never appear in shared links;
the client sends the graph version (`graph=`) with numeric item IDs, and the server
answers 409 if they belong to another graph. When a new Wiktionary language export
is selected, record the codes it retired so links naming them keep working:

```sh
cargo run --locked -p wety-core --example retired_language_codes -- PREVIOUS/files/languages.json
```

This updates `data/registry/retired-language-codes.json`.
