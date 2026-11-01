# Status

*Updated 2026-10-31. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump; 8,101,979 etymological
  entries. This branch adopts `a1bdab67…` (9,946,413 edges, each with its basis), which
  needs this branch's server. Dates cover 348 codes, 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs stay in `data/`, backed up
  with `wety-data store`; [provenance](docs/architecture/provenance.md) covers recovery.

## What changed

- **Etymology claims model adopted** (shared doc "wety etymology claims model",
  outside git). wety will build trees from attributed claims: alternatives, certainty,
  credits between sources, automatic synthesis. The doc's work plan is the tracker:
  14 milestones in four tracks (core, ACD, LiLa, synthesis), one branch each
  (`claude/claims-<id>`), commits titled `Claims <id>: …`.
- **core-1 done:** every graph edge records its basis: `etymon`, `template`,
  `descendants` (Wiktionary's) or `chain`, `form_of`, `root`, `root_vote` (wety's
  rules). `graph-edges` dumps edges for diffing builds. Against `dab06f67`, only root
  edges changed, plus 455 stated etymologies restored: root votes had overwritten
  compounds of two bases (fixed, `e28afacb`). Rules alone assert 49,538 edges, form-of
  links aside. Lone `{{m}}` never fires: the dump has none. Results are in the run's
  `checks/core-1/`. Old graph files no longer load.
- **Earlier on this stack:** the client UX review is worked through; the root fixes
  (testum's \*per-, two-sense roots) are in graph `a1bdab67`.
- Checks pass: 44 processor, 15 core and 7 server tests; clippy clean.

## Uncertainties

- `/cognates` and `/descendants` changed shape on the client branch: deploy server and
  client together, with a rebuilt graph.
- Research reviews remain model reviews; R2 pruning still needs a live dry run.

## Next three actions

1. core-2, the assertion schema, on a branch holding both core-1 and lila-1
   (`claude/claims-lila-1`, stacked on acd-1, both cut from `921b69a0`; their checks
   passed). Joining them needs the maintainer's word.
2. acd-2 and lila-2: the converters, once core-2 lands.
3. Maintainer: review the stack; ship server, client and graph `a1bdab67` together.

## Decisions needed from the maintainer

- The doc proposes a new Python package, `packages/etymology`, for the schema and
  converters (a layout change); it stands unless the maintainer objects.
- Whether to ship the client work before the claims work lands.
