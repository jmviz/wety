# Status

*Updated 2026-10-04. Local `main` has `etymon-templates` (graph `7c39072a…` selected).
Branch `processor-review` (on top of `canonical-forms`) is unmerged; its graph
`aeeabc6d…` is built, validated and copied into `data/`, not selected. Nothing pushed.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Graph** `7c39072a…` (2026-09-01 dump): homographs, Descendants sections and
  `{{etymon}}` etymologies are read. Stable links; deployment via `main` → R2 →
  `production` (host setup not yet run); dates for 14 of 20 pilot codes.
- **Graph** `aeeabc6d…` (`processor-review`) adds, against `cf09bd87…`
  (`canonical-forms`): 210,934 more entries with an etymology (8,105,359; English
  1,060,370, +30,828), and 0.79M instead of 1.26M placeholder nodes for missing terms.

## What changed on `processor-review`

- **Entry names:** cited terms are found as Wiktionary links them, via the page title
  its own `stripDiacritics` gives (grc *περῐ-* → *περι-*). New export
  `wiktionary-entry-names/d582e4fa…` (`source_tools.entry_names`, 302,900 titles),
  a workflow input.
- **`{{af}}` parts:** `lang1=`/`type1=` were read one part off; `grc:χρῶμα`,
  `<lang:>`, `<type:>` and `[[lemma|form]]` parts were junk terms.
- **Roots:** all roots of an entry are kept; root senses pick the homograph root by
  its `{{etymon}}` id; chain tops take the root most items above them name;
  deterministic. A hyphenated part that is a root entry is a base (*ph₂tḗr*).
- **Chains:** borrowings from missing terms and compounds with missing parts were
  dropped; now linked. An entry's own section beats a Descendants list unless the
  list is more specific or the section gives only a mention/form-of. Citations of one
  missing term share a node. Inline `<ety:>` chains cross borrowings.

English+Latin descendant trees (nodes):

| Root | `bf5c28d2` | `7c39072a` | `cf09bd87` | `aeeabc6d` | api.wety.org |
|---|---|---|---|---|---|
| *méh₂tēr* | 121 | 179 | 179 | 196 | 143 |
| *bʰer-* | 781 | 915 | 923 | 1,091 | 946 |

## Uncertainties

- Homographs are still chosen by gloss similarity where nothing else decides:
  English *bairn* goes to Middle English *bern* "man" rather than *bern* "child".
- Cycle removal drops all parents of an arbitrary item per cycle (~3,600 items).
- An entry whose first template has no parent (`{{unk}}`) may take a later one.
- Only spot checks and the two root trees were compared; no systematic accuracy sample.

## Next three actions

1. Maintainer: review `canonical-forms` and `processor-review`; if accepted, merge and
   select `aeeabc6d…` (CURRENT.json, `deploy/graph.json`).
2. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)).
3. Supply the six queued works in Zotero, then `dating holdings sync`.

## Decisions needed from the maintainer

- Whether to adopt `aeeabc6d…`, and whether to report the `{{fro-noun}}` headword
  parse to Wiktextract.
