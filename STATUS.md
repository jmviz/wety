# Status

*Updated 2026-11-03. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2` (acd-1, lila-1, core-2, acd-2, lila-2), then
`claude/claims-core-3`, then `claude/claims-core-4` (core-5 merged in), then
`claude/claims-core-6` (acd-3 and lila-3 merged in).*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump. The stack adopts `3da0f0c9…`:
  10.1 million words and 0.8 million cited-only ones, 9.95 million links, 6,003
  languages, with each entry's References; it needs the stack's server and client.
  Dates cover 348 codes, 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **core-6 (Wiktionary's part):** the chart draws alternatives as "or" branches, dashes
  uncertain links and marks rule-inferred modes; word details list each origin with who
  asserts it and its credits (`/origin/:id`).
- **acd-3, lila-3 (matchers v4):** ACD and LiLa records matched to wety's items, weighing
  keys, glosses, shared reflexes and ancestry (schema v3), with learnt weights and
  reviewers' judgements deciding. ACD: 2,216 reconstructions and 15,713 attested forms
  matched; held-out target precision 0.962. LiLa: 1,380 Latin entries, 964 Proto-Italic
  and 832 PIE etymons; held-out precision 0.925, reviewed sample 112 of 118. Of 1,202
  claims crediting the ACD, 355 agree with it and 21 differ. Schema v4 adds the relay
  difference `form` (one parent, reconstructed otherwise). Checks:
  `data/workspaces/etymology/sources/{acd,lila-edl}/checks/{acd,lila}-3-v4/`.

## Uncertainties

- core-6 is not finished: "disputed", "whose view", the `view`/`floor` tree parameters,
  "implied by the ACD's tree" and "possibly related" in Cognates wait for syn-1, which
  brings the matched sources into the build.
- wiktextract drops most footnotes (38 credits to de Vaan in the graph, about 3,257
  sections citing him in the dump) and keeps no position for any: parsing the Wiktionary
  HTML dump directly, a project the maintainer envisions, would recover both.
- acd-2 left out 12,629 ACD reconstructions that are in no cognate set.
- *lupus* keeps its Descendants reading: its own section names a language but no word
  ("possibly borrowed from an Osco-Umbrian language"), which wety cannot represent yet.
- `/cognates` and `/descendants` changed shape on the client branch: ship server, client
  and graph together.

## Next three actions

1. syn-1: synthesis tiers 3–4 with both sources in the build; the generation report.
2. Finish core-6 on top: disputed, whose view, implied links.
3. syn-2: model extraction of prose citations and conjectures.

## Decisions needed from the maintainer

- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
- Whether to ship the client work before the claims work lands.
