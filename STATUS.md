# Status

*Updated 2026-10-04. Branch `etymon-templates` (from `main`), not merged or pushed.
Local `main` has `wiktionary-refresh`, `stable-links` and the deployment automation.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Graph** `etymology-graph/bf5c28d2…` (2026-09-01 dump) is selected for local use and
  deployment. Homographs are separate items; Descendants sections are read.
- **New graph** `etymology-graph/7c39072a…` (same dump, branch `etymon-templates`,
  validated, copied into `data/`; not selected): reads `{{etymon}}`/`{{ety}}`.
  About 376k Wiktextract entries whose only etymology templates are `{{etymon}}`
  (pl 64k, en 64k, es 40k, pt 27k, la 21k) have a parseable parent. Latin *mātrīx*
  now derives from *māter* + *-trīx*; English *father* from Middle English *fader*.
- **Stable links**, **deployment** via `main` → R2 → `production`
  ([deploy/README.md](deploy/README.md), untested on the host); **dates** for 14 of
  20 pilot codes.

## What changed

- The processor parses `{{etymon}}` per Module:etymon ([etymon.rs](crates/processor/src/etymon.rs)).
  The first structural keyword group gives the parent(s); `:af`/`:blend`/`:univ`
  give components, other keywords the first alternative; `:root`, `:afeq`,
  `:influence` are skipped; inline `<ety:…>` continues the chain.
- Where an entry has both, `{{etymon}}` wins; the classic templates after the one
  naming the same parent continue the chain. Without a usable group, classic
  templates are used as before.
- `<id:…>` references resolve to the item whose `{{etymon}}` has that `id=`.
- Classic template terms lose inline modifiers (`līber<id:free>` → `līber`).

PIE descendant trees, English+Latin (nodes; extra parents in brackets):

| Root | `bf5c28d2` | new | api.wety.org |
|---|---|---|---|
| *méh₂tēr* | 121 (21) | 179 (39) | 143 (30) |
| *bʰer-* | 781 (79) | 915 (216) | 946 (187) |

## Uncertainties

- *bʰer-* loses 220 baseline nodes. In a sample of 40 lost English/Latin items, 24
  still descend from it but now sit under their own-language base word (*broaden*
  under *broad*), which the descendants view does not expand. 16 no longer reach it;
  most had been attached by root-category shortcuts, several wrongly (*Middlesex*,
  *mark*, *tell*). Not checked beyond the sample.
- PIE roots written with a final hyphen in `:af` read as prefixes (as with `{{af}}`).
- `id=`-resolved links get confidence 1 and so override Descendants-derived links.
- Family-code etymons (`gem:…`) are skipped; Old French terms display junk canonical
  forms (*octobre oblique singular or*), unchanged from before.
- Host deployment setup not run; client 409 reload not exercised in a browser.

## Next three actions

1. Maintainer: review and merge `etymon-templates`; select `7c39072a…` in
   `data/products/etymology_graph/CURRENT.json` and `deploy/graph.json`, then publish
   with `deploy/publish-graph.sh`.
2. Maintainer: one-time deployment setup, including the first push of `main`.
3. Supply the six queued works in Zotero, then `dating holdings sync`.

## Decisions needed from the maintainer

- Whether the new graph replaces `bf5c28d2` (trees grow; some root-shortcut links
  disappear, see above).
