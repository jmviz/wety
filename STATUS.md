# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2`; `claude/claims-acd-2-5deff1` (this) and lila-2 run in parallel
on core-2.*

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
  `wety-data store` (snapshot `fcae3295`); [provenance](docs/architecture/provenance.md).

## What changed

- **acd-2 done:** the whole ACD v2.0 release enters as 412,197 validated assertions
  (0 errors): 10,857 cognate sets, 2,364 cf groups by the kind map (loan, noise, near,
  root, also, 424 Dempwolff reconstructions), 716 doublet and 340 disjunct alternative
  sets, 7,348 borrowings. Every count equals the release's tables.
- A wety rule compiles 112,901 links from the ACD's tree, each marked "implied by the
  ACD's tree": 110,552 reflexes inherited from their set's reconstruction, 1,687
  sub-entries descending from their main entry, and 181 near groups given an unnamed
  ancestor (662 links at "possible").
- **Schema v1** ([packages/etymology/schema](packages/etymology/schema/README.md)):
  `compiled` on assertions; empty member lists for 5 ACD sets with no listed forms. Two
  ACD extensions added (`name`, `of_set`); none promoted. Result:
  `data/workspaces/etymology/sources/acd/checks/acd-2/result.json`.
- **lila-2 runs in parallel** on its own branch; its schema version may clash with v1 at
  merge (one bump covers both).

## Uncertainties

- ACD doublets and disjuncts are between forms; v1 holds alternative sets of cognate
  sets, so 2 groups (within one set) have none and 5 collapse into another group's set.
- No link for 99 loan-flagged reflexes, 174 reflexes the tree puts outside their set's
  level, 998 sub-entries at their main entry's level, 11 sub-entries outside it.
- 88 of 181 unnamed ancestors sit at PWMP, which has no Wiktionary code.
- `/cognates` and `/descendants` changed shape on the client branch: ship server, client
  and graph `a1bdab67` together.
- The Wiktionary sample's claims are a model's unreviewed annotation; 1,764
  `{{etymon}}` "or" groups is an upper bound (core-3).

## Next three actions

1. lila-2 finishes; the maintainer merges acd-2 and lila-2 into the stack.
2. core-3: several etymologies per word, checking how `{{etymon}}` groups read.
3. acd-3: match ACD records to items, weighing gloss and reflexes, not form keys alone.

## Decisions needed from the maintainer

- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether a reported rejection ("denied by De Vaan") enters the core schema now.
- Whether to ship the client work before the claims work lands.
