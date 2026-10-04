# Status

*Updated 2026-10-04. Branch `wiktionary-refresh` (from `local-sources`, from
`dating-pilot`), not pushed; nothing merged into `main`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Refreshed data:** graph, catalog and dating now come from Wiktionary's 2026-09-01
  dump (Kaikki Wiktextract data plus our language export; 8,959 languages, 1,242
  families), retained with receipts. The new graph
  (`etymology-graph/18a9fa26…`) validates and was checked in a browser: search,
  etymology and cognate trees, and the dates toggle.
- **Dates:** 14 of 20 pilot codes have a cited period, unchanged by the refresh (all
  decisions re-reviewed). Catalog scope: `cu` narrower, `sa` broader, `non` narrower
  (Glottolog). Decisions can be withdrawn (`dating withdraw`).
- **Supplied sources and requests:** `dating holdings zotero|sync` ingests PDFs from
  Zotero; researchers use `library`, `open`, `request`; six works are queued in Zotero
  with full records (`dating requests list|enrich|push`).
- Graph builds use Metal on macOS (about 75 minutes on an M1 Pro).

## What changed

Inputs refreshed and selected; language exporter updated for 2026 Wiktionary
modules; processor on candle 0.11 with batch-independent embeddings; catalog and
dating rebuilt; client re-resolves remembered languages by name. See git log.

## Uncertainties

- Language and item IDs are positions in compiled tables: links shared before the
  refresh (`/etymology/ID?distLang=…`) point elsewhere after deployment.
- Wiktionary's `{{etymon}}` template is not parsed (as before); its use is growing,
  e.g. "father" shows only its root. A follow-up task is suggested.
- Embeddings changed (new pooling and candle version), so ambiguous etymology links
  may resolve differently from the legacy graph.
- Still no period: `non got cu sa ojp ine-pro`, mostly scope gaps.

## Next three actions

1. Maintainer: update the host server with the new graph and server binary, then
   merge (deploys the client fix with it).
2. Parse `{{etymon}}` in the processor and rebuild the graph.
3. Supply the six queued works in Zotero, then `dating holdings sync` and a
   continuation run.

## Decisions needed from the maintainer

- Deployment and merge of `wiktionary-refresh` (graph, server and client together).
- Whether to make shared links stable (IDs by code and term rather than position).
