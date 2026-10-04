# Medieval Europe batch, 2026-10-04

Batch run from the generic period brief
([batch brief](../briefs/period-medieval-europe-2026-10-04.md)): 15 medieval European
codes from `dating period-targets`, read in the supplied handbooks (Barnes, Schmidt,
Fortson, Kapović, Clackson & Horrocks, Baugh & Cable) and open sources; the researcher
could request handbooks.

## Run

- Profile `codex-astra` (gpt-6-astra, high effort, Codex CLI 0.160.0, ChatGPT
  subscription). 2700 s research + 600 s delivery; used 2321 s (completed). Ran in
  parallel with the literary-languages batch.
- 269 tool actions: 6 holdings opened, 46 searches, 53 acquisitions (15 failed), 41
  `find`, 62 `read`, 17 page renders, 4 requests. Usage: 8,518,314 input tokens
  (7,897,728 cached), 50,762 output (15,385 reasoning). `check`: 23 assertions, 0 errors.

## Review (Claude Opus 5.5)

Adopted quotes are exact in the retained texts. 17 of 23 assertions supported; 11 codes
adopted. Corrections: three slashed periods (Schmidt's Old Saxon and Middle Low German,
Breitbarth's Middle Dutch "1150–1500/1600") encoded as endpoint ranges and supported;
Old Occitan (an incidental "11th–16th centuries" in a handwriting-recognition paper)
supported → candidate.

Adopted: Old Norse c. 750–1350 (Barnes, the broad sense, which is Wiktionary's `non`
under rules 2); Old Dutch c. 700–1150, Middle Dutch 1150–1500/1600, Old Saxon c.
800–1200, Middle Low German 1200–1650 (Breitbarth; Schmidt, Pijnenburg, Quak and
Olbertz alternatives); Old French 9th–13th c. and Middle French 14th–15th c.
(provisional: Regnault et al. 2019, in passing; Rickard 1989 requested); Old Frisian
recorded c. 1250–1550 (Bremmer); Middle Irish 10th–13th c. and Middle Welsh mid-12th –
14th c. (Fortson); Middle Armenian from its oldest text, 1037 (Budak & Pifer).

Not adopted: Old Spanish, Old Occitan, Medieval Latin (requested or scope only), Old
East Slavic (an abstract's "5th–16th c."; the Ostromir Gospel is a manuscript date).
