# Status

*Updated 2026-11-08. `main` carries the client work, the root fixes, the claims model
(core-1 to core-6, acd-1 to acd-3, lila-1 to lila-3, syn-1) and client-polish. Branch
`language-dating-contd` (unmerged) adds the supplied-sources dating batches.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `15c68ef6…` live, from the 2026-09-01 dump: 10.1 million words, 9.95
  million links, 6,003 languages, References, the ACD's and de Vaan's claims. Dates
  cover 348 codes (361 on `language-dating-contd`), 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **Supplied sources (dating):** `dating holdings sync` ingested the maintainer's Zotero
  uploads (66 holdings; 47 requests supplied). Two Opus 5.5 batches read them: 13 codes
  gained a period (Old Uyghur, Old Turkic, Classical Mongolian, Pumpokol, Classical
  Mandaic, Old Nubian, Samaritan Aramaic, Christian Palestinian Aramaic, Middle Russian,
  Wiyot, Wappo, Klamath-Modoc, Mobilian Jargon). Yug gained its end (1972). Jewish
  Babylonian Aramaic (Sokoloff) and Meroitic (Rilly & de Voogt's body text) were
  replaced. Runs: `data/workspaces/dating/_research/period-supplied-{asia,west}-2026-11-07/`.
- **Research runs use the Claude subscription:** `claude-opus` (Opus 5.5, WebSearch) is
  the default profile; `claude-agent-sdk` 0.2.179; `run-readme` reads Claude usage.
- **client-polish** (merged): no page blanking on hover, deeper default Cognates roots,
  "via LiLa ↗" claim links, an about page.

## Uncertainties

- Not built: "whose view", the `view`/`floor` tree parameters, "possibly related" in
  Cognates, source-only items for unmatched records (110,240 links dropped).
- wiktextract drops most footnotes and their positions; parsing the HTML dump directly
  would recover them.
- acd-2 left out 12,629 ACD reconstructions that are in no cognate set.
- Proto-Mongolic, Kartvelian and Proto-Permic now have conflicting published dates
  beside their defaults. Bartens's Proto-Permic breakup (700s CE) contradicts the
  displayed Parpola period (c. 800–200 BCE).

## Next three actions

1. Rework the processor to read Wiktionary's Enterprise HTML and wikitext dumps
   directly; syn-2 (the model tier, prose citations) folds into it.
2. Source-only items for records with no match.
3. Whose view and the tree parameters (core-6's rest).

## Decisions needed from the maintainer

- Two supplied files do not answer their requests (reopened): the Piispanen PDF is part
  VI (2022), not the 2018 article; The Uralic Languages PDF holds only chapter 1. Blake's
  Woiwurrung chapter still has no file.
- Whether to delete the superseded first import of Bartens's Proto-Permic end
  (`ev_period_f07a5df8…`), which gives urj-prm-pro a wrong alternative "from c. 1000 CE".
- Which Proto-Permic period to show (Parpola or Bartens).
- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
