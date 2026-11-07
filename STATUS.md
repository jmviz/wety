# Status

*Updated 2026-11-07. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2` (acd-1, lila-1, core-2, acd-2, lila-2), then
`claude/claims-core-3`, then `claude/claims-core-4` (core-5 merged in), then
`claude/claims-core-6` (acd-3 and lila-3 merged in), then `claude/claims-syn-1`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump. The stack adopts `15c68ef6…`:
  10.1 million words, 9.95 million links, 6,003 languages, each entry's References, and
  the ACD's and de Vaan's claims; it needs the stack's server and client.
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
- The server, client and graph change shape together: ship them in one release.

## Next three actions

1. Release: publish graph `15c68ef6…` (`deploy/publish-graph.sh`), commit
   `deploy/graph.json`, merge the stack into `main`.
2. Rework the processor to read Wiktionary's Enterprise HTML and wikitext dumps
   directly; syn-2 (the model tier, prose citations) folds into it.
3. Source-only items for records with no match; whose view and the tree parameters.

## Decisions needed from the maintainer

- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
- Whether to ship the client work before the claims work lands.
