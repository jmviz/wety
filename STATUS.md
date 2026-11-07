# Status

*Updated 2026-11-07. Released: `main` and `production` at 75546f07 carry the client work,
the root fixes and the claims model (core-1 to core-6, acd-1 to acd-3, lila-1 to lila-3,
syn-1).*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `15c68ef6…` live, from the 2026-09-01 dump: 10.1 million words, 9.95
  million links, 6,003 languages, each entry's References, and the ACD's and de Vaan's
  claims.
  Dates cover 348 codes, 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **syn-1:** the graph build reads the ACD's and LiLa's claims through three new
  receipted workflows (`etymology_assertions`, `acd_claims`, `lila_claims`). 11,476
  words carry claims: 1,248 read by consensus, 5 by preference, 152 unsettled (disputed,
  awaiting syn-2's model tier); review 53 of 55 right. Word details show other sources'
  claims and how the primary was chosen; the chart badges disputed words. LiLa gives
  every link the type inheritance, so schema v5 marks its mode uniform and nothing
  compares a mode against it. Report: `data/runs/aa966f3bd0dc4142915995f2fb7462dd/checks/syn-1/`.
- **acd-3, lila-3 (matchers v4):** 17,929 ACD and 3,176 LiLa records matched to wety's
  items; held-out precision 0.962 (ACD) and 0.925 (LiLa).

## Uncertainties

- Not built: "whose view", the `view`/`floor` tree parameters, "possibly related" in
  Cognates, source-only items for unmatched records (110,240 links dropped). The ACD adds
  only compiled credits: none of its stated borrowings is fully matched. Wiktionary's
  `{{der}}` along a line of descent stays "derived": it mostly marks learned borrowings,
  names and new formations, so ancestry alone does not make it inherited.
- wiktextract drops most footnotes (38 credits to de Vaan in the graph, about 3,257
  sections citing him in the dump) and keeps no position for any: parsing the Wiktionary
  HTML dump directly, the planned rework, would recover both.
- acd-2 left out 12,629 ACD reconstructions that are in no cognate set.
- An origin naming a language but no word ("possibly from an Osco-Umbrian language",
  *lupus*) cannot be represented yet.

## Next three actions

1. Rework the processor to read Wiktionary's Enterprise HTML and wikitext dumps
   directly; syn-2 (the model tier, prose citations) folds into it.
2. Source-only items for records with no match.
3. Whose view and the tree parameters (core-6's rest).

## Decisions needed from the maintainer

- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
