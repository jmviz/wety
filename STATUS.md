# Status

*Updated 2026-10-17. UI improvements are on `codex/client-ux-audit`; not deployed.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…`, the 2026-09-01 dump; 8,101,979 etymological entries with
  stable word URLs and homographs. Dates cover 348 codes, including 135 estimated
  proto-language ends.
- **Local client:** port 8000 against the full local graph on port 3000 while the
  review processes remain running.
- **Research store:** private evidence, artifacts and runs stay in `data/`, backed up
  with `wety-data store`; [provenance](docs/architecture/provenance.md) covers recovery.

## What changed

- Cognates default to **Base lineage**, following the processor's recorded single
  base in both directions. **All components** retains the broader affix/compound
  view, with `scope=all` in shared links. API clients retain the old all-components
  default; the new client sends its scope explicitly.
- Head traversal reads existing graph edges and stops at ambiguous compounds, rather
  than trusting older summaries that can end at an intermediate word. No graph
  rebuild is required. With Latin and English selected, Latin *status* has 1,123
  base-lineage words versus 3,217 through all components.
- Target-language suggestions use distinct reachable words in this lineage, with
  the browser language first when present. Counts precede target/display filtering;
  there is no fixed favoured-language list. `/explore/:id` caches this context.
- Filters have presets, collapsed groups limited to connection types present, a
  searchable bounded language list, visible exclusions and reset controls.
- Homographs use a compact selector and full selected sense; search definitions get
  separate lines. Hover previews stay brief; full details disclose date evidence,
  varieties, alternative estimates and bibliographic citations.
- Tree guides explain direction, relationships and date labels. Outline controls
  stay visible while scrolling; “Jump to searched word” reopens and focuses its path.
- Checks pass: workspace compilation, 10 core tests, 7 server tests, production client
  build and TypeScript. Real-data browser checks cover suggestions, both scopes,
  *blackbird*'s missing-base state, presets/reset, expansion/jump, citations and mobile.

## Uncertainties

- Recorded bases are morphological analysis, not guaranteed semantic/syntactic heads
  or strict inherited cognacy. Loans and word formations remain explicitly included.
- Touch hardware and other browser engines were not tested.
- Deploy the updated API with the client; its exploration endpoint is new.
- Research reviews remain model reviews; R2 pruning still needs a live dry run.

## Next three actions

1. Maintainer: review the local UI and branch before merging.
2. Inspect the R2 pruning dry run before the next server deployment.
3. Resume approved dating research from `dating period-targets`.

## Decisions needed from the maintainer

- None; the requested UI changes are implemented.
