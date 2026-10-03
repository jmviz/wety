# Repository organization

Implementation is tracked in `data/registry/migrations/2026-09-reorganization/steps.json`.

| Location | Responsibility |
| --- | --- |
| `crates/{core,processor,server}` | Rust libraries, graph transformation and service |
| `packages/catalogs` | Catalog assertions, cross-reference builder and IE-CoR proposals |
| `packages/dating` | Dating harness, schemas, benchmarks and domain documentation |
| `packages/source_tools` | Download and export implementations |
| `packages/workflow_tools` | Paths, receipts, artifact verification and local execution |
| `workflows/` | Stack-neutral executable workflow definitions |
| `data/registry` | Selected inputs, artifact inventory, acquisition recipes and migration records |
| `data/artifacts` | Preserved source snapshots and versioned generated bundles |
| `data/workspaces` | Mutable research, editorial decisions, retrieval records and annotations |
| `data/products` | Current product pointers and convenient validated views |
| `data/runs` | Execution receipts and logs |
| `data/cache` | Disposable computation caches; never the only copy of evidence |
| `data/archive` | Superseded generations, uncertain backups and historical material |
| `experiments` | Exploratory code/notebooks, separate from production entry points |

A workflow is a process contract, independent of its implementation language. The Rust
processor remains one graph transformation; Python runs it through its CLI. Domain
algorithms are not copied into orchestration code. Extraction, annotation, OCR,
linked-data exports and future graph builds can use the same artifact/input/run
contract without another top-level reorganization.

Artifact identity is content-based; a product is a selection, not an artifact identity.
Source, transformed and editorial are roles recorded in manifests, not competing
folder trees. Research workspaces are intentionally mutable. A successful run does
not grant scholarly approval. Catalog correspondence still does not permit date
transfer, and unknown language chronology remains unknown.

Legacy provenance must not be invented: adoption records a hash and present location,
not a historical download date or an unverified upstream version. Existing receipts
and source labels are retained. Historical record payloads and archived generations
retain their original paths; a checked relocation map resolves those references.

Catalog source identity now hashes named input-file hashes, excluding physical paths.
The initial identity migration has an explicit receipt; source hashes were verified
unchanged before retaining the existing bounded decision. New relocations therefore
do not invalidate mappings solely because a directory changed. Generated bundle IDs
still bind actual implementation code and selected provenance.
