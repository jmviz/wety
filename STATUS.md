# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2` (acd-1, lila-1, core-2, acd-2, lila-2), then
`claude/claims-core-3`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump. The stack adopts `68f378ee…`
  (core-3), which needs the stack's server. Dates cover 348 codes, 135 estimated
  proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **core-3 done:** a word may have several etymologies. 2,089 words show alternatives
  from `{{etymon}}` ("from A or B"), and 8 from a root named under several senses.
  2,007 have no primary, so trees follow every alternative; 90 have one, the others
  hedged. 305 etymologies keep every parent joined by "and" (core-1 kept the first),
  2,007 links are held "possible", 10,693 words are of known-unknown origin (`{{unk}}`).
- The check compares the edges trees follow with core-1's graph: 4,953 words changed,
  each listed with its reason (`data/runs/f3303f112c0940628abae7f26fca83f9/checks/core-3/`).
  It found and fixed five bugs.
- A word's own section now replaces another page's Descendants link unless every
  parent it names is missing from the data (before: unless any is). 2,232 words change
  this way, 2,201 of them compounds and affixations (ro *interfață*: *inter-* + *față*,
  before "calque of en *interface*").
- Lost root votes (114) are either alternatives that no longer vote for one line, or
  core-1 errors (*tauglich* had PIE \*keyt-, which its derivatives' suffix *-keit* carries).

## Uncertainties

- Some "or" groups are editors' slips for "+" (2 of 5 in core-2's sample):
  `{{etymon|en|-ic|-s}}` renders "from *-ic* or *-s*". wety reads what Wiktionary shows.
- 48 other parent changes and 23 other lost etymologies are listed, not each traced.
- Not read yet: `{{unc}}` prose conjectures (syn-2), a mid-chain `{{unk}}`, alternatives
  inside an inline etymology. The client does not show alternatives yet.
- `/cognates` and `/descendants` changed shape on the client branch: ship server, client
  and graph together.

## Next three actions

1. core-5: References from page wikitext as entry-scope credits; it needs only core-2.
2. core-4: the processor emits Wiktionary's assertions, now that a word can carry
   alternatives and hedges.
3. Maintainer: review the stack, lila-2's readings in `etc/forms.json`, and the decisions
   below. acd-3 and lila-3 wait for core-4.

## Decisions needed from the maintainer

- Whether a word's own compound should replace a Descendants link when one of its parts
  is missing from the data (core-3 says yes: 2,232 words).
- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
- Whether to ship the client work before the claims work lands.
