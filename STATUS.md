# Status

*Updated 2026-10-17. Client work is on `claude/client-ux-audit-findings-c7d2b1`; not deployed.*

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

- The client UX review lives in a shared doc outside git ("wety client UX review"),
  with recommendations as tickable items and the cognates proposal below.
- Kept codex's bug fixes (`b4f9a4ef`): error states, stale search responses, focus,
  mobile zoom, popover overflow.
- Reverted the client and server design additions of `b36265cf`: the Base lineage /
  All components scope, `/explore`, language suggestion cards, tree reading guide,
  filter presets and homograph selector. Base lineage left *mother* and *democracy*
  with no cognates. Four bug fixes from that commit were kept: stale popovers,
  popover placement on resize, stale language-search responses, and re-scrolling
  on every filter change.
- Checks pass: workspace compilation, 8 core tests, TypeScript and production build.

## Uncertainties

- The proposed cognate rule is measured on 38 words and all English entries, not
  reviewed word by word; "derived" steps sometimes overreach (*chess*) or follow
  data errors (*robot*).
- The local client on port 8000 still serves the main checkout's build of `b36265cf`.
- Research reviews remain model reviews; R2 pruning still needs a live dry run.

## Next three actions

1. Implement proposal A1 (cognates from a chosen ancestor) once its rule is approved.
2. Then the one-row controls and quieter rows (review items B and C).
3. Inspect the R2 pruning dry run before the next server deployment.

## Decisions needed from the maintainer

- A1: follow "derived" steps; stop at calques; root guard only on "derived" steps;
  later merge Descendants into the same view.
- Whether `codex/client-ux-audit` is superseded by this branch.
