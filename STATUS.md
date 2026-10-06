# Status

*Updated 2026-10-06. Local `main` has `canonical-forms` and `processor-review`; graph
`dab06f67…` is selected (CURRENT.json, `deploy/graph.json`). Nothing pushed.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Graph** `dab06f67…` (2026-09-01 dump): 8,101,979 entries with an etymology
  (English 1,059,987), against 7,894,425 in `cf09bd87…` (`canonical-forms` alone).
  Stable links; deployment via `main` → R2 → `production` (host setup not yet run;
  the graph is not yet uploaded); dates for 14 of 20 pilot codes.

## What changed (`processor-review`)

- Entry names from Wiktionary's Lua; `{{af}}` parts; deterministic root votes;
  one placeholder per missing term; cycles broken at their weakest link.
- **Homographs** ([resolve.rs](crates/processor/src/resolve.rs)): ids, then agreement
  (Descendants/Derived lists, the citation's next step), then contradiction, then meaning.
- **Meaning** ([embeddings.rs](crates/processor/src/embeddings.rs)): Granite small
  English R2 (48M parameters; replaces MiniLM), comparing senses one by one, using the
  template's `t=` gloss, an "alternative form of" sense's gloss, and part of speech.
  Offline, on 2,000 ambiguous citations with a known answer: 72.9% → 84.6% right.
  Larger models that fit in 16 GB (EmbeddingGemma 300M, Qwen3 0.6B, Granite R2 149M)
  scored lower here and were 3–11× slower.
- **Every homograph on the cited page** is a candidate: Middle Dutch *dië* (on page
  *die*) had been invisible.
- **Missing homographs** (the meant word lacks an entry): 159 citations, down from 274;
  only an entry's own etymology counts against it, spelling variants do not, and a
  cited root does not.

Server check, two samples of 100 ambiguous citations with known answers:

| Answer known by | `aeeabc6d` | `deb28401` | `dab06f67` |
|---|---|---|---|
| id | 92 right, 6 wrong | 96 / 1 | 94 / 1 |
| the candidate's Descendants list | 86 / 3 | 89 / 1 | 87 / 1 |

The rest have intermediate-language parents. Trees, English+Latin nodes:
*méh₂tēr* 184 (production 143), *bʰer-* 1,121 (946). *bairn* < *bern* "child" <
Old English *bearn* again (it had followed *barn* "barn" in `abf64f81…`).

## Uncertainties

- "Missing" verdicts: 6 of 8 sampled right; false ones are related words that no
  template links (*szenzitív* < *sensitif* < *sentire*).
- 566k citations are decided by meaning alone; offline accuracy is about 85%.
- Wrong Wiktionary-side links persist (French *genre* < English *gender*; Tatar *бүре*
  "wolf" under *bürgä* "flea" via a Descendants list).
- Builds take 25 minutes with a warm embedding cache, 3–4 hours cold.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md));
   `deploy/publish-graph.sh` uploads `dab06f67…` before `main` is pushed.
2. Supply the six queued works in Zotero, then `dating holdings sync`.
3. Review the remaining false "missing" verdicts and cross-language misparents.

## Decisions needed from the maintainer

- Code edits no longer make the catalog stale (`e5c5221`). Dating still fingerprints
  `workflow_tools` and the IE-CoR receipt names `iecor.py`; dropping both needs a dating
  republish, best done by the active dating branch. Proceed?
