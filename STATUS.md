# Status

*Updated 2026-11-09. `main` carries the client work, the root fixes, the claims model
and client-polish. Branch `wiktionary-entries` (unmerged) carries the rework below
(x-1 to x-4, graph adopted) and has merged `language-dating-contd`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `15c68ef6…` live, from the 2026-09-01 dump. Dates cover 362 codes on this
  branch (348 live).
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **Reading Wiktionary directly (plan "Read Wiktionary's HTML and wikitext dumps
  directly"):** sources of 2026-11-01 (Wikimedia Enterprise snapshot, Reconstruction
  pages by revision, pages-articles dump); `crates/wiktionary-entries` writes 11.5
  million entries with prose, templates and footnotes at their offsets; the graph build
  reads them instead of Kaikki's wiktextract. Branch graph `546e8406…`: 10.44M links
  (+4.9 %), 95.2 % of the live graph's kept; claims rematched (14,065 words carry them).
  `{{CJKV}}` Sino-Xenic readings give no link. Report:
  `data/workspaces/etymology/sources/wiktionary/checks/x-4/result.json`.
- **Dating (`language-dating-contd`):** the maintainer's supplied sources read in four
  batches (15 codes gained a period); claims quoting a chapter credit its author;
  chronology frames: a proto-language may not end after its descendant, and each
  family's frame names the study its proto-languages follow (Indo-European: Heggarty
  et al. 2023; Uralic: Honkola et al. 2013; 13 more).

## Uncertainties

- Not built: "whose view", the tree parameters, source-only items (110,240 links).
- Branch graph: words only a `{{CJKV}}` list linked to Chinese have no parent unless
  their own entry gives one; wrapper templates' own calls (`{{ko-etym-sino}}`) are not
  read yet.
- Frames favour one coherent method over each node's best estimate (Heggarty et al.'s
  Proto-Indo-European, c. 6120 BCE, is the contested hybrid hypothesis). Austroasiatic,
  North-East Caucasian, Uto-Aztecan, Algic, Tupian and Hmong-Mien have no frame.

## Next three actions

1. Select the 2026-11-01 language export, regenerate catalogs, dating and the graph
   together.
2. x-5: footnote credits with position; then syn-2 on the entries' spans.
3. Source-only items for records with no match.

## Decisions needed from the maintainer

- Whether wety shows `{{CJKV}}` Sino-Xenic readings beside the graph, never as links.
- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
