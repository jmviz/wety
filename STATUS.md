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
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump. The stack adopts `6401e3c9…`
  (core-4): 10.1 million words and 0.8 million cited-only ones, 9.95 million links,
  6,003 languages; it needs the stack's server. Dates cover 348 codes, 135 estimated
  proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **core-4 done:** every etymology in the graph says who asserts it. Wiktionary's editors
  assert 2.8 million, on the page whose section states them; wety's rules, each a
  versioned holder, assert 5.8 million. `graph-assertions` writes all 8.6 million in the
  assertion schema; trees are byte-identical to core-3's.
- **core-5 merged and joined:** References sections, read from the pages dump, credit
  348,693 assertions (559,333 credits on 8,347 works) in the export; footnotes credit
  2,605. A 1-in-50 sample validates with 0 errors. Check:
  `data/runs/afe2a4119ee944f7932a1684e01163cb/checks/entry-credits/`.

## Uncertainties

- Inline citations exist only in prose (`{{R:...}}` never reaches wiktextract's
  etymology templates), and footnotes and References are credited to a whole section:
  wiktextract keeps no position. Parsing the Wiktionary HTML dump directly (with the XML
  dump), a separate project the maintainer envisions, would recover both.
- The graph does not hold entry credits, only the export; 6,253 credited entries match
  no word.
- Some "or" groups are editors' slips for "+" (2 of 5 in core-2's sample).
- The client shows no alternatives or credits yet (core-6). `/cognates` and
  `/descendants` changed shape on the client branch: ship server, client and graph
  together.

## Next three actions

1. core-6: API and client for alternatives, hedges and credits (needs the graph or the
   server to hold entry credits).
2. acd-3 and lila-3: match ACD and LiLa records to items.
3. syn-2: model extraction of prose citations and conjectures.

## Decisions needed from the maintainer

- How the graph gets entry credits: a References workflow whose product the graph build
  reads, or the server reading core-5's table beside the graph.
- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
- Whether to ship the client work before the claims work lands.
