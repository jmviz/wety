# Status

*Updated 2026-10-11. `main` is deployed: api.wety.org serves graph `dab06f67…`, and
wety.org follows `production`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org, deployed by pushing `main` (CI → R2 → host over
  SSH → `production` → Pages; [deploy/README.md](deploy/README.md)). After a restart the
  API answers 502 for a few minutes while the graph loads.
- **Graph** `dab06f67…` (2026-09-01 dump): 8,101,979 entries with an etymology;
  homographs resolved by ids, Descendants lists, then meaning. Stable word URLs.
- **Dates:** 348 codes show a cited period; 135 proto-languages an estimated end with
  its method and reported mean or median; tree estimates can stand beside any proto
  default (Proto-Mayan: Kaufman 4200 BP, Atkinson's Bayesian roots beside).
- **Research store:** git holds code, docs, the app's inputs, briefs and run READMEs;
  everything else is in `data/`, snapshotted with restic to R2 (`wety-research`) and
  merged between checkouts by `wety-data store`
  ([provenance](docs/architecture/provenance.md)). `data/SNAPSHOT.json` names the
  current snapshot, `a23f5c6c…` (with dating generation `840b8f8b…`).
- **Editorial ledger:** decisions, withdrawals, sources and errata are append-only
  events; concurrent sessions merge by copying files.
- **Tooling:** `dating brief | review | publish | run-readme | review-summary |
  adopt-clades | ledger` replace session scripts.

## What changed

- Docs describe the running system instead of the migration: deploy README (operating,
  setting up a host, accounts and secrets), provenance (history-rewrite recipe
  removed), AGENTS (restore from the store; the boundary-commit rule removed), README and
  dating README (what git tracks), `docs/architecture/layout.md` (was
  `reorganization.md`) trimmed to the current layout; `history-filter.txt` deleted.
- Links from tracked docs to research-store files (packets, run reports) became code
  text; AGENTS now says to link only to tracked files.
- Each server deploy now prunes R2 (`deploy/prune-r2.sh`): the five newest releases,
  their graphs, the selected graph and graphs from the last 30 days stay. Tested on a
  local rclone backend only.
- `dating gc` counts references in the research store: besides tracked files it reads
  `data/workspaces/dating` (packets, run directories, evidence, ledger, family
  workspaces), skipping retrieved and retained source text, caches and binaries, in
  about 2 s. It now keeps the Uralic packet baseline `a82941da…`, which the old scan
  listed for removal. Republished dating generation `840b8f8b…` for the harness edit:
  0 validation errors, no rejected decisions, the same 348 periods.

## Uncertainties

- Reviews are model reviews (Sol or Astra research, Claude review).
- 566k graph citations are decided by meaning alone (about 85% accurate offline).
- R2 pruning has not run against R2 yet; its first run is the next server deploy.

## Next three actions

1. Maintainer: before merging, run `deploy/prune-r2.sh` (dry run) and check its list.
2. Resume dating research from `dating period-targets`.
3. Merge this branch into `main` (deploys the server; the first R2 prune follows).

## Decisions needed from the maintainer

- None.
