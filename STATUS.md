# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2` (acd-1, lila-1, core-2, acd-2, lila-2), then
`claude/claims-core-3`, then `claude/claims-core-4` (core-5 merged in).*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump. The stack adopts `3da0f0c9…`:
  10.1 million words and 0.8 million cited-only ones, 9.95 million links, 6,003
  languages, now with each entry's References; it needs the stack's server. Dates cover
  348 codes, 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **core-4 done:** every etymology in the graph says who asserts it (Wiktionary's editors
  2.8 million, wety's versioned rules 5.8 million); `graph-assertions` writes all 8.6
  million in the assertion schema.
- **References in the graph:** a new workflow, `wiktionary_references`, reads the pages
  dump into a bibliography and each References section's credits; the graph build reads
  that product, and the processor joins sections to its own items. 562,108 entry credits
  on 349,671 assertions, 641,494 words covered. The superseded join guessed the
  processor's keys in Python and lost words to a differing canonical form (eu *edun*).
  Trees are byte-identical; a 1-in-50 sample validates with 0 errors. Check:
  `data/runs/ccaf05246fcb466f94916b3f5ae6654a/checks/core-4-references/`.

## Uncertainties

- Inline citations exist only in prose, and footnotes and References are credited to a
  whole section: wiktextract keeps no position. Parsing the Wiktionary HTML dump directly
  (with the XML dump), a separate project the maintainer envisions, would recover both.
- 6,936 References sections (1.1%) cover no word: mostly entries the processor makes no
  word of (no gloss, phrases, unlisted languages); not every one was examined.
- Some "or" groups are editors' slips for "+" (2 of 5 in core-2's sample).
- The client shows no alternatives or credits yet (core-6). `/cognates` and
  `/descendants` changed shape on the client branch: ship server, client and graph
  together.

## Next three actions

1. core-6: API and client for alternatives, hedges and credits.
2. acd-3 and lila-3: match ACD and LiLa records to items.
3. syn-2: model extraction of prose citations and conjectures.

## Decisions needed from the maintainer

- Whether to delete the superseded join's outputs: `checks/entry-credits/` of run
  afe2a411 and core-5's `entries.jsonl.gz`, `entry_credits.jsonl.gz`.
- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
- Whether to ship the client work before the claims work lands.
