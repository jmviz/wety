# Status

*Updated 2026-11-01. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-core-2` (with acd-1 and lila-1 merged in), then `claude/claims-lila-2-f4f573` (lila-2).
acd-2 runs in parallel on `claude/claims-acd-2`.*

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
  with `wety-data store` (snapshot `c76478bb`); [provenance](docs/architecture/provenance.md)
  covers recovery.

## What changed

- **Etymology claims model adopted** (shared doc "wety etymology claims model",
  outside git). Its work plan is the tracker: one branch per milestone, commits titled
  `Claims <id>: …`.
- **core-1, acd-1, lila-1, core-2 done:** edges record their basis; ACD v2.0 and LiLa's
  de Vaan data are retained; assertion schema v0
  ([packages/etymology/schema](packages/etymology/schema/README.md)) with sample
  converters.
- **lila-2 done:** all of LiLa's de Vaan data enters as assertions, 0 errors, nothing
  lost: 1,437 etymologies (tīgnum's two as exclusive alternatives), 2,653 links, 2,860
  reconstructions and 1,453 Latin entries as records, 225 cognate links. 165
  reconstructions carry de Vaan's doubt; -cellō and calvor carry his credits to
  Schrijver and LIV. LiLa's conversion damage is read by rules or by the new curation
  file `etc/forms.json` (48 forms); every decision is in the check's `result.json`.
  Four LiLa extension fields added, none promoted. Result: `checks/lila-2/`.

## Uncertainties

- `/cognates` and `/descendants` changed shape on the client branch: deploy server,
  client and graph `a1bdab67` together.
- The Wiktionary sample's claims are one model's annotation, unreviewed.
- `{{etymon}}` "or" groups: 1,764 is an upper bound (core-3 must tell).
- LiLa encodes one chain where its curators note de Vaan gives alternatives (3
  etymons), and 7 etymons it never linked (ruō's second proposal, inquinō, -cellō
  duplicates); its 206 etymon-to-etymon cognate links have no documented meaning.
- lila-2 and acd-2 both touch `schema/extensions.json` and the package README; expect
  small merge conflicts.

## Next three actions

1. acd-2: the full ACD converter with compiled links (running).
2. core-3: several etymologies per word, checking how `{{etymon}}` groups read.
3. Maintainer: review schema v0, lila-2's readings in `etc/forms.json`, and the stack.

## Decisions needed from the maintainer

- Whether to ship the client work before the claims work lands.
- Whether a reported rejection ("denied by De Vaan") enters the core schema now, or
  stays a Wiktionary extension until a second source needs it.
- Whether wety should restore LiLa's 7 unlinked etymons as etymologies from their
  "Etymology for" comments (a wety inference, not LiLa's claim).
