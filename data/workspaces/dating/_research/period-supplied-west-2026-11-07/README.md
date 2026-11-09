# Supplied sources: Europe, the Americas, Australia and proto-languages, 2026-11-07

[Batch brief](../briefs/period-supplied-west-2026-11-07.md).

## Run

- Profile `claude-opus` (claude-opus-5-5, high effort, Claude Agent SDK on Claude subscription).
  2700 s + 600 s; used 917 s (completed).
  112 tool actions. Usage attempt 2: 8,582,444 input tokens (8,297,376 cached), 105,506 output, 111 turns.
  `check`: 38 assertions, 0 errors. Requests: The Uralic Languages (2nd edition): branch chapters; Additional Turkic and Tungusic borrowings into Yukaghir.

## Review (Claude Opus 5.5 (Claude Code))

Quotes exact in the retained texts. 12 of 38 supported; 5 codes adopted.

Rechecks 25 codes from Europe, the Americas, Australia and the proto-languages against
the works the maintainer supplied in Zotero in November 2026 ([batch brief](../briefs/period-supplied-west-2026-11-07.md)).
The first dispatch of this batch (`period-supplied-west-protos-2026-11-07`) failed on the
old bundled Claude Code; this is its rerun after the SDK upgrade.

Adopted: Middle Russian 15th–17th c. (Hill 1977, "roughly speaking" the grammarians'
Middle Russian; page image checked); Wiyot 1851–1962 (Golla: Gibbs's vocabularies, the
last fluent speaker's death); Klamath-Modoc from 1841 to no fluent speakers by 2006
(Golla); Wappo from the early 1850s (Golla gives no year for its last fluent speaker, so
the 1990 lead is unconfirmed); Mobilian Jargon from 1699 (Drechsel; a pidgin, no native
transmission).

Recorded beside the current defaults, not adopted: Janhunen's "not earlier than the
thirteenth century" for the end of Proto-Mongolic (against Häkkinen 2009's c. 1150 CE);
Klimov's glottochronological splits of Kartvelian (by the late 3rd millennium BC) and
Georgian-Zan (early 1st millennium BC), against Gavashelishvili et al. 2023; Bartens's
Proto-Permic breakup in the 700s CE and period lasting past the mid-1st millennium AD,
which contradicts the displayed Parpola 2017 period (c. 800–200 BCE). Corrections: analysis
ids added to the divergence estimates; Wappo's 1960–1985 documentation of its last
speaker kept as context, not a period.

Nothing usable: Proto-Yukaghir (the supplied file is part VI of Piispanen's series, 2022,
not the 2018 article), Proto-Philippines (Blust gives no absolute date), New Latin
(Moul's c. 1400–1700 is the book's focus), Proto-Nakh (Nichols gives none), Anguthimri,
Wailaki, Southern Ohlone (variety-level only), Galatian (reports only). The supplied The
Uralic Languages PDF holds only the front matter, chapter 1 and the bibliography, so the
Uralic proto-stages stay as they were; both requests are open again.

Known defect: a first publish imported Bartens's "continued past the middle of the 1st
millennium AD" (a-urj-prm-01) as supported, encoded as an open period bound; it displays
as an alternative "from c. 1000 CE" for urj-prm-pro. The review now holds it as a
candidate, but the append-only registry keeps the first import
(`ev_period_f07a5df89b04e0807dadcc6992011de8d3915e1f`); removing it needs the maintainer.
