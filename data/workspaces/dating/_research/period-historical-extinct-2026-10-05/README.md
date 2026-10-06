# Historical and extinct languages batch, 2026-10-05

Past-stage codes ranked by `dating period-targets`, plus extinct languages with a
recorded last native speaker, now shown as a period's end under basis "last speaker"
([batch brief](../briefs/period-historical-extinct-2026-10-05.md)).

## Run

- Profile `codex-astra` (gpt-6-astra, high effort, Codex CLI 0.160.0). 2700 s + 600 s;
  used 2047 s (completed). 284 tool actions. Usage 8,710,484 input tokens (8,149,760
  cached), 40,069 output. `check`: 23 assertions, 0 errors. One request filed:
  Woiwurrung, the Melbourne language.
- Sources: Kerslake and Johanson (The Turkic Languages, 2022), Creason and Tuite
  (Woodard 2008), Lee & Ramsey 2011, Krause & Slocum (EIEOL Old Russian), Penny 2002,
  Loprieno 1995, Brosig 2014, Krauss 2005, Durhan 2014, Schulze 2017, Parkvall &
  Jacobs 2023, Jacobs & Parkvall 2024, the Mittelniederdeutsches Handwörterbuch.

## Review (Claude Opus 5.5)

Adopted quotes are exact in the retained texts. 11 of 23 claims supported; 8 codes
adopted. Correction: Krause & Slocum's two Old Russian claims (end early 14th c., end
late 15th c.) merged into one period with an end ranging over both, as the source
reports two scholarly positions; the second claim became a candidate.

Adopted: Old Anatolian Turkish 13th–15th c. (Kerslake), Old East Slavic 10th –
14th/15th c. (Krause & Slocum), Aramaic first texts at the beginning of the 1st
millennium BCE (Creason), Old Korean 918–1592 and Early Middle Korean 918–1392
(Lee & Ramsey), Middle Mongol texts 13th–14th c. (Brosig), Sirenik until 1997
(Krauss: last speaker) and Ubykh until 1992 (Durhan: Tevfik Esenç).

Not adopted: Ottoman Turkish (only its middle and new sub-periods are dated), Old
Spanish (the literary period 12th–15th c. is a subset), Old Georgian (Tuite dates its
early and classical stages separately; joining them would be our construction), Old
Uyghur, Caucasian Albanian and Skepi Dutch (attestation samples narrower than the
code), Modern Low German (the dictionary's period covers more than `nds-de`), Berbice
Dutch (2010 is an official declaration, not a last speaker's death), Loprieno 1995's
Egyptian stages (the same author's adopted Woodard chapter stays the default).

## Revision: periodizations (`revision-periodizations.json`)

Kerslake and Tuite each give a periodization of the whole language, so a code takes
the phases its scope covers. `ota`: Middle and New Ottoman, 16th century – 1928 (Old
Ottoman is Wiktionary's `trk-oat`, an ancestor of `ota`). `oge`: Early and Classical Old
Georgian, 5th–11th c. `ka-mid`: Middle Georgian, 12th–18th c. (the Classical claim
re-targeted to the next line of the same list). Labels now print source years as
printed (1928, not 1930); only years computed from an age are rounded to the decade.
