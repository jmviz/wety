# Status

*Updated 2026-10-05. Local `main` has `etymon-templates` (graph `7c39072a…` selected).
Branch `processor-review` (on `canonical-forms`) is unmerged; its graph `deb28401…`
is built, validated and copied into `data/`, not selected. Nothing pushed.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Graph** `7c39072a…` (2026-09-01 dump): homographs, Descendants sections and
  `{{etymon}}` are read. Stable links; deployment via `main` → R2 → `production`
  (host setup not yet run); dates for 14 of 20 pilot codes.
- **Graph** `deb28401…` (`processor-review`): 8,102,248 entries with an etymology
  (English 1,060,015), against 7,894,425 in `cf09bd87…` (`canonical-forms`).

## What changed on `processor-review`

- Cited terms are found by Wiktionary's own entry names (new export
  `wiktionary-entry-names/d582e4fa…`); `{{af}}` parts read correctly; roots imputed
  deterministically, by vote, with homograph roots by sense id; missing parents and
  parts are linked, not dropped; one placeholder per missing term.
- **Homographs** ([resolve.rs](crates/processor/src/resolve.rs)): ids (`{{etymon}}`,
  `{{etymid}}`, sense ids), then agreement (the candidate's Descendants or Derived
  terms name the citing word, or its etymology fits the citation's next step), then
  contradiction, then the template's `t=` gloss, then gloss embeddings.
- **Missing homographs:** when every candidate's own etymology names an unrelated
  source, the cited word is taken as missing (278 citations).
- Cycles are broken at their weakest link; an etymology opening with `{{unk}}`/`{{unc}}`
  takes no parent from the conjectures after it; joined Descendants sections of
  several parts of speech were being lost.

Homograph choice on two samples of 100 ambiguous citations whose answer is known:

| Answer known by | `aeeabc6d` | `deb28401` |
|---|---|---|
| id | 92 right, 6 wrong | 96 right, 1 wrong |
| the candidate's Descendants list | 86 right, 3 wrong | 89 right, 1 wrong |

Trees, English+Latin nodes: *méh₂tēr* 183 (production 143), *bʰer-* 1,144 (946).
English *bairn* now reaches *bʰer-* through Middle English *bern* "child".

## Uncertainties

- "Missing" verdicts: about two-thirds right in a sample of 15; false ones are
  related words with unlinked entries (*Alemania* < *Alamanni*).
- The second sample's answer key is a signal the resolver uses; samples are small.
- 549k citations are still decided by gloss embeddings; part-of-speech hints are unused.
- Speculative etymologies after `{{unk}}` (e.g. *Madrid* < *mātrīx*) are no longer
  linked, by design.

## Next three actions

1. Maintainer: review `canonical-forms` and `processor-review`; if accepted, merge and
   select `deb28401…` (CURRENT.json, `deploy/graph.json`).
2. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)).
3. Supply the six queued works in Zotero, then `dating holdings sync`.

## Decisions needed from the maintainer

- Whether to adopt `deb28401…`, including the unknown-origin and missing-homograph
  policies.
