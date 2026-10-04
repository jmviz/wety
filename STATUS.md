# Status

*Updated 2026-10-04. Local `main` has `etymon-templates` merged; nothing pushed.
Branch `canonical-forms` is unmerged; its graph `cf09bd87…` is built and validated.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Graph** `etymology-graph/7c39072a…` (2026-09-01 dump), selected for local use and
  in `deploy/graph.json`: homographs are separate items, Descendants sections are read,
  and `{{etymon}}`/`{{ety}}` etymologies are read. About 376k Wiktextract entries whose
  only etymology templates are `{{etymon}}` (pl 64k, en 64k, es 40k, pt 27k, la 21k)
  have a parseable parent; e.g. Latin *mātrīx* now derives from *māter* + *-trīx*.
- **Stable links**, **deployment** via `main` → R2 → `production`
  ([deploy/README.md](deploy/README.md), untested on the host), **dates** for 14 of 20
  pilot codes.

## What changed

- The processor parses `{{etymon}}` per Module:etymon
  ([etymon.rs](crates/processor/src/etymon.rs)); where an entry also has classic
  templates, `{{etymon}}` gives the parent and the classic chain continues it.
  `<id:…>` references resolve to the item with that `id=`.
- The new graph is selected; `deploy/graph.json` names it but it is not yet in R2.
- AGENTS.md says how to run workflows from a worktree without `data/`.
- `canonical-forms` (unmerged): a "canonical" form is used only if it has as many
  words as the page title, which drops Wiktextract headword junk such as Old French
  *octobre oblique singular or* (210k of 1.97M canonical forms; all sampled junk).
  Its graph `etymology-graph/cf09bd87…` (copied into `data/`, not selected) shows
  *octobre*, *frere*, *matrice* and Sanskrit stems cleanly.

PIE descendant trees, English+Latin (nodes; extra parents in brackets):

| Root | `bf5c28d2` | `7c39072a` | `cf09bd87` | api.wety.org |
|---|---|---|---|---|
| *méh₂tēr* | 121 (21) | 179 (39) | 179 (39) | 143 (30) |
| *bʰer-* | 781 (79) | 915 (216) | 923 (214) | 946 (187) |

## Uncertainties

- *bʰer-* lost 220 earlier nodes. Of 40 sampled English/Latin ones, 24 still descend
  from it under their own-language base word (*broaden* under *broad*), which the
  descendants view does not expand; 16 no longer reach it, mostly former
  root-category shortcuts, several wrong (*Middlesex*, *mark*, *tell*).
- Root imputation onto chain tops depends on processing order, which shifts when
  terms change: *περιφέρω* and *fortūnātus* (no parsed etymology in either graph)
  hang under *bʰer-* in `7c39072a` but other roots in `cf09bd87`.
- References with length marks (`περῐ-`) miss entries titled without them (`περι-`),
  dropping the whole compound.
- PIE roots with a final hyphen in `:af` read as prefixes; family-code etymons are
  skipped; `id=` links override Descendants-derived links.
- Host deployment setup not run; client 409 reload not exercised in a browser.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)); its
   `publish-graph.sh` step uploads `7c39072a…` (600 MB).
2. Merge `canonical-forms` and select `cf09bd87…` (CURRENT.json, `deploy/graph.json`).
3. Supply the six queued works in Zotero, then `dating holdings sync`.

## Decisions needed from the maintainer

- Whether to adopt `canonical-forms`/`cf09bd87…`, and whether to report the
  `{{fro-noun}}` headword parse to Wiktextract.
