# Status

*Updated 2026-10-04. Branch `stable-links`, from local `main` (which has
`wiktionary-refresh` fast-forwarded in); nothing pushed.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Rebuilt graph** (`etymology-graph/bf5c28d2…`, 2026-09-01 dump, selected):
  homographs are separate items again (`en lime` has its five etymologies) and
  Descendants sections are read again. Checked in a browser.
- **Stable links:** URLs name a word, e.g. `/cognates/en/lime?ety=3&lang=la`, and
  survive rebuilds. `/resolve` finds the word; a link naming several etymologies
  without a number offers a chooser; a line under the view bar switches between
  them. Numeric IDs stay inside a session and carry the graph version; a stale
  tab reloads (409). Language codes replace numeric language IDs throughout.
- **Dates:** 14 of 20 pilot codes have a cited period.

## What changed

- Processor reads string etymology numbers (`"3"`, `"1.1"`) and nested
  descendants; imputed duplicates are numbered by their citing item.
- Core/server: `keys` (resolution), `Lang::from_code` with retired codes
  (`data/registry/retired-language-codes.json`, empty until the next language
  refresh), `/resolve`, codes in the API. Client rebuilt around word links.

## Uncertainties

- Trees are still smaller than production in places (PIE *méh₂tēr, English+Latin:
  142 nodes vs 173; *bʰer-: 831 vs 1,079; *pénkʷe: 249 vs 186). About 380k entries
  (64k English, 22k Latin) give their etymology only in `{{ety}}`/`{{etymon}}`,
  which the processor does not parse, e.g. Latin *mātrīx*.
- The client's reload on 409 was not exercised in a browser (the server's 409 was).
- Term matching is exact apart from Unicode normalization and page titles;
  Wiktionary renumbering can still move a link to a sibling homograph.

## Next three actions

1. Parse `{{ety}}` and `{{etymon}}` in the processor; rebuild and compare with
   production.
2. Maintainer: deploy the server with the rebuilt graph, then merge (the client
   needs `/resolve`; see `crates/server/deployment.md`).
3. Supply the six queued works in Zotero, then `dating holdings sync`.

## Decisions needed from the maintainer

- Deploy now, or after action 1 (some trees are smaller than production's).
