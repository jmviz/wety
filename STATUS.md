# Status

*Updated 2026-10-30. Client work and a processor fix are on `claude/root-vote-fix`, on top
of `codex/client-ux-audit` (the client work); not deployed.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…`, the 2026-09-01 dump; 8,101,979 etymological entries with
  stable word URLs and homographs. Dates cover 348 codes, including 135 estimated
  proto-language ends.
- **Research store:** private evidence, artifacts and runs stay in `data/`, backed up
  with `wety-data store`; [provenance](docs/architecture/provenance.md) covers recovery.

## What changed

The client UX review (a shared doc outside git) is worked through, after reverting
codex's design additions (`b36265cf`):

- **Cognates** start from the oldest form that is the same word, or its root
  (*mother*: \*méh₂tēr); a trail lets readers start elsewhere (`root=`).
- **Every view reads oldest at the top**, the etymology chart included.
- **Languages:** the reader's languages by default, "all languages" (`all=1`) a choice;
  same-language formations only on request (`within=1`). One languages panel.
- **One header line, a title line, one control row**; quieter rows; word details beside
  the word; dates and evidence in a language view; dark mode; one Tab stop per tree.
- **Search** folds diacritics, ʰ, subscripts and romanizations and ranks headwords first.
- **Period labels** have no slashes: dating generation `b0bed444`, no date changed.
- **Root votes (processor, not yet in a graph):** a word with no etymology no longer
  takes a root from its derivatives' prefixes. Latin *testum* got PIE \*per- "to go
  through" (from Polish *przetestować*'s prze-) over English *test*'s \*ters-, on a
  lowest-id tie, and every *test* tree showed it. A root reached anywhere in a voter's
  etymology, as any homograph, no longer votes; tied votes impute nothing.
- **Rust tooling:** the toolchain now installs clippy and rustfmt; the workspace is
  clippy-clean and rustfmt-formatted, except `processor/src/embeddings.rs`, whose
  text keys the embedding cache (any edit discards it). Trees take a `TreeView`.
- Checks pass: 41 processor (2 new), 15 core, 7 server and 254 dating tests, dating
  validation, TypeScript and build.

## Uncertainties

- The trail rule is measured, not reviewed word by word; "derived" steps can
  overreach (*chess*) or follow data errors (*robot*).
- `/cognates` and `/descendants` responses changed shape: deploy server and client
  together.
- The root-vote change alters other imputed roots; how many is unknown until a rebuild.
- Research reviews remain model reviews; R2 pruning still needs a live dry run.

## Next three actions

1. Rebuild the graph (`wety-data run etymology_graph`, about 6 h), compare its imputed
   roots with `dab06f67`, then publish it with the client work.
2. Maintainer: review the branch.
3. Inspect the R2 pruning dry run before the next server deployment.

## Decisions needed from the maintainer

- Whether to rebuild and ship the graph with the client work, or ship the client first.
