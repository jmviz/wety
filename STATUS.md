# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2` (acd-1, lila-1, core-2, acd-2, lila-2), then
`claude/claims-core-3`, then `claude/claims-core-4` (core-5 merged in), then
`claude/claims-core-6`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump. The stack adopts `3da0f0c9…`:
  10.1 million words and 0.8 million cited-only ones, 9.95 million links, 6,003
  languages, with each entry's References; it needs the stack's server and client.
  Dates cover 348 codes, 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **core-6 (Wiktionary's part):** the etymology chart draws a word's alternatives as
  separate "or" branches (all when none is preferred, else behind "or N more"), dashes
  possible and doubtful links with a "?", underlines modes a wety rule inferred and shows
  "origin unknown". Word details list each origin with who asserts it (Wiktionary's
  editors and on which page, or the wety rule) and the works it follows, from a new
  `/origin/:id`. Descendants rows no longer name another alternative's parent as a
  co-parent. Checked in the browser on *doubt*, *elk*, *lupus*, *cats* and *douten*.
- **References in the graph** (core-4): a `wiktionary_references` workflow product the
  graph build reads; 562,108 entry credits on 349,671 assertions. The superseded join's
  outputs are deleted from the store.

## Uncertainties

- core-6 is not finished: "disputed", "whose view", the `view`/`floor` tree parameters,
  "implied by the ACD's tree" and "possibly related" in Cognates need a second source
  (acd-3, lila-3) or were not built.
- *lupus* keeps its Descendants reading: its own section names a language but no word
  ("possibly borrowed from an Osco-Umbrian language"), which wety cannot represent yet.
- Inline citations and the place of a footnote within a section need the Wiktionary HTML
  dump parsed directly, a separate project the maintainer envisions.
- `/cognates` and `/descendants` changed shape on the client branch: ship server, client
  and graph together.

## Next three actions

1. acd-3 and lila-3: match ACD and LiLa records to items.
2. Finish core-6 once a second source is in: disputed, whose view, implied links.
3. syn-2: model extraction of prose citations and conjectures.

## Decisions needed from the maintainer

- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
- Whether to ship the client work before the claims work lands.
