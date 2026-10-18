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

The client UX review (a shared doc outside git, "wety client UX review") is worked
through on this branch, after reverting codex's design additions (`b36265cf`):

- **Cognates** start from the oldest form that is the same word (*mother* shows the
  \*méh₂tēr family); a trail lets readers start elsewhere (`root=`).
- **Every view reads oldest at the top**, the etymology chart included.
- **Languages:** trees of up to 1,000 words show every language (*quiz*: 111 words in
  21, was 1); larger ones the reader's languages, with "show all" (`all=1`). One
  languages panel replaces three controls.
- **One header line, a title line, one control row**: the tree starts about a quarter
  of the way down (was nearly half); the toolbar and help rows are gone.
- **Quieter rows:** one ink, unmarked inheritance, "+N" branches, meanings on by
  default and in the chart, dates only where the language changes.
- **Word details** open beside the word; dates and evidence sit in a language view.
- **Search** folds diacritics, ʰ, subscripts and romanizations (paternitas, bher,
  Greek pater now match) and ranks headwords before inflected forms; language search
  matches any word in a name ("fren" finds Old French).
- One Tab stop per tree with arrow keys; dark mode; tab titles name the word.
- **Period labels** have no slashes ("from 9th or 10th century CE", "until c. 1000–500
  BCE"; Proto-Germanic "c. 500 BCE – 1 CE"): dating generation `b0bed444`, 51 labels
  reworded, no date changed; store snapshot `7054f5c2`.
- Checks pass: 13 core, 7 server and 254 dating tests, dating validation (0 errors,
  no rejected decisions), TypeScript and build; browser-checked on the full graph.

## Uncertainties

- The trail rule is measured, not reviewed word by word; "derived" steps can
  overreach (*chess*) or follow data errors (*robot*).
- `/cognates` and `/descendants` responses changed shape: deploy server and client
  together.
- With the browser pane hidden, a blank gap after load could not be checked.
- Research reviews remain model reviews; R2 pruning still needs a live dry run.

## Next three actions

1. Maintainer: review this branch at localhost:5198 (API 3918) before merging.
2. Merge Descendants into Cognates (start from the word itself), approved for later.
3. Inspect the R2 pruning dry run before the next server deployment.

## Decisions needed from the maintainer

- Whether `codex/client-ux-audit` is superseded by this branch.
