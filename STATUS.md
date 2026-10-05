# Status

*Updated 2026-10-10. Local `main` has `canonical-forms` and `processor-review`; graph
`dab06f67…` is selected. Branch `claude/language-dating-period-9aa9ac` (main merged in)
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

- No shared code is part of a fingerprint any more: catalogs (`e5c5221`), dating (harness
  and `uv.lock` only) and the IE-CoR receipt. Tooling can live where it fits; the store
  is `wety-data store`.
- The session scripts had credited eight GPT-6.1 Sol runs to gpt-6-astra: ledger errata,
  34 decisions re-recorded, READMEs regenerated.
- Removed: source reuse rights and the linked-data release (maintainer decision).
- Deleted the pushed `dating-pilot` and `reorg-baseline` branches, which carried the
  pilot's research data.

## Client fixes (`claude/client-ui-ux-review-3d08c7`, on `processor-review`)

Filters no longer blank the tree; wide charts open centred on the word; home clears
the last word; empty cognates explain themselves; wrapped rows align.

## Uncertainties

- Reviews are model reviews (Sol or Astra research, Claude review).
- Merging this branch deletes the newly ignored files from `main`'s working tree; they
  are in `main/data` already (`sync`) and in the Desktop copy.
- 566k graph citations are decided by meaning alone (about 85% accurate offline).

## Next three actions

1. Maintainer: R2 bucket, `brew install restic`, `restic init`, first
   `wety-data store snapshot`
   ([setup](docs/architecture/provenance.md#setting-up-the-store)).
2. Maintainer: merge, `sync` back, then the range-limited history rewrite before any
   push ([recipe](docs/architecture/provenance.md#leaving-research-data-out-of-published-history));
   deployment setup and graph upload.
3. Supply the queued works in Zotero, then `dating holdings sync`.

## Decisions needed from the maintainer

- None.
