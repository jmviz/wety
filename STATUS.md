# Status

*Updated 2026-11-08. `main` carries the client work, the root fixes, the claims model
(core-1 to core-6, acd-1 to acd-3, lila-1 to lila-3, syn-1) and client-polish. Branch
`wiktionary-entries` (unmerged) carries x-1 to x-4 of the rework below (x-4: graph adopted).*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `15c68ef6…` live, from the 2026-09-01 dump: 10.1 million words, 9.95
  million links, 6,003 languages, each entry's References, and the ACD's and de Vaan's
  claims. Dates cover 348 codes, 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **Reading Wiktionary directly (plan "Read Wiktionary's HTML and wikitext dumps
  directly" in the project's plans; x-1 to x-4, branch `wiktionary-entries`):**
  - Sources of 2026-11-01: the Wikimedia Enterprise snapshot (HTML and wikitext of 9.9
    million pages), the Reconstruction pages Enterprise lacks (46,385, fetched by
    revision), the pages-articles dump (redirects, References) and its language export.
  - `crates/wiktionary-entries` (workflow `wiktionary_entries`, 15 minutes) writes 11.5
    million entries: each etymology's prose with templates and footnotes at their
    offsets, footnote bodies, `{{ety}}` trees, mentions with their entry names, senses,
    headwords, descendants.
  - The graph build reads the entries instead of Kaikki's wiktextract. Graph
    `546e8406…` (adopted on the branch, not deployed): 10.44M links (+4.9 %), 95.2 % of
    the live graph's kept; 384k form-of links, 88k lone `{{m}}` etymologies wiktextract
    skipped, more immediate parents. Other sources' claims, rematched: 14,065 words
    carry them (11,485). `{{CJKV}}` lists (Sino-Xenic readings) give no link: they are
    not derivation. Report:
    `data/workspaces/etymology/sources/wiktionary/checks/x-4/result.json`.

## Uncertainties

- Not built: "whose view", the `view`/`floor` tree parameters, "possibly related" in
  Cognates, source-only items for unmatched records (110,240 links dropped).
- Branch graph: words that only a `{{CJKV}}` list linked to Chinese (mostly Korean,
  Vietnamese, Japanese) have no parent unless their own entry gives one; wrapper
  templates' own calls (`{{ko-etym-sino}}`) are not read yet.
- Footnote credits are read with their offsets but not yet credited with them (x-5).
- acd-2 left out 12,629 ACD reconstructions that are in no cognate set.

## Next three actions

1. Deploy the branch graph once reviewed (maintainer), with or without the 2026-11-01
   language table.
2. x-5: footnote credits with position and References from the entries; then syn-2
   (model reading of prose) on the entries' spans.
3. Source-only items for records with no match.

## Decisions needed from the maintainer

- Whether wety shows `{{CJKV}}` Sino-Xenic readings beside the graph, never as links
  (plan, "Sino-Xenic readings").
- Whether `data/registry/inputs.json` selects the 2026-11-01 language export
  (`wiktionary-export/57d16abe…`: 12 languages added, `aga` retired). It also feeds the
  catalogs and the dating harness, so it means a catalog rebuild.
- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
