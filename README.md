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
It can take hours on a full dump. Model files are cached by Hugging Face; the model
revision and expected local file hashes are recorded in `data/registry/embedding-model.json`.
The workflow command specifies CPU defaults. Rust features `metal`, `cuda`,
`accelerate` and `mkl` remain available by explicitly changing the workflow command;
that change becomes part of the run receipt.

For direct development use, `cargo run --locked --release --bin processor -- --help`
shows explicit input/output/model options. Direct CLI runs do not create a workflow
receipt. Embedding revisions must be immutable commits; computed caches are separated
by model revision and embedding implementation. The old unkeyed cache is preserved
but is not reused automatically. A lower embedding batch size can reduce memory use.

## Serve a graph

```sh
WETY_GRAPH_PATH=data/artifacts/etymology-graph/VERSION/files/wety.json.gz \
  cargo run --locked --release --bin server
```

Use the artifact selected by `data/products/etymology_graph/CURRENT.json` in place of
`VERSION`. New graphs are rejected if their language/POS fingerprint differs from
the server binary. The preserved legacy graph is the default local path but has no
verified reference fingerprint. To intentionally use that older graph:

```sh
WETY_ALLOW_LEGACY_GRAPH=1 cargo run --locked --release --bin server
```

This accepts unverified numeric IDs; it does not validate or rewrite that graph.
The server listens on port 3000. See [client development](client/README.md) and
[server deployment](crates/server/deployment.md).
