# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2` (acd-1, lila-1, core-2, acd-2, lila-2), then
`claude/claims-core-3`, then `claude/claims-core-4`. `claude/claims-core-5` branches
from core-3, beside core-4, and is not merged.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump. The stack adopts `6401e3c9…`
  (core-4), which needs the stack's server. Dates cover 348 codes, 135 estimated
  proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **core-4 done:** every etymology in the graph says who asserts it. Wiktionary's editors
  assert 2.8 million, on the page whose section states them (668,000 on another page's
  Descendants list); wety's rules, each a versioned holder, assert 5.8 million, a root
  vote naming its voters ("inferred by wety from 14 derivatives"). Footnotes credit
  2,605 assertions on 1,080 works.
- `graph-assertions` writes all 8.6 million in the assertion schema; a 1-in-50 sample
  (679,730 objects) validates with 0 errors. Trees are byte-identical to core-3's. The
  graph stores 21 MB of assertion tables, against the design's 0.2 GB estimate.
- **core-5 done** in its own session (`claude/claims-core-5`): 990,292 References
  credits on 17.1% of entries with an etymology, and a bibliography of 19,699 works.

## Uncertainties

- Inline citations exist only in prose (`{{R:...}}` never reaches wiktextract's
  etymology templates): syn-2's model extraction. A footnote is credited to its whole
  section.
- core-5's entry credits are not joined to core-4's assertions; its counts are its
  session's report, not checked here.
- Some "or" groups are editors' slips for "+" (2 of 5 in core-2's sample).
- The client shows no alternatives or credits yet (core-6). `/cognates` and
  `/descendants` changed shape on the client branch: ship server, client and graph
  together.

## Next three actions

1. Merge core-5 into core-4 and copy its entry credits onto the entries' assertions.
2. core-6: API and client for alternatives, hedges and credits.
3. acd-3 and lila-3: match ACD and LiLa records to items, now that core-4 is done.

## Decisions needed from the maintainer

- Whether to merge `claude/claims-core-5` into `claude/claims-core-4` for the join.
- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
- Whether to ship the client work before the claims work lands.
