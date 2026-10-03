# Status

*Updated 2026-10-03. Local `main`; nothing pushed.*

## What the project promises

wety shows Wiktionary etymologies as trees. The dating work will let a user see when
each language in a tree was spoken, optionally on etymology/descendant trees and on a
separate language-family page. Each language node gets a time interval with
uncertainty, its basis (conventional period, attestation, model estimate) and
citations, derived from the evidence registry in `packages/dating`.

## What is usable today

- **In the wety app: nothing yet.** No code in `crates/` or `client/` reads dating data.
- Evidence registry: 3,575 claims (5 supported, 3,256 candidate, 314 quarantined);
  0 of 8,625 Wiktionary codes have an exported chronology. Validation passes.
- Reproduction of three deposited phylogenetic analyses (IE, Sino-Tibetan, Uralic).
- Source-level dossiers for Uralic, Indo-European, Sino-Tibetan, Japonic and Turkic:
  [packages/dating/families/](packages/dating/families/). All are proposals or
  context; none maps a sampled clade to a Wiktionary stage.

## What changed in this checkpoint

- The reorganization (`crates/`, `packages/`, `workflows/`, `data/`) is committed.
  `data/` is ignored by default; only small records are tracked.
- Retired the multi-role harness experiments (9 modules, 47 tests). All test
  suites and `cargo check` pass.
- Dating docs reduced from 31 files to the README, 5 reference docs and 5
  family pages. Agent rules are in [AGENTS.md](AGENTS.md).
- Integrated the server performance work (response cache, typed JSON,
  compact descendant langs) and the redesigned client trees (HTML outline and
  chart, lean payloads, `/item` and `/lang`). The selected legacy graph still
  loads; both tree views were checked in a browser.

## Known issues and uncertainties

- Accuracy of the single frontier researcher is unmeasured. The Japonic review
  found 11 issues across 12 records in two runs that had passed self-checks.
- Every harness code edit invalidates the dating generation fingerprint
  (`registry.py` `context_hashes`). This forces a republish; nothing is lost, but
  it churns decisions once they exist.
- Dating, catalog and workflow tests read local data. In a fresh clone without the
  backup, many fail rather than skip.
- 37 dating generations (1.6 GB) are kept locally. There is no retention rule yet.

## Next three actions

1. **Display export schema.** Define the per-node record that both views need:
   start/end as ranges, basis, citations, and family split estimates as separate
   records. Produce it from the registry as `language_periods.json`.
2. **Pilot of about 20 codes, end to end.** Mix of conventional stages, attested
   languages and proto-languages. Path: source check, code mapping, adoption, export,
   server endpoint, client toggle. Stop and report if the registry path cannot carry
   them within one bounded session.
3. **Retention.** `gc --dry-run` keeping the current generation, the previous two,
   and anything referenced.

## Decisions needed from the maintainer

- Push `main` to GitHub. Before pushing, confirm the Brill font licence permits
  redistributing the `.woff2` files in `client/public/fonts` publicly.
- Remove the merged and superseded worktrees: `rust-server-perf`, `wety-tree-viz`,
  `upbeat-tharp`, `condescending-moore`. The perf worktree holds a 3.1 GB
  converted graph in its ignored `data/`.
- Whether to narrow the dating fingerprint to the modules that affect evidence
  semantics.
