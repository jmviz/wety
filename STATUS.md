# Status

*Updated 2026-10-11 on branch `claude/cranky-noyce-c69059` (`dating gc` fix). Local
`main` has `canonical-forms` and `processor-review`; graph `dab06f67…` is selected. Branch `claude/language-dating-period-9aa9ac` (main merged in)
adds dating rules 2, period batches, proto-language estimates, and moves research data
out of git. Nothing pushed.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Graph** `dab06f67…` (2026-09-01 dump): 8,101,979 entries with an etymology;
  homographs resolved by ids, Descendants lists, then meaning. Stable word URLs.
- **Deployment:** `main` → R2 → `production`; host setup not yet run, graph not uploaded.
- **Dates:** 348 codes show a cited period; 135 proto-languages an estimated end with
  its method and reported mean or median; tree estimates can stand beside any proto
  default (Proto-Mayan: Kaufman 4200 BP, Atkinson's Bayesian roots beside).
- **Research store:** git holds code, docs, the app's inputs, briefs and run READMEs;
  everything else is in `data/`, merged between checkouts with `wety-data store sync`
  and snapshotted with restic once configured
  ([provenance](docs/architecture/provenance.md)). A copy of every checkout's `data/`
  is on the Desktop (`wety-data-2026-10-09`, APFS clones on the same disk).
- **Editorial ledger:** decisions, withdrawals, sources and errata are append-only
  events; concurrent sessions merge by copying files.
- **Tooling:** `dating brief | review | publish | run-readme | review-summary |
  adopt-clades | ledger` replace session scripts.

## What changed

- `dating gc` counts references in the research store: besides tracked files it reads
  `data/workspaces/dating` (packets, run directories, evidence, ledger, family
  workspaces), skipping retrieved and retained source text, caches and binaries. On
  the main checkout's store it now keeps Uralic packet baseline `a82941da…`, which the
  old scan listed for removal; the scan takes about 2 s. `gc --apply` was not run.
- Holdings and source requests are described as research-store records, not tracked.
- Republished dating generation `840b8f8b…` for the harness edit: 0 validation errors,
  no rejected decisions, the same 348 periods.
- Earlier on `main`: client fixes (filters no longer blank the tree, wide charts open
  centred, empty cognates explain themselves), relative store snapshots, deploy token
  names. Never merge `claude/client-ui-ux-review-3d08c7`: its history carries research
  data.

## Uncertainties

- Reviews are model reviews (Sol or Astra research, Claude review).
- Merging this branch deletes the newly ignored files from `main`'s working tree; they
  are in `main/data` already (`sync`) and in the Desktop copy.
- 566k graph citations are decided by meaning alone (about 85% accurate offline).

## Next three actions

1. Maintainer: fast-forward `main`, rename the rclone remote, retake the snapshot
   relatively and forget the first one, commit `data/SNAPSHOT.json`.
2. Maintainer: the range-limited history rewrite, then push `main`
   ([recipe](docs/architecture/provenance.md#leaving-research-data-out-of-published-history)).
3. Maintainer: server rebuild and deployment ([deploy/README.md](deploy/README.md) steps 6–9).

## Decisions needed from the maintainer

- None.
