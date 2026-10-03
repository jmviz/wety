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
- Research caps removed: searches, acquisitions and tool actions are uncapped
  (`null`); the time budget bounds runs. The model-turn cap is now 400; the OpenRouter
  cost stop stays.
- Research tools gained `find`: matching lines with exact offsets plus the table
  header. The pilot spent 183 reads paging one CSV.
- `harness.cli gc` (dry run by default) retires old generations. It keeps the
  current one, the two most recent, and any named by a tracked file.

## Uncertainties

- Most adopted sources are catalogs, not handbooks: the four handbooks were unreadable.
- No usable period: `goh non got cu akk sa ojp ine-pro sla-pro`.
- The `enm`/`ltc` mapping overrides live only in the claims' subjects. The catalog
  layer has no reviewed scope relation for them, so the next run would redo them.
- Every harness edit makes all decisions stale; they were re-recorded once today.

## Next three actions

1. PDF/handbook access route: a separate session (handoff written). Then retry the 9
   missing codes from this branch.
2. Record the pilot's scope judgments as reviewed catalog relations (`equivalent`,
   `source_narrower`) for `enm ltc goh cu sa ojp sga`. This needs a catalog rebuild.
3. If approved, bind decisions to their evidence and a semantic rule version instead
   of the whole harness fingerprint.

## Decisions needed from the maintainer

- `gc --apply`: the dry run removes 33 of 43 generations (1.66 GB), keeping 10
  (0.49 GB). Seven of the ten are kept only because the reorganization inventory
  (`data/registry/migrations/`) lists them. Should that inventory count as a
  reference?
- Approve action 3 (fingerprint narrowing).
