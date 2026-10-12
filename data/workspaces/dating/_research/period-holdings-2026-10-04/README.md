# Period holdings run, 2026-10-04

Third round of the period check, reading what the maintainer supplied after the
[retry](../period-retry-2026-10-04/README.md): the six requested works (Baugh & Cable,
now the 7th edition of 2026; Schmidt 2013; Clackson & Horrocks 2007; Horrocks 2010;
Sun 2006; Jamison's Sanskrit chapter, supplied as the whole volume) and five Woodard
(ed.) 2008 volumes reprinting the Cambridge Encyclopedia of the World's Ancient
Languages.

## Run

- Brief: `packages/dating/prompts/period-verification.md` (third-round version).
- Profile `codex-astra`: gpt-6-astra, high effort, bundled Codex CLI 0.160.0 on the
  ChatGPT subscription. 2700 s research + 600 s delivery; used 1882 s (completed).
- 220 tool actions: 11 holdings opened, 51 `find`, 61 `read`, 66 page renders (all
  succeeded); no web searches, acquisitions or requests. One `find` call failed on a
  missing argument.
- Codex usage, one turn: 7,577,572 input tokens (7,033,600 cached), 46,443 output
  (13,791 reasoning). No dollar cost (subscription).
- `check`: 41 assertions, 0 errors.

Run files: `report.md` (Astra's report), `assertions.jsonl`, `coordinator-review/review.json`.

## Review (Claude Opus 5.5, the session that ran the holdings sync and wrote the brief)

The four adopted quotes were re-checked on the page images: Baugh & Cable p. 46 and
the copyright page (7th edition, Routledge 2026); Schmidt p. 23 (periodization table),
p. 224 (Langobardic as an OHG dialect) and the title page (11th edition, Hirzel 2013).
Not a separate session from the brief's author; not independent corroboration.

13 of 41 assertions supported. Corrections: 12 time encodings, representation only
(century and millennium points as one-unit ranges; `within_unit` dropped from
endpoints, kept in the expression; Fortson's ranged PIE bound kept as expression, as
in the retry). `la-clackson-horrocks-continuation` (Latin continues as a learned
language) supported → candidate: an undated observation cannot be a supported dating
claim.

Adopted:

| Code | Now shown | Source | Replaces |
|---|---|---|---|
| ang | 450–1150 CE | Baugh & Cable 2026, p. 46 | Slocum & Lehmann, 5th–11th century |
| enm | 1150–1500 CE | Baugh & Cable 2026, p. 46 | ISO 639-3, 1100–1500 |
| gmh | 1050–1350 CE | Schmidt 2013, p. 23 (linguistic periods) | ISO 639-3, c. 1050–1500 |
| goh | 500–1050 CE | Schmidt 2013, p. 23; Langobardic p. 224 | Fortson, attested from c. 600 (withdrawn, now an alternative) |

ang and enm now share Baugh & Cable's 1150 boundary, goh and gmh Schmidt's 1050.
Wiktionary files Early New High German under `de`, so ISO's gmh period ran into it.
The maintainer had kept ISO for gmh over Wright 1917 while Schmidt was unread.

Not adopted: Huehnergard & Woods (akk) and Watkins (hit) agree with the adopted dates;
Clackson's seventh-century first documents (la) are coarser than Wallace's; the two new
Old Irish first attestations conflict (glosses 7th century vs manuscripts c. 8th);
Sun's Old Chinese (771 BCE – 220 CE) is narrower than `och` and his Middle Chinese
(220–960) only overlaps `ltc`. Nine existing decisions went stale with the new evidence
and were re-adopted unchanged, each with a note.

Still without a period: `non` (Faarlund dates the runic predecessor), `got` (Jasanoff
dates witnesses only), `cu`, `sa` (Jamison's Sanskrit covers all Old Indic but treats
Buddhist Hybrid Sanskrit under Middle Indic; no span), `ojp`, `ine-pro` (only a
"before the mid-5th millennium BC" bound, Hoenigswald & Woodard). No sources requested.
