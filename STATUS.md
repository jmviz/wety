# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2` (with acd-1 and lila-1 merged in).*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…` live, from the 2026-09-01 dump; 8,101,979 etymological
  entries. This stack adopts `a1bdab67…` (9,946,413 edges, each with its basis), which
  needs this stack's server. Dates cover 348 codes, 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs stay in `data/`, backed up
  with `wety-data store` (snapshot `a5dc7ba3`); [provenance](docs/architecture/provenance.md)
  covers recovery.

## What changed

- **Etymology claims model adopted** (shared doc "wety etymology claims model",
  outside git). Its work plan is the tracker: one branch per milestone, commits titled
  `Claims <id>: …`.
- **core-1, acd-1, lila-1 done:** every graph edge records its basis; ACD v2.0 and
  LiLa's de Vaan data are retained with their curation files (`etc/`, tracked in git).
- **core-2 done:** the assertion schema v0
  ([packages/etymology/schema](packages/etymology/schema/README.md)): sources, agents,
  works, extractions, records, hypotheses, cognate sets, alternative sets and
  assertions, validated per line and across lines. Converters turn 30 Wiktionary
  entries, 24 ACD groups and 20 LiLa etymologies into assertions; all validate. Eight
  fields only one source needs are extensions. Results: each source's `checks/core-2/`.
- Checks pass: the catalogs, dating, workflow_tools, source_tools and 28
  `packages/etymology` tests.

## Uncertainties

- `/cognates` and `/descendants` changed shape on the client branch: deploy server,
  client and graph `a1bdab67` together.
- The Wiktionary sample's claims are one model's annotation, unreviewed; the processor
  replaces it for templates in core-4.
- `{{etymon}}` "or" groups: the template's documentation is ambiguous, and only 3 of 5
  drawn are alternatives, so the 1,764 counted is an upper bound (core-3 must tell).
- Language-map and pair reviews are model reviews. A form key alone pairs ACD
  homographs wrongly (4 of 24 drawn); gloss and reflexes must weigh in (acd-3).

## Next three actions

1. acd-2 and lila-2: full converters on the schema, with the ACD's compiled links.
2. core-3: several etymologies per word, checking how `{{etymon}}` groups read.
3. Maintainer: review schema v0 and the stack; ship server, client and graph together.

## Decisions needed from the maintainer

- Whether to ship the client work before the claims work lands.
- Whether a reported rejection ("denied by De Vaan") enters the core schema now, or
  stays a Wiktionary extension until a second source needs it.
