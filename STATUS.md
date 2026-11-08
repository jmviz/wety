# Status

*Updated 2026-11-08. `main` carries the client work, the root fixes, the claims model
(core-1 to core-6, acd-1 to acd-3, lila-1 to lila-3, syn-1) and client-polish. Branch
`language-dating-contd` (unmerged) adds the supplied-sources dating batches, chapter
credits and chronology frames.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `15c68ef6…` live, from the 2026-09-01 dump: 10.1 million words, 9.95
  million links, 6,003 languages, References, the ACD's and de Vaan's claims. Dates
  cover 348 codes (362 on `language-dating-contd`).
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed (`language-dating-contd`)

- **Supplied sources:** the maintainer's Zotero uploads are ingested (all in
  `wety/holdings` and `wety/dating/periods`). Four Opus 5.5 batches read them: 15 codes
  gained a period (e.g. Old Uyghur, Classical Mongolian, Classical Mandaic, Old Nubian,
  Middle Russian, Wiyot, Omok, Chuvan); Jewish Babylonian Aramaic, Meroitic and
  Proto-Yukaghir were replaced.
- **Chapter credits:** a claim quoting a chapter of an edited volume credits the chapter's
  author ("Burtea 2011", not "Weninger 2011"); 118 recorded.
- **Chronology frames:** `validate` fails if a proto-language ends after its descendant;
  each family's frame names the one study its proto-languages show (Indo-European:
  Heggarty et al. 2023; Uralic: Honkola et al. 2013; Sino-Tibetan: Zhang et al. 2019;
  12 more). Proto-Germanic, Proto-Slavic, Proto-Greek, Proto-Permic (c. 760 CE) and four
  other Uralic nodes now follow their frames; nodes outside a frame say so.
- **Research harness:** Claude subscription by default (`claude-agent-sdk` 0.2.179).

## Uncertainties

- Heggarty et al.'s Proto-Indo-European (c. 6120 BCE) is the contested hybrid
  hypothesis, shown as that study's estimate with alternatives; Honkola et al.'s
  Proto-Uralic is older than the 2023 handbook's dispersal. Frames favour one coherent
  method over each node's best single estimate.
- Austroasiatic, North-East Caucasian, Uto-Aztecan, Algic, Tupian and Hmong-Mien have
  no frame (no study dates several nodes); Proto-Hmongic is undated.
- Not built: "whose view", the tree parameters, source-only items (110,240 links).
- The store snapshot of this work did not run: another session's backup held the
  repository; the data is synced into the main checkout's `data/`.

## Next three actions

1. Merge `language-dating-contd`, then `dating synthesize` in the main checkout (7
   product mirrors there are older) and a store snapshot.
2. Rework the processor to read Wiktionary's Enterprise HTML and wikitext dumps.
3. Source-only items for records with no match; whose view and the tree parameters.

## Decisions needed from the maintainer

- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
