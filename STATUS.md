# Status

*Updated 2026-10-09. Local `main` has `wiktionary-refresh`, `stable-links` and the
server deployment automation merged; nothing pushed. Branch
`claude/language-dating-period-9aa9ac` adds dating rules 2, period batches, published
proto-language estimates, and moves research data out of git.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Graph** (`etymology-graph/bf5c28d2…`, in `deploy/graph.json`): homographs are
  separate items; Descendants sections are read. Stable word URLs.
- **Deployment:** pushing `main` deploys the server, then the website. Untested on the host.
- **Dates:** 348 codes show a cited period; 135 proto-languages an estimated end with
  its method and reported mean or median; tree estimates can stand beside any proto
  default (Proto-Mayan: Kaufman, with Atkinson's Bayesian roots beside).
- **Research store:** git now holds code, docs, the app's inputs, briefs and run READMEs
  (401 files, from 7,028). Everything else lives in `data/`, merged between checkouts
  with `tools/wety_store.py sync` and snapshotted with restic once configured
  ([provenance](docs/architecture/provenance.md)). A full copy of every checkout's
  `data/` is on the Desktop (`wety-data-2026-10-09`, APFS clones: same disk).
- **Editorial ledger:** decisions, withdrawals, sources, rights and errata are
  append-only events; concurrent sessions merge by copying files.
- **Tooling:** `dating brief | review | publish | run-readme | review-summary |
  adopt-clades | rights | release` replace session scripts.

## What changed

- The append-only ledger replaced `decisions.json`, `withdrawals.json`, `sources.json`
  (migrated unchanged; published periods identical).
- The session scripts had credited eight GPT-6.1 Sol runs to gpt-6-astra: errata
  recorded, 34 decisions re-recorded, READMEs regenerated.
- `dating release` writes Turtle and JSON-LD of what wety shows; quotes are withheld
  until a source's reuse rights are recorded (none yet).

## Uncertainties

- Reviews are model reviews (Sol or Astra research, Claude review).
- Two pushed branches (`dating-pilot`, `reorg-baseline`) already carry the 2026-10-03
  pilot's research data.
- Merging this branch deletes the newly ignored files from `main`'s working tree; they
  are already in `main/data` via `sync`, and in the Desktop copy.
- A workflow-tools test fails only in worktrees (a symlinked `data/cache`; pre-existing).

## Next three actions

1. Maintainer: R2 bucket, `brew install restic`, `restic init`, first snapshot
   ([setup](docs/architecture/provenance.md#setting-up-the-store)).
2. Maintainer: merge, then the range-limited history rewrite before any push
   ([recipe](docs/architecture/provenance.md#leaving-research-data-out-of-published-history)).
3. Record reuse rights for the most-cited sources (`dating rights` lists them).

## Decisions needed from the maintainer

- Delete the pushed `dating-pilot` and `reorg-baseline` branches?
- Default for quotes of unassessed sources in public releases: withheld (current) or
  short quotes?
