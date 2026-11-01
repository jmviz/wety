# Status

*Updated 2026-10-31. Branch stack, none deployed: `codex/client-ux-audit` (client work),
then `claude/root-vote-fix` (root fixes), then `claude/claims-core-1`, then
`claude/claims-acd-1`, then `claude/claims-lila-1`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Language dates show an interval, its basis
and citations from reviewed evidence in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org; only the maintainer merges/deploys `main` and
  `production` ([deploy/README.md](deploy/README.md)).
- **Graph:** `dab06f67…`, the 2026-09-01 dump; 8,101,979 etymological entries and
  9,946,921 edges. Dates cover 348 codes, including 135 estimated proto-language ends.
- **Research store:** private evidence, artifacts and runs stay in `data/`, backed up
  with `wety-data store`; [provenance](docs/architecture/provenance.md) covers recovery.

## What changed

- **Etymology claims model adopted** (shared doc "wety etymology claims model",
  outside git). Its work plan is the tracker: one branch per milestone, commits titled
  `Claims <id>: …`.
- **core-1 (code done, rebuild running):** every graph edge records its basis. The
  graph format changed: old graph files no longer load.
- **acd-1 (check passed):** ACD v2.0 (Zenodo 14608975) retained, with its source
  profile, kind map and language map in `data/workspaces/etymology/sources/acd/etc/`,
  which git now tracks for every source, by the maintainer's decision.
  920 of 1,022 ACD varieties (90.0%) and 15 of 42 proto-languages have an exact
  Wiktionary code; 20 ACD sets are paired with Wiktionary reconstructions. v2.0 sets
  none of its `Doubt` flags.
- **lila-1 (check passed):** LiLa's de Vaan data (Zenodo 4147500, the latest)
  retained: all 1,437 etymologies, as on the live endpoint. De Vaan's and
  Wiktionary's PIE and Proto-Italic spellings get comparison keys from orthography
  profiles: all 20 same-reconstruction pairs in a sample share a key, and 9 of 9 held
  out once one rule for Proto-Italic fricatives was added (6 before). LiLa encodes
  no certainty beyond 185 "?" forms, and no credits.
- Checks pass: 43 processor, 15 core, 7 server and 16 new `packages/etymology` tests.

## Uncertainties

- `/cognates` and `/descendants` changed shape on the client branch: deploy server and
  client together, with a rebuilt graph.
- How many imputed roots the root fixes change is unknown until the rebuild.
- Language-map and pair reviews are model reviews. A form key alone pairs ACD
  homographs wrongly (4 of 24 drawn); gloss and reflexes must weigh in (acd-3).
- The store snapshot does not yet hold the ACD and LiLa data; it waits for the rebuild.

## Next three actions

1. Finish the core-1 rebuild; diff its `graph-edges` dump against `dab06f67`'s (only
   `root` and `root_vote` edges may differ); count edges by basis. Then snapshot the
   store and commit `data/SNAPSHOT.json`.
2. core-2: the assertion schema, drafted against 30 Wiktionary entries, the 20 ACD
   sets and the 20 LiLa etymologies now in the store.
3. acd-2 and lila-2: the converters, once core-2 lands.

## Decisions needed from the maintainer

- `claude/claims-lila-1` is cut from `claude/claims-acd-1`, which starts
  `packages/etymology`, rather than from core-1.
- Whether to ship the client work before the claims work lands.
