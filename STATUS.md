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
- **Cognates start from an ancestor on the word's trail** (review item A1, approved).
  The trail runs from the word back through each older form that is the same word
  (inherited, borrowed, a form, "derived from"; never a "derived" step onto a bare
  root). The tree is the last step's descendants: *mother* now shows the *méh₂tēr
  family (182 words with English selected) and *democracy* reaches Greek
  δημοκρατία. Readers can start from any step or source (`root=` in links); long
  trails collapse their middle. Same-word branches open first in the initial view.
- Checks pass: workspace compilation, 11 core tests (3 new for the trail rule), 7
  server tests, TypeScript and production build. Checked in the browser against the
  full graph: mother, father, democracy, sugar, quiz, a chosen root, language
  changes, Descendants, phone width.

## Uncertainties

- The trail rule is measured on 38 words and all English entries, not reviewed word
  by word; "derived" steps sometimes overreach (*chess*) or follow data errors (*robot*).
- The `/cognates` response shape changed; deploy server and client together (old
  open tabs break until reloaded).
- Port 8000 still serves the main checkout's build of `b36265cf`; this branch runs on
  5198 (client) and 3918 (API) while those preview processes remain.
- Research reviews remain model reviews; R2 pruning still needs a live dry run.

## Next three actions

1. Maintainer: review this branch in the browser before merging.
2. Review items A2–A4 and B (one direction, one language control, one control row).
3. Inspect the R2 pruning dry run before the next server deployment.

## Decisions needed from the maintainer

- When to merge Descendants into Cognates (start from the word itself), approved
  for later as its own change.
- Whether `codex/client-ux-audit` is superseded by this branch.
