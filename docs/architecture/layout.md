# Repository layout

| Location | Responsibility |
| --- | --- |
| `crates/{core,processor,server}` | Rust libraries, graph transformation and service |
| `packages/catalogs` | Catalog assertions, cross-reference builder and IE-CoR proposals |
| `packages/dating` | Dating harness, schemas, benchmarks and domain documentation |
| `packages/source_tools` | Download and export implementations |
| `packages/workflow_tools` | Paths, receipts, artifact verification, local execution and the research store |
| `workflows/` | Stack-neutral executable workflow definitions |
| `deploy/` | Server deployment: host setup, release switching, graph selection |
| `data/registry` | Selected inputs, artifact inventory, acquisition recipes and migration records |
| `data/artifacts` | Preserved source snapshots and versioned generated bundles |
| `data/workspaces` | Mutable research, editorial decisions, retrieval records and annotations |
| `data/products` | Current product pointers and convenient validated views |
| `data/runs` | Execution receipts and logs |
| `data/cache` | Disposable computation caches; never the only copy of evidence |
| `data/archive` | Superseded generations, uncertain backups and historical material |
| `experiments` | Exploratory code/notebooks, separate from production entry points |

Under `data/`, git tracks only what the app is built from, small references code
reads, the snapshot pointer and human-readable research records (`.gitignore`); the
rest is the private research store ([provenance](provenance.md)).

A workflow is a process contract, independent of its implementation language. The Rust
processor remains one graph transformation; Python runs it through its CLI. Domain
algorithms are not copied into orchestration code. Extraction, annotation, OCR and
future graph builds can use the same artifact/input/run contract.

Artifact identity is content-based; a product is a selection, not an artifact identity.
Source, transformed and editorial are roles recorded in manifests, not competing
folder trees. Research workspaces are intentionally mutable. A successful run does
not grant scholarly approval. Catalog correspondence still does not permit date
transfer, and unknown language chronology remains unknown.

Legacy provenance must not be invented: adoption records a hash and present location,
not a historical download date or an unverified upstream version. Existing receipts
and source labels are retained. Historical record payloads and archived generations
retain their original paths; the relocation map of the 2026-09 reorganization
(`data/registry/migrations/2026-09-reorganization/`) resolves those references.

Catalog source identity hashes named input-file hashes, excluding physical paths, so
relocations do not invalidate mappings. Generated bundle IDs bind content and selected
provenance; the code is recorded in the workflow run receipt.
