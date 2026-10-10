# Source acquisition, artifacts, and recovery

`data/registry/inputs.json` selects active input locations. `artifacts.json` inventories
adopted collections, `source-lock.json` retains known source versions and checksums,
and `source-recipes.json` contains executable acquisition/export recipes. Placeholder
parameters must be supplied from a chosen release; they are never guessed.

`uv run --frozen wety-sources COLLECTION HTTPS_URL --sha256 HASH --version VERSION`
downloads original bytes into `data/artifacts/COLLECTION/SHA256/files/source`.
A mismatched hash fails before publication. Each request gets a separate receipt
under `data/registry/acquisitions/`: requested/final URL, UTC request times, relevant
HTTP headers, file digest, version/licence declarations and acquirer hash. Credentials
are not recorded. An absent licence/version stays unknown. Repeated acquisitions do
not rewrite an existing artifact. `--extract-zip WORKSPACE` safely unpacks a retained
archive to an explicit working location.

For a new snapshot whose digest is not known yet, omit `--sha256`, inspect its receipt,
and lock the resulting hash. A successful HTTP response means the response is complete,
not that a corpus or query covers every relevant language. Inspect source-specific
coverage and scholarly scope separately.

## Existing source adapters

- ISO 639 and Wikidata: `uv run --frozen python -m source_tools.catalog_sources`.
  Flags can select ISO only, Wikidata only, or full statement details. Responses are
  preserved as artifacts; parsed snapshots stage under `data/workspaces/catalogs/sources`.
  An archive retains the previous snapshot; each successful refresh also adopts an immutable source-set artifact. Build and refresh share a writer lock.
  The registry rejects stale targeted Wikidata input hashes and mapping decisions.
- Glottolog: acquire a pinned release commit ZIP using the recipe; retain its archive
  and extracted CLDF together. Select the exact CLDF directory in `inputs.json`.
- Wiktionary: acquire a dated dump and its release metadata, then run
  `uv run --frozen --extra extraction python -m source_tools.language_data DUMP`. The exporter records dump, Lua, code,
  dependency and output hashes. It writes workspace outputs, never overwrites an
  adopted legacy artifact. The optional `extraction` dependency set pins `wiktextract` and `wikitextprocessor`
  Git revisions in the root lock. The original legacy exporter environment remains
  unknown; these dependencies support future exports, not a retroactive receipt.
  With the Kaikki dump of the same date, `python -m source_tools.entry_names DUMP
  WIKTEXTRACT LANGUAGES` exports the page titles (entry names) of the terms the dump
  cites, by the dump's own `Language:stripDiacritics`; the graph processor uses them to
  find entries cited with diacritics their titles lack.
- IE-CoR: acquire the exact publication/deposit table, preserving its source IDs and
  analysis identity. `python -m catalogs.iecor` generates review proposals from the
  explicitly selected table. The old table's upstream revision remains unresolved.
- Kaikki: retain the dump plus the `rawdata.html` extraction metadata, including dump
  date and extractor commits. A moving URL is an acquisition route, not an archive.
- Dating retrieval: original research material formerly called `dating/cache` is
  durable under `data/workspaces/dating/retrieved`. New dataset requests preserve
  response bodies through the shared acquirer; GitHub file retrieval pins one commit,
  records its tree separately and rejects truncated trees. Bibliography, retrieval
  coverage, mechanical reproduction and expert review remain separate records.

The embedding-model recipe likewise downloads each file at the pinned commit with
its expected SHA-256. Restore the verified bytes to
`$HF_HOME/hub/models--sentence-transformers--all-MiniLM-L12-v2/snapshots/REVISION/FILENAME`
(`HF_HOME` defaults to `~/.cache/huggingface`). The workflow verifies these files
before processing; source acquisition remains separate from computation.

## Storage and recovery

Three layers, each with one job.

1. **Git (public):** code, schemas, prompts and docs; what the server and website are
   built from (`inputs.json`, the Wiktionary export, `language_periods.json`,
   `deploy/graph.json`); small references code reads (build configuration, locks,
   product pointers, the relocation map, curated catalog inputs); the snapshot pointer
   `data/SNAPSHOT.json`; and human-readable research records (briefs, run READMEs,
   family pages, STATUS). The exact list is `.gitignore`.
2. **The research store (private):** everything else under `data/`: retained texts,
   artifacts and receipts, evidence, the editorial ledger, runs, packets, generations.
   It is backed up as restic snapshots (deduplicated, encrypted) by `wety-data store`
   (`packages/workflow_tools/store.py`); a commit names the data it was made with in
   `data/SNAPSHOT.json`.
