# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2` (acd-1, lila-1, core-2, acd-2, lila-2), then
`claude/claims-core-3`, then `claude/claims-core-5`. core-4 is in progress on its own
branch.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump. The stack adopts `68f378ee…`
  (core-3), which needs the stack's server. Dates cover 348 codes, 135 estimated
  proto-language ends.
- **Research store:** private evidence, artifacts and runs in `data/`, backed up with
  `wety-data store`; [provenance](docs/architecture/provenance.md).

## What changed

- **core-5 done:** Wiktionary's References sections are read from the 2026-09-01 pages
  dump (`etymology.convert.wiktionary_references`). 478,230 of the 2,807,017 entries
  with an etymology (17.0%) have References citing a work, which gives 992,583
  entry-scope credits. 669,660 (23.9%) have a References section at all. The
  bibliography holds 19,699 works: 9,862 from `Template:R:` pages, read from the
  citation template each calls (7,081 with authors), and 9,338 from `{{cite-...}}`
  lines. It validates with 0 errors.
- The credits wait in a table, one row per entry with its record id, checked against
  the schema's `Credit`. core-4 will copy them onto the assertions each entry endorses.
  No schema change: one extension, `wiktionary: work.template`.
- Credits per scholar, beside a recount of inline mentions that matches the claims
  doc's (Vasmer 550/545, Kroonen 492/490): regional dictionaries are mostly cited in
  References (Turner 5,809, Vasmer 2,225, Mayrhofer 1,289). PIE-family ones are cited
  mostly in `<ref>` footnotes: Kroonen in 3,386 language sections against 191 References
  credits, de Vaan 3,257 against 818. The check is in
  `data/workspaces/etymology/sources/wiktionary/checks/core-5/` (`summary.md`).

## Uncertainties

- Most References lines cite dictionaries of meaning (Gaffiot, Lewis and Short, Webster
  1913). An entry-scope `follows` says only that the editors consulted them.
- Unexplained: 2,807,017 entries with an etymology here, 2,980,681 in the claims doc. The
  two counting rules agree on a 400,000-line sample.
- One person can be two agents across spellings or scripts (Мудрак, Mudrak). 433
  References sections match no wiktextract entry.
- Not read yet: `{{unc}}` prose conjectures (syn-2), a mid-chain `{{unk}}`, alternatives
  inside an inline etymology. `/cognates` and `/descendants` changed shape on the
  client branch: ship the server, client and graph together.

## Next three actions

1. core-4: the processor emits Wiktionary's assertions with footnote credits, and copies
   in core-5's entry credits. Most etymologists' credits are footnotes, and core-4
   should check whether wiktextract keeps their templates.
2. Maintainer: review the stack, lila-2's readings in `etc/forms.json`, and the decisions
   below.
3. acd-3 and lila-3, after core-4.

## Decisions needed from the maintainer

- Whether alternative sets may hold records, so the ACD's form-level doublets fit.
- Whether wety restores LiLa's 7 unlinked etymons from their "Etymology for" comments.
- Whether to ship the client work before the claims work lands.
