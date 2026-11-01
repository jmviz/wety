# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2`, which now holds acd-1, lila-1, core-2, acd-2 and lila-2.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump. The stack adopts `a1bdab67…`
  (9,946,413 edges, each with its basis), which needs the stack's server. Dates cover
  348 codes, 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store` (snapshot `c76478bb`); [provenance](docs/architecture/provenance.md).

## What changed

- **Claims model** (shared doc, outside git): core-1, acd-1, lila-1, core-2 done.
- **Assertion schema v1** ([packages/etymology/schema](packages/etymology/schema/README.md)):
  `compiled` marks what a wety rule derived from a source's structure. Extensions are
  declared with their concept; a test fails while two sources declare one concept.
- **acd-2 done:** the whole ACD v2.0 release enters as 412,197 validated objects: 10,857
  cognate sets, 2,364 cf groups by the kind map, 716 doublet and 340 disjunct sets,
  7,348 borrowings, every count equal to the release's tables. A wety rule compiles
  112,901 links from the ACD's tree, each "implied by the ACD's tree", and gives 181
  near groups an unnamed ancestor.
- **lila-2 done:** all of LiLa's de Vaan data enters, nothing lost: 1,437 etymologies,
  2,653 links, 2,860 reconstructions (165 with de Vaan's doubt), credits to Schrijver
  and LIV. Damaged forms are read by rules or in `etc/forms.json` (48).
- Checks pass: 44 `packages/etymology` tests; all five converted outputs revalidate.

## Uncertainties

- ACD doublets and disjuncts are between forms; alternative sets hold cognate sets, so 2
  groups have none and 5 collapse into another group's set.
- 1,282 ACD reflexes and sub-entries get no compiled link (loans, or outside their
  level); 88 of 181 unnamed ancestors sit at PWMP, which has no Wiktionary code.
- LiLa encodes one chain where its curators note de Vaan gives alternatives, and 7
  etymons it never linked; its 206 etymon-to-etymon cognate links are undocumented.
- The Wiktionary sample's claims are a model's unreviewed annotation; 1,764 `{{etymon}}`
  "or" groups is an upper bound.
- `/cognates` and `/descendants` changed shape on the client branch: ship server, client
  and graph `a1bdab67` together.

## Next three actions

1. core-3: several etymologies per word, checking how `{{etymon}}` groups read.
2. core-5: References from page wikitext as entry-scope credits; it needs only core-2.
3. Maintainer: review the stack, lila-2's readings in `etc/forms.json`, and the decisions
   below. acd-3 and lila-3 wait for core-4.

## Decisions needed from the maintainer

- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments (a
  wety inference, not LiLa's claim).
- Whether to ship the client work before the claims work lands.