3. **Published products (public):** built from what wety shows, never copied from the
   store; `language_periods.json` is one.

Research outputs never enter git: they are large and grow with every run.

### The editorial ledger

Decisions, withdrawals, registered sources and errata live in
`data/workspaces/dating/_ledger/` as immutable batch files: every write adds one file,
nothing is edited (`harness/ledger.py`). An event names the heads it replaces, so
checkouts merge by copying files; two sessions that write one key without seeing each
other leave a conflict, which the registry withholds until a new event replaces both.
`dating ledger status` shows the state; `dating ledger erratum` records a correction to
an imported packet (claims are content-addressed and never edited).

### Setting up the store

Once per machine: `brew install restic`; create a private bucket (for example
Cloudflare R2 `wety-research`) and an API token scoped to it; then

    export RESTIC_REPOSITORY=s3:https://<account-id>.r2.cloudflarestorage.com/wety-research
    export AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=...
    export RESTIC_PASSWORD_FILE=~/.config/wety/restic-password   # keep a copy elsewhere
    restic init

`rclone:<remote>:<bucket>` works too (rclone is already used to publish the graph).
Without the password the snapshots cannot be read: store it outside the machine.

### Working with it

- Snapshot at checkpoints, from the main checkout's `data/` (worktrees link parts of
  it): `uv run --frozen wety-data store snapshot --note "..."`, then commit
  `data/SNAPSHOT.json`.
- A worktree's new records reach the main checkout with
  `wety-data store sync WORKTREE/data MAIN/data --apply`: missing files are cloned,
  nothing is overwritten, and a file that differs is reported as a conflict (append-only
  areas should never conflict; derived products are regenerated with `dating synthesize`).
- New machine or lost data: `wety-data store restore --target DIR` (into an empty
  directory), then `sync DIR data --apply`, then `uv run --frozen wety-data verify-all`
  and the catalog and dating workflows.
- `check` confirms that the pointer's snapshot exists in the configured repository.

Archived generations are historical evidence, not current selections. Current pointer
files identify current generations; directory modification times never select them.
Historical absolute paths inside evidence are resolved through the exact checked
migration map without modifying claim payloads or their identities. Do not delete local
payloads on the assumption that hashes make them downloadable.

### Leaving research data out of published history

Research data was tracked until 2026-10-09. Two pushed branches already carry some of
it (`dating-pilot`, `reorg-baseline`: the 2026-10-03 pilot's packets, evidence and
claims); deleting them on GitHub is the maintainer's call, and pushed content may have
been cached. Before the first push of the rest, remove it from the unpushed commits only,
so the commits already on GitHub keep their hashes (rewriting them would also drop
GitHub's merge signatures). In the main checkout, with every worktree's branch either
merged or listed, and `data/` backed up:

    uvx git-filter-repo --force --invert-paths \
      --paths-from-file docs/architecture/history-filter.txt \
      --refs origin/main..main origin/main..BRANCH ...

Tested on a clone (2026-10-09): `origin/main` stays an ancestor (a plain push applies)
and no research path remains in the rewritten commits. Afterwards run
`wety-data store sync BACKUP/data data --apply` to put back any file the rewrite
removed from the working tree.

`data/cache` contains disposable computation/environment caches only. The legacy
virtual environments were preserved there for inspection, not advertised as relocatable
installations. The root `.venv` is recreated from the root lock. Publication can be
reproduced from retained input artifacts; live upstream services and model extraction
are not assumed deterministic or historically replayable.

## Publishing a new local source or editorial set

Use `wety-data adopt COLLECTION REPO_RELATIVE_DIRECTORY --role source|transformed|editorial
--provenance REPO_RELATIVE_JSON` (one shell command) to preserve a local file set as an
immutable artifact. The provenance JSON should name upstream identity or input/run
references and disclose unknowns. Original workspace files are retained. Review the
result, update the relevant explicit selection in `inputs.json`, and rebuild consumers.
Changed source hashes invalidate existing scoped decisions; adoption is not approval.

For the optional exporter, install with `uv sync --frozen --extra extraction`. Both
upstream Git revisions and transitive dependencies are locked. Its receipt includes
Python version, dependency origins, root lock hash and UTC execution times. A fixture
smoke check established Lua invocation/JSON decoding with this environment; a full
Wiktionary re-export was deliberately not substituted for the retained legacy inputs.
