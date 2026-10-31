# Status

*Updated 2026-10-31. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…`, the 2026-09-01 dump; 8,101,979 etymological entries and
  9,946,921 edges. Dates cover 348 codes, including 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs stay in `data/`, backed up
  with `wety-data store`; [provenance](docs/architecture/provenance.md) covers recovery.

## What changed

- **Etymology claims model adopted** (shared doc "wety etymology claims model",
  outside git). wety will build trees from attributed claims: alternatives, certainty,
  credits between sources, automatic synthesis. The doc's work plan is the tracker:
  14 milestones in four tracks (core, ACD, LiLa, synthesis), one branch each
  (`claude/claims-<id>`), commits titled `Claims <id>: …`.
- **core-1 (code done, check pending):** every graph edge records its basis: `etymon`,
  `template`, `chain`, `mention`, `form_of`, `descendants`, `root`, `root_vote`; the
  last five are wety's rules. No walk changes. `graph-edges` dumps a graph's edges as
  TSV for diffing builds. The graph format changed: old graph files no longer load.
- **Earlier on this stack:** the client UX review is worked through; the root fixes
  (testum's \*per-, two-sense roots) are in the processor, not yet in a graph.
- Checks pass: 43 processor, 15 core and 7 server tests; clippy clean.

## Uncertainties

- `/cognates` and `/descendants` changed shape on the client branch: deploy server and
  client together, with a rebuilt graph.
- How many imputed roots the root fixes change is unknown until the rebuild.
- Research reviews remain model reviews; R2 pruning still needs a live dry run.

## Next three actions

1. Rebuild the graph (`wety-data run etymology_graph`, about 6 h) for core-1 and the root
   fixes at once; diff its `graph-edges` dump against `dab06f67`'s: only `root` and
   `root_vote` edges may differ. Count edges by basis.
2. acd-1 and lila-1: retain ACD v2.0 and LiLa's de Vaan data with their curation files.
3. core-2: the assertion schema, drafted against Wiktionary, ACD and LiLa samples.

## Decisions needed from the maintainer

- The doc proposes a new Python package, `packages/etymology`, for the schema and
  converters (a layout change); it stands unless the maintainer objects.
- Whether to ship the client work before the claims work lands.
