# Reorganization checklist

The machine-readable ledger is
[`steps.json`](../../data/registry/migrations/2026-09-reorganization/steps.json).
**All seven stages are complete.** The final run passed 123 Python tests, three Rust
core tests, all-target Rust compilation, three dating publication checks and the
graph workflow smoke test. All 1,975 original files are accounted for: 1,920 unchanged
at their new locations, and 55 with hash-verified originals preserved separately.

1. **Inventory and preservation.** Record pre-existing Git changes and SHA-256/size for
   all 1,975 inventoried files (25,476,626,488 bytes). Preserve earlier scientific
   generations and original implementations. No historical source payload is deleted.
2. **Physical layout.** Move Rust into `crates`, Python into `packages`, research into
   `data/workspaces`, versioned bundles into `data/artifacts`, selections/views into
   `data/products`, and superseded material into `data/archive`. Keep a complete
   old-to-new path map. Move the exploratory notebook into `experiments`.
3. **Consumers.** Update Cargo workspace members, compiled reference input selection,
   Python imports/configuration, tests, editor launch settings and server graph
   selection. Replace implicit Glottolog directory discovery with explicit selection.
   Resolve historical evidence paths without changing evidence payloads.
4. **Provenance.** Add adoption manifests, source locks/recipes, raw-response acquisition
   receipts, safe archive extraction, optional pinned Wiktionary extraction dependencies,
   and content-based catalog snapshot identity. Retain unknown historical provenance.
5. **Workflows.** Add a language-neutral local runner with explicit argv, input/code
   hashes, logs, exit statuses, staging, validation and atomic product selection.
   Integrate catalog and dating registries through their own validated transactions.
   Invoke the existing Rust graph processor as a workflow step.
6. **Verification.** Rebuild catalog/dating offline, compare scientific content with
   prior generations, verify file preservation and artifact manifests, run Python
   regressions and Rust checks, and execute a small real graph build/validation.
7. **Documentation.** Update root/domain READMEs, source recovery instructions and
   deployment guidance; distinguish historical audits from current instructions.

See [`verification.json`](../../data/registry/migrations/2026-09-reorganization/verification.json)
for the preservation/scientific comparison and
[`checks.json`](../../data/registry/migrations/2026-09-reorganization/checks.json)
for the final commands, outcomes and run receipts.

## Intentional limits

- The complete multi-gigabyte Wiktextract corpus was not reprocessed. The graph
  workflow was exercised with a small input using the actual pinned model and Rust
  processor; this does not benchmark full-corpus resource requirements.
- The legacy graph remains selected and requires `WETY_ALLOW_LEGACY_GRAPH=1` because
  its original numeric-reference identity cannot be verified retrospectively.
  New graphs enforce a compiled language/POS fingerprint.
- Original download/export receipts missing from legacy Wiktionary, IE-CoR,
  Wiktextract and graph inputs remain missing. Adoption records present hashes,
  not invented download dates. Preserve/back up the payloads.
- No fresh scientific source snapshot, scholarly adjudication, production deployment,
  or change to language-date conclusions was made as part of this cleanup.
- The client remains its own application. Language-neutral workflow definitions can
  accommodate future OCR, annotation, linked-data and graph work without moving
  domain algorithms out of their native languages.
