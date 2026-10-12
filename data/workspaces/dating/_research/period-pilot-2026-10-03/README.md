# Period pilot run, 2026-10-03

One bounded Astra run checking display periods for the 20 pilot codes listed in
STATUS at the time (`ang enm goh gmh non sga got cu akk hit la grc sa ltc och ojp
gem-pro ine-pro sla-pro cel-pro`).

## Run

- Brief: `packages/dating/prompts/period-verification.md`, via
  `harness.frontier_research prepare --brief`.
- Profile `codex-astra`: gpt-6-astra, high effort, Codex CLI 0.160.0 on the ChatGPT
  subscription (no API key, no paid fallback).
- Limits, raised for this run through an alternate `DATING_LLM_CONFIG`: 16 searches,
  30 acquisitions, 260 tool actions, 2700 s research plus 600 s delivery.
- Used: 1017 s active time (completed); 13 searches, 30 acquisitions (16 failed:
  publisher 403/404, blocked PDFs), 250 tool actions (183 reads: paging the 2.4 MB
  Glottolog CSV took most of them).
- Codex usage, one turn: 2,094,702 input tokens (1,947,776 cached), 22,430 output tokens
  (5,475 reasoning). No dollar cost (subscription).
- `check`: 39 assertions, 0 structural errors.

Run files: `report.md` (Astra's report), `assertions.jsonl` (Astra's
proposals), `coordinator-review/review.json` (the review below), with the retained
sources, events and tool logs, all in the research store.

## Astra's findings

Of the 30 existing candidate claims for these codes, 2 were confirmed, 22 corrected
and 6 unverifiable. All six handbook claims (Baugh & Cable, Schmidt, Clackson &
Horrocks, Horrocks) were unverifiable: no open text. Their recorded quotes had been
written from memory. Corrections: every Glottolog value is a database documentation
window, not a conventional period. Akkadian's −2350 and Old Japanese's 712 are
last-documentation fields, not attestations. Several Wikipedia periods are narrower
than the Wiktionary code (Vedic only, eighth century only). Astra proposed 7 defaults.

## Review (Claude Opus 5.5, separate session)

Every adopted quote was re-checked against the retained text at its offsets. Each
was unique in its text, with locators matching. Context was read for the Koch
passages and the CSV header. Corrections to Astra's output:

1. `enm`, `ltc`: mapping unresolved → exact. Wiktionary itself classes Early Scots as
   Middle English. Glottolog's Middle Chinese is the whole stage that Wiktionary `ltc`
   covers.
2. `iso_sga`, `iso_grc`: "to 900" / "to 1453" re-encoded from a deadline bound to a
   period with a stated end and no start.
3. `koch_gem_inception_alternative`: role → secondary report (Koch reporting Mallory),
   so candidate, not supported.
4. `cldf_akk`: supported → candidate. A 2350 BCE last documentation contradicts
   Akkadian's first-millennium texts; likely a Glottolog data error.
5. Representation only, numbers unchanged: mixed-era and uncertain endpoints
   (`och`, `la`, `gem-pro`) use the registry's new start/end endpoint form; Gothic
   century points become one-century ranges. Precision of the Koch period: century.
6. `cldf_got` stays supported but is not adopted. 350–380 covers Wulfila's Bible only,
   which is misleading as an attestation window.

Adopted (11 codes): `ang enm gmh` (ISO 639-3 conventional periods), `sga grc` (ISO
end only), `hit och ltc` (Glottolog documentation windows, shown as attestation),
`la` (first written record, Koch 2020), `gem-pro cel-pro` (Koch 2020 estimates). The
review was a separate model session, not independent corroboration.
