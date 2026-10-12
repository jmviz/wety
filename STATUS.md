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
  current snapshot.
- **Editorial ledger:** decisions, withdrawals, sources and errata are append-only
  events; concurrent sessions merge by copying files.
- **Tooling:** `dating brief | review | publish | run-readme | review-summary |
  adopt-clades | ledger` replace session scripts.

## What changed

- Docs describe the running system instead of the migration: deploy README (operating,
  setting up a host, accounts and secrets), provenance (history rewrite reduced to a
  note), AGENTS (restore from the store; the boundary-commit rule removed), README and
  dating README (what git tracks), organization page trimmed to the current layout.
- Links from tracked docs to research-store files (packets, run reports) became code
  text; AGENTS now says to link only to tracked files.

## Uncertainties

- Reviews are model reviews (Sol or Astra research, Claude review).
- 566k graph citations are decided by meaning alone (about 85% accurate offline).
- `dating gc` keeps a generation only if a git-tracked file names it; packets are no
  longer tracked, so a generation named only by a packet would be listed for removal.
  Do not run `gc --apply` until this is fixed.
- `deploy/bootstrap.sh` does not install the host firewall rules; a rebuilt host relies
  on the image's rules or a manual step.

## Next three actions

1. Fix `dating gc` to count references in the research store, not only tracked files.
2. Add the host firewall rules to `deploy/` and `bootstrap.sh`, if approved.
3. Resume dating research from `dating period-targets`.

## Decisions needed from the maintainer

- Firewall rules in `bootstrap.sh`; delete `docs/architecture/history-filter.txt`;
  merge `docs/architecture/reorganization.md` into the README and provenance.
