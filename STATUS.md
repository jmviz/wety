# Status

*Updated 2026-10-04. `wiktionary-refresh` fast-forwarded into local `main` (not
pushed); branch `stable-links` from it, not pushed.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Production** (`api.wety.org`) serves the preserved legacy graph; its numeric IDs
  match `data/artifacts/etymology-graph/legacy` and the legacy language table.
- **Refreshed graph** (`etymology-graph/18a9fa26…`, 2026-09-01 dump) loads and
  validates but is **not fit to deploy** (see Uncertainties).
- **Dates:** 14 of 20 pilot codes have a cited period. Supplied sources and requests
  as before (`dating holdings`, `dating requests`).

## What changed

Investigated stable shared links (findings from `experiments/stable_links/`, run on
the legacy and 2026-09-01 dumps):

- 1.6% of language sections (1.2% English) have several etymologies.
- Over the interval between the dumps, 99.1% of numbered etymologies kept their
  number; 0.4% of those with etymology text now carry a number that names another
  etymology (e.g. `en kid` 2 → 3 after an insertion).
- Wiktionary's own etymology IDs are too sparse to key on: `{{etymon|id=}}` covers
  1.3% of etymologies on multi-etymology pages, senseids 1.8%.
- 172 of 8,625 main language codes were retired (108 recoverable by name); 212 kept
  codes changed names.

## Uncertainties

- The refreshed graph merges homographs: Wiktextract now emits `etymology_number`
  as a string (`"3"`, also `"1.1"`), the processor reads an integer, so all of
  `en lime`'s five etymologies are one item.
- Descendants sections are ignored: both available dumps use nested descendants
  without `depth`/`templates`, which the processor requires. Trees shrink (PIE
  *méh₂tēr, English+Latin: 173 nodes in production, 84 refreshed).
- `{{etymon}}` is still not parsed.
- Legacy dump date is unknown (file dated 2026-04-08), so drift per month is approximate.

## Next three actions

1. Processor: read string etymology numbers, parse nested descendants; rebuild and
   compare trees against production.
2. Stable links, server: resolve `language code + term + optional Wiktionary
   etymology number`; codes in all URLs; translate production's numeric links from the
   legacy graph.
3. Stable links, client: key-based URLs and a homograph switcher; deploy together with
   the rebuilt graph.

## Decisions needed from the maintainer

- Approve the stable-link design (URL form, legacy-link translation) before
  implementation.
- Do not deploy the refreshed graph until action 1 is done; whether local `main`
  should be reset until then (it now contains the refresh).
