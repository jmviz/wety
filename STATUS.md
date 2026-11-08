# Status

*Updated 2026-11-08. `main` carries the client work, the root fixes, the claims model
(core-1 to core-6, acd-1 to acd-3, lila-1 to lila-3, syn-1) and client-polish. Branch
`wiktionary-entries` (unmerged) carries x-1 and x-2 of the rework below.*

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

- **Reading Wiktionary directly (x-1, x-2; plan "Read Wiktionary's HTML and wikitext
  dumps directly" in the project's plans):**
  - Acquired the 2026-11-01 Wikimedia Enterprise snapshot of en.wiktionary (21.99 GB,
    9.9 million pages, HTML and wikitext of each revision; `source_tools.enterprise`)
    and the pages-articles dump of the same date. Enterprise has no Reconstruction
    namespace: `source_tools.wiktionary_html` fetches those 46,395 pages' HTML by
    revision, within Wikimedia's rate limits.
  - `crates/wiktionary-entries` turns the snapshot into 11.47 million entries in 13
    minutes: each etymology's prose with its templates and footnotes at their
    offsets, the footnotes' bodies, `{{ety}}` trees, link targets (entry names),
    mentions, senses, headwords and descendants. Nothing reads them yet.
  - Against Kaikki's wiktextract on unchanged pages: entries 99.86 %, parts of speech
    98.2 %, first glosses 98.7 %, canonical forms 97.2 %, descendant terms 97.6 %,
    etymology calls 99.5 % (counting mentions written by wrapper templates). Report:
    `data/workspaces/etymology/sources/wiktionary/checks/x-2/result.json`.

## Uncertainties

- Not built: "whose view", the `view`/`floor` tree parameters, "possibly related" in
  Cognates, source-only items for unmatched records (110,240 links dropped). Wiktionary's
  `{{der}}` along a line of descent stays "derived".
- The graph still reads wiktextract, which drops most footnotes (38 credits to de Vaan,
  about 3,257 sections citing him) and every position; the entries keep both.
- A wrapper template's own calls (`{{ko-etym-sino}}` → `{{etymid}}`) reach the entries
  only as mentions.
- The language export must come from the snapshot's dump: 1,873 headings name
  languages added since 2026-09-01.
- acd-2 left out 12,629 ACD reconstructions that are in no cognate set.
- Deeper default Cognates roots make large trees (tabula: 76,217 words unfiltered).

## Next three actions

1. x-4: the processor reads the entries instead of wiktextract (design in the plan);
   graph diff against `15c68ef6…`.
2. x-5: footnote credits with position and References from the same extraction;
   then syn-2 (model reading of prose) on the entries' spans.
3. Source-only items for records with no match.

## Decisions needed from the maintainer

- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
