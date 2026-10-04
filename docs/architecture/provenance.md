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

Small reference inputs, editorial records, product pointers, source manifests, locks, and run/acquisition receipts are
version controlled. Large original dumps, extracted deposits, generated generations and product mirrors,
research retrievals, run logs, and legacy archives are local and ignored by Git.
Back up `data/` as a unit before moving machines. A manifest does not provide remote
storage: for legacy inputs without a known immutable upstream location, a backup is
the only currently established recovery route. Do not delete local payloads on the
assumption that hashes make them downloadable.

To restore, place the exact bytes at manifest locations, then run
`uv run --frozen wety-data verify-all`, followed by catalog and dating workflows.
Archived generations are historical evidence, not current selections. Current pointer
files identify current generations; directory modification times never select them.
Historical absolute paths inside evidence are resolved through the exact checked
migration map without modifying claim payloads or their identities.

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
