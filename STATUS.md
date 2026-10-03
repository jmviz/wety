# Status

*Updated 2026-10-03. Branch `dating-pilot` (from `reorg-baseline`); pilot in progress.*

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

## Dating pilot in progress: 20 codes

Note: no existing claim has a `wikt:` subject. Candidate claims reach a code only through
its Glottolog link, so the pilot checks the exact code mapping as part of source checking.

- **Conventional stages** — `ang`, `enm`, `goh`, `gmh`, `non`, `sga`: medieval
  periodizations with candidate handbook (Baugh & Cable, Schmidt) or Glottolog/Wikipedia
  claims. They test whether conventional periods survive a source check.
- **Attested languages** — `got`, `cu`, `akk`, `hit`: known from texts. Candidate
  Glottolog documentation windows and Wikipedia periods. They test attestation versus
  conventional basis.
- **Classical languages** — `la`, `grc`, `sa`, `ltc`, `och`, `ojp`: Wiktionary files
  later stages (Late/Medieval/New Latin, Koine/Byzantine Greek, Vedic/Classical Sanskrit,
  Middle Japanese) under these codes. This tests whether a source's period maps
  *exactly* to the Wiktionary code.
- **Proto-languages** — `gem-pro`, `ine-pro`, `sla-pro`, `cel-pro`: no catalog link
  and no candidate claims. They test whether dates cited for the proto stage itself
  (never a family split) can be found.

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

1. Run the pilot above: source check (Astra), review, adoption, `language_periods.json`,
   `/periods`, client toggle.
2. Retention: `gc --dry-run` keeping the current generation, the previous two, and
   anything referenced.
3. Decide whether to narrow the dating fingerprint.

## Decisions needed from the maintainer

- When to merge `reorg-baseline` (pushed) into `main`. Merging deploys the site, so
  the host first needs the new server build and a graph. The legacy graph loads with
  `WETY_ALLOW_LEGACY_GRAPH=1`; a converted copy is at
  `data/workspaces/etymology_graph/wety-csr-converted-2026-09-19.json`.
- Whether to narrow the dating fingerprint to the modules that affect evidence
  semantics.
