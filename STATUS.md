# Status

*Updated 2026-10-03. Branch `dating-pilot` (from `reorg-baseline`), pushed; not merged
(maintainer: do not merge yet).*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **In the app:** a "dates" toggle on the view bar. **11 of the 20 pilot codes have a
  displayable, cited period.** Eight are full intervals: `ang enm gmh` (ISO 639-3
  conventional periods), `hit och ltc` (Glottolog documentation windows, shown as
  attestation) and `gem-pro cel-pro` (Koch 2020 estimates). Three are partial: `sga`
  and `grc` (end only), `la` (first written record only). The popover shows quote,
  locator, link and other sources' periods. Checked in a browser on the legacy graph.
- `GET /periods` serves `data/products/dating/language_periods.json` (tracked;
  `WETY_PERIODS_PATH` overrides; a missing file serves an empty list).
- Registry: 3,614 claims (22 supported), 11 adopted decisions, 0 family splits.
- ISO 639-3 names and Glottolog windows are accepted display sources when nothing
  better is available (maintainer, 2026-10-03).

## What changed

- Pilot: one Astra run checked 20 codes in 1,017 s; review corrected 6 items. See the
  [run README](data/workspaces/dating/_research/period-pilot-2026-10-03/README.md).
- Research caps removed (time budget bounds runs; 400 model turns; OpenRouter cost
  stop kept). Research tools gained `find` (matching lines with exact offsets).
- Decisions bind to their subject's claims, sources and catalog record plus
  `DECISION_RULES`, not the whole harness fingerprint (`harness.cli decision-digest`).
  A later republish with new IE-CoR inputs left all 11 decisions valid.
- Catalog: 19 reviewed scope relations from the pilot (`ang enm gmh goh sga grc got
  akk hit och ltc ojp`). A code may hold several reviewed decisions; a scope relation
  (narrower/broader) keeps the catalog lookup. Catalog and IE-CoR matches rebuilt.
- `harness.cli gc` applied: 48 dating generations (2.1 GB) reduced to 5 (240 MB):
  current, two most recent, and two named by tracked records. Removal log is in
  `data/archive/dating/gc/`.

## Uncertainties

- Most adopted sources are catalogs, not handbooks: the four handbooks were unreadable.
- No usable period: `goh non got cu akk sa ojp ine-pro sla-pro`. The pilot left the
  scope of `cu sa la non` unresolved; no catalog relation was recorded for them.
- The review-to-packet step (registering sources, building claims) was a one-off
  script in a scratchpad; the packet records its output, not the procedure.

## Next three actions

1. PDF/handbook access route: a separate session (handoff written). Then retry the 9
   missing codes from this branch.
2. Turn the review-to-packet integration into a harness command, so a reviewed run
   imports reproducibly (`assertions.jsonl` + `review.json` -> packet).
3. Resolve the scope of `cu sa la non` during the retry and record it in the catalog.

## Decisions needed from the maintainer

- None open. Merging `dating-pilot` into `main` waits for the maintainer (it deploys).
