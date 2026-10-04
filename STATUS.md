# Status

*Updated 2026-10-03. Dating: branch `dating-pilot`, not merged (do not merge yet).
Deployment: branch `claude/server-deployment-automation-950d54`, not merged.*

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
- Research caps removed (time budget bounds runs); research tools gained `find`.
- Decisions bind to their subject's claims, sources and catalog record plus
  `DECISION_RULES`, not the whole harness fingerprint (`harness.cli decision-digest`).
- Catalog: 20 reviewed scope relations (`ang enm gmh goh sga grc got akk hit och ltc
  ojp la`). A code may hold several reviewed decisions; a scope relation keeps the
  catalog lookup. A scope run for `cu sa la non` settled only `la`.
  See the [scope run README](data/workspaces/dating/_research/scope-check-2026-10-03/README.md).
- `build-packet` / `adopt`: reproducible review-to-registry import (reproduces the
  pilot packet byte for byte).
- Deployment: pushing `main` deploys the server (CI build, R2, health check, rollback),
  then the website. One-time manual setup, untested on the host: [deploy/README.md](deploy/README.md).

## Uncertainties

- Most adopted sources are catalogs, not handbooks: the four handbooks were unreadable.
- No usable period: `goh non got cu akk sa ojp ine-pro sla-pro`. Scope of `cu`, `sa`
  (both catalogs) and ISO `lat`/`non` remains unresolved; it needs catalog
  denotations that the sources read do not give.
- The catalog's Wiktionary export is legacy, and current Wiktionary has drifted
  (Old Gutnish split off `non`, Sanskrit stage codes renamed, more varieties under `la`).

## Next three actions

1. PDF/handbook access route: a separate session (handoff written). Then retry the 9
   missing codes from this branch with `build-packet`/`adopt`.
2. Refresh the Wiktionary export behind the catalog (a source refresh: rebuild and
   re-review stale catalog and dating decisions). Then record `non` as narrower.
3. Settle `cu`/`sa` scope during the retry, where handbooks define the stages.

## Decisions needed from the maintainer

- Whether to refresh the Wiktionary export (action 2); it touches every consumer.
- Merging `dating-pilot` into `main` waits for the maintainer (it deploys).
