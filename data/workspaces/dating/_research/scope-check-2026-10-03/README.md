# Scope check run, 2026-10-03

One bounded Astra run settling how Wiktionary `cu`, `sa`, `la` and `non` relate in
scope to their ISO 639-3 and Glottolog entries (for the dating display: may a period
stated for the catalog entity be shown for the whole Wiktionary code?). It also
re-checked in current Wiktionary the filings behind earlier catalog decisions.

## Run

- Brief: `packages/dating/prompts/scope-verification.md` (`prepare --brief`).
- Profile `codex-astra`: gpt-6-astra, high effort, Codex CLI on the ChatGPT
  subscription. Default limits: 1200 s research + 300 s delivery, actions uncapped.
- Used: 1077 s (completed); 4 searches, 44 acquisitions (4 failed: three guessed
  Wiktionary URLs, one publisher 403), 238 tool actions. The new `find` action
  served 103 lookups; there was no paging of the Glottolog table.
- Codex usage, one turn: 2,535,627 input tokens (2,385,920 cached), 24,141 output
  (6,024 reasoning). `check`: 86 assertions, 0 structural errors.

Run files: `report.md` (Astra's report), `assertions.jsonl` (86 quoted pieces of scope
evidence), `coordinator-review/review.json` (the review below).

## Result and review (Claude Opus 5.5, separate session)

Astra proposed two directional relations and left six unresolved. Both proposals'
quotes were re-checked against retained text (exact, unique). The Glottolog
classification paths and the Old Gutnish placement were checked as well.

- `la` → Glottolog `lati1261`: **Wiktionary broader**, recorded in the catalog.
  Wiktionary files Old Latin under `la`; Glottolog's Latin is a separate language
  from Old Latin.
- `non` → Glottolog `oldn1244`: Astra said Wiktionary narrower (Glottolog's Old Norse
  includes Old Norn). **Correction: not recorded.** Astra judged current Wiktionary,
  but the catalog is pinned to the legacy export, where Old Gutnish is filed under
  `non`. Glottolog does not place Old Gutnish under Old Norse, so against the
  snapshot the relation is narrower or overlap, unknown which.
- `cu` (both targets), `sa` (both), `la`/ISO, `non`/ISO: unresolved. Nothing recorded;
  the catalog lookups stay unadjudicated. The missing evidence is a catalog
  denotation for Middle Bulgarian and the later Church Slavonic recensions, for Buddhist
  Hybrid and New Sanskrit, and for ISO's Latin and Old Norse boundaries.

Filing checks: Early Scots (now `enm-esc`), Lombardic, Middle Japanese and
Koine/Medieval Greek are still filed under `enm`, `goh`, `ojp` and `grc`, so the
earlier catalog decisions stand. Current Wiktionary has drifted from the legacy
export in other ways: Old Gutnish is now its own code, and `sa-cls`/`sa-ved` became
`cls`/`vsn`.

Effect on wety: no period changed. The `la` decision went stale with its mapping and
was re-adopted after review (Koch's first written Latin is Old Latin, inside
Wiktionary `la`).
