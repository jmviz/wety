# Period retry run, 2026-10-04

The pilot's period check repeated with the handbooks the maintainer supplied as holdings
(Barnes, Fortson, Frellesvig, Huehnergard, Kapović, Lunt, Miller; `dating holdings
list`). It is the first run with the `library`, `open` and `request` tool actions.

## Run

- Brief: `packages/dating/prompts/period-verification.md` (2026-10-04 version):
  the nine codes without a period, and better sources for the eleven with one.
- Profile `codex-astra`: gpt-6-astra, high effort, bundled Codex CLI 0.160.0 on the
  ChatGPT subscription, cached web search for discovery. 2700 s research + 600 s
  delivery; used 2628 s (completed).
- 242 tool actions: `library` once, all seven holdings opened, 38 `find`, 52 `read`;
  43 searches, 45 acquisitions (18 failed: 8 not found, 5 bot challenges, 2 access
  denied, others network); 6 requests. Semantic Scholar answered 8 searches with 429.
- Codex usage, one turn: 5,999,292 input tokens (5,555,072 cached), 46,733 output
  (12,236 reasoning). No dollar cost (subscription).
- `check`: 55 assertions, 0 errors.
- Tool faults found during the run (fixed afterwards, commit `dca9beb` and the commit
  with this README): `page` refused holdings whose text came partly from OCR (15 page
  renders failed, for Huehnergard, Fortson, Kapović and Miller); `check` and `read`
  compared quotes against newline-translated text, which hid a CRLF mismatch in one
  quote (Wright).

Tracked: `report.md` (Astra's report), `assertions.jsonl`, `coordinator-review/review.json`.
The six requests were merged into `data/registry/source-requests.json`.

## Review (Claude Opus 5.5, the session that built the holdings route)

Every adopted and supported quote was re-checked: build-packet located each in its
retained text, and the cited pages of Huehnergard (scanned), Fortson, and Kapović were
viewed as page images, since Astra's page checks failed. The review was not a
separate session from the implementation, and is not independent corroboration.

Identity: Huehnergard's file is the 3rd edition, Eisenbrauns 2011 (2nd printing 2015),
not Brill 2018 as its Zotero record says; Kapović's is the 2nd edition of 2017
(paperback 2019). Fortson's file is a reflowed e-book without printed pages; it is cited
by section.

12 of 55 assertions supported; the rest stay candidates (subsets of the code, scope
evidence, or unresolved mapping). Corrections: `sla-pro-fortson-end` re-encoded as a
period end ranging AD 800–1000 (Astra's encoding would have read as a span from 800
to 1000); two representation-only fixes (Barnes endpoints; Fortson's range-valued PIE
bound kept as expression); scope claims quoted once per catalog target now get one
claim each (build-packet keyed claims without the target).

Adopted:

| Code | Now shown | Source | Replaces |
|---|---|---|---|
| akk | attested 26th c. BCE – 1st c. CE | Huehnergard 2011, pp. xxiii–xxiv | nothing (new) |
| goh | attested from c. 600 CE | Fortson 2010 §15.70 (OHG includes Langobardic, §15.71) | nothing (new) |
| sla-pro | estimated, until c. 800/1000 CE | Fortson 2010 §18.29 | nothing (new) |
| ang | 5th–11th century CE | Slocum & Lehmann, EIEOL Old English | ISO 639-3 c. 450–1100 |
| la | attested from mid-7th c. BCE | Wallace in Kapović 2017, p. 322 | Koch 2020 c. 550 BC |

| hit | attested c. 19th century – 1200 BCE | Kapović 2017 p. 3 (names), Fortson 2010 §9.17 (end of text production) | Glottolog window 1650–1180 BCE (withdrawn) |

`hit` was added by an amendment to the review after the maintainer approved decision
withdrawal (the review's `withdraw` list). Supported but not adopted: Wright 1917 for
gmh (1100–1500; maintainer agreed to keep ISO), Sims-Williams for sga (eighth-century
manuscripts; the end-only ISO period stays). Six decisions went stale with the new
evidence and were re-adopted after review, each with a note.

Incident: the pilot reproduction test called the live `adopt`, and a test run after
this import silently reverted `ang` and `la` to the pilot's choices and dropped the
re-adoption notes; commits `2489e6a` and `2a4919c` recorded that reverted state. The
test no longer writes, and the decisions were restored in the next commit.

Still without a period: `non` (Barnes's broad Old Norse includes the colonies, so is
broader than `non`), `got` (witness dates only), `cu` (Lunt's OCS canon excludes
Middle Bulgarian), `sa` (no whole-code span; Buddhist Hybrid Sanskrit scope open),
`ojp` (Frellesvig dates OJ, EMJ, LMJ separately), `ine-pro` (a bound only).
Catalog scope for `cu` and `sa` stays unrecorded: the handbooks define the stage
names, not the ISO or Glottolog entities.
