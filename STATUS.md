# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`; acd-1 and lila-1
(`claude/claims-lila-1`, on `claude/claims-acd-1`) are merged into it on
`claude/claims-core-2`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump; 8,101,979 etymological
  entries. This stack adopts `a1bdab67…` (9,946,413 edges, each with its basis), which
  needs this stack's server. Dates cover 348 codes, 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs stay in `data/`, backed up
  with `wety-data store` (snapshot `150fa9a3`, with the ACD and LiLa sources);
  [provenance](docs/architecture/provenance.md) covers recovery.

## What changed

- **Etymology claims model adopted** (shared doc "wety etymology claims model",
  outside git). Its work plan is the tracker: one branch per milestone, commits titled
  `Claims <id>: …`.
- **core-1 done:** every graph edge records its basis: `etymon`, `template`,
  `descendants` (Wiktionary's) or `chain`, `form_of`, `root`, `root_vote` (wety's
  rules). Against `dab06f67` only root edges changed, plus 455 stated etymologies
  restored that root votes had overwritten (fixed, `e28afacb`). Rules alone assert
  49,538 edges, form-of links aside. Results: the run's `checks/core-1/`.
- **acd-1 done:** ACD v2.0 (Zenodo 14608975) retained. Its source profile, kind map and
  language map are in `data/workspaces/etymology/sources/acd/etc/`, which git tracks
  for every source. 920 of 1,022 varieties (90.0%) and 15 of 42 proto-languages have
  an exact Wiktionary code. v2.0 sets none of its `Doubt` flags.
- **lila-1 done:** LiLa's de Vaan data (Zenodo 4147500) retained: 1,437 etymologies.
  Orthography profiles give de Vaan's and Wiktionary's PIE and Proto-Italic forms equal
  comparison keys for all 20 same-reconstruction pairs sampled, and 9 of 9 held out.
  LiLa encodes no certainty beyond 185 "?" forms, and no credits.
- Checks pass: 44 processor, 15 core, 7 server and 16 `packages/etymology` tests.

## Uncertainties

- `/cognates` and `/descendants` changed shape on the client branch: deploy server,
  client and graph `a1bdab67` together.
- Language-map and pair reviews are model reviews. A form key alone pairs ACD
  homographs wrongly (4 of 24 drawn); gloss and reflexes must weigh in (acd-3).
- Research reviews remain model reviews; R2 pruning still needs a live dry run.

## Next three actions

1. core-2: the assertion schema in `packages/etymology`, drafted against 30 Wiktionary
   entries, the 20 ACD sets and the 20 LiLa etymologies in the store.
2. acd-2 and lila-2: the converters, once core-2 lands.
3. Maintainer: review the stack; ship server, client and graph together.

## Decisions needed from the maintainer

- Whether to ship the client work before the claims work lands.
