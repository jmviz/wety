# Supplied sources: Yukaghir, 2026-11-08

[Batch brief](../briefs/period-supplied-yukaghir-2026-11-08.md).

## Run

- Profile `claude-opus` (claude-opus-5-5, high effort, Claude Agent SDK on Claude subscription).
  1800 s + 600 s; used 343 s (completed).
  46 tool actions. Usage attempt 1: 2,176,821 input tokens (2,052,330 cached), 38,900 output, 47 turns.
  `check`: 8 assertions, 1 errors.

## Review (Claude Opus 5.5 (Claude Code))

Quotes exact in the retained texts. 3 of 8 supported; 3 codes adopted.

Reads Nikolaeva's A Historical Dictionary of Yukaghir (2006), supplied in November 2026
in place of the unavailable Piispanen 2018 article, and her Yukaghir chapter in Vajda
(ed.) 2024 ([batch brief](../briefs/period-supplied-yukaghir-2026-11-08.md)). The run
finished its work in 343 s; the runner labels it partial because `check` reported one
time error, corrected in review.

Adopted: Proto-Yukaghir ends with the Kolyma/Tundra split "possibly from around 2000
years ago" (Nikolaeva 2024, p. 359), read before 1950 under the labeled convention for
rough "years ago" estimates (c. 50 BCE). It is the specialist's estimate of the family's
first split and agrees with Holman et al. 2011's automated ASJP date (c. 30 BCE), now the
alternative. Omok: known only from Matjuškin's list recorded in 1821. Chuvan: first
recorded in 1821 (Matjuškin); no end is adopted, since Maydell's 1870 remark that most
Chuvans could not speak the language does not end transmission.

Corrections: the Proto-Yukaghir "years ago" had been encoded as BP with an assumed 2024
epoch (now `years_ago`, conventional 1950); Chuvan's 1821 list is a first attestation,
not its documentary window. Nikolaeva 2024 doubts that Omok and Chuvan were distinct from
other Old Yukaghir varieties (scope claim, kept).

Known limitation: the 2024 volume is one source record per holding, made for Vajda's
Yeniseian chapter in an earlier run, so wety credits Nikolaeva's chapter to "Vajda 2024"
(the volume's editor); the claim's quote and locator are hers.
