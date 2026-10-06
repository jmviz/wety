# Source dates and calendar interpretation

BP conventionally means years before AD 1950. This is useful information, not a
reason to classify every unstated epoch as unknowable. Preserve three distinct
things: the original notation and quantity; an epoch explicitly specified by the
source; and an interpretation justified by a convention. A source-specific epoch
(including AD 2000 in some pilot analyses) takes precedence over the convention.
Publication year, retrieval year and the numerical age itself are not epochs.

There is a separate issue: **age scale**. An uncalibrated radiocarbon age in BP
cannot be converted to a calendar date by subtracting it from 1950. Calibrated
`cal BP` can; calendar/model years before a stated epoch can. An archaeological
association may quote a radiocarbon determination, a calibrated chronology or an
unspecified secondary date. Preserve that distinction and the calibration method,
curve/version, uncertainty and reservoir correction when supplied. This harness
does not perform radiocarbon calibration. See [Oxford's calibration explanation](https://c14.arch.ox.ac.uk/calibration.html)
and [NIST's account of radiocarbon standards](https://pmc.ncbi.nlm.nih.gov/articles/PMC4853109/).

For new extraction, retain the source-stated epoch separately from a labeled
conventional interpretation. A conventional 1950 interpretation is appropriate
for explicit BP notation without a contradictory source definition. Bare “years
ago”, `ka` or `kya` need contextual interpretation; do not silently assign them
1950, 2000 or the current year. Unknown age scale still prevents a calendar
conversion even when the BP epoch is interpretable. Preserve the native claim
and a specific gap instead of discarding it.

A source's own calibration that equates a calendar date with an age sets its epoch
(maintainer decision, 2026-10-06): Kolipakam et al. 2018 put Tamil's 254 BCE at 2250
years ago (AD 2000); Lee & Hasegawa 2011 put 1603 at 407 YBP (AD 2010). This takes
precedence over the 1950 convention. It needs an unambiguous equation: Greenhill et
al. 2023 tie their 445 BP tip to both 1555 and 1571, so the convention applies there.
The manifest records the equation as `present_evidence`.

The transport now supports `years_ago` and `ka_ago` separately from BP units.
These preserve native quantities with an unknown epoch. Typed `century_BCE`,
`century_CE`, `millennium_BCE` and `millennium_CE` retain the source ordinal;
the program expands periods and bounds. Literal ordinal matching supports
expressions such as “third millennium BCE” without accepting arbitrary generated
years. A “by” bound uses the period's latest endpoint; “before” uses its earliest.

Existing source payloads and claim identities must not be rewritten to add this
interpretation. A reviewed new interpretation needs separate provenance and, if
imported as evidence, a new assertion linked to the original. Historical records
predating explicit age-scale fields retain their original limitations; absence
of the new field is not certification that radiocarbon calibration was checked.

The first full-source evaluation was frozen under the earlier contract, which
requested only a source-explicit epoch. Its report therefore distinguishes
contract compliance from scientific plausibility: choosing the conventional 1950
is not intrinsically fabrication. Putting the age (for example 5,000) into the
epoch field is a different, substantive error. The user's BP reference arrived
during reference review, before extraction predictions were inspected; the
interpretive amendment is retained with the frozen reference. Original model
prompts and outputs remain unchanged.

Two further representational issues surfaced in those sources. An elapsed
duration such as “300 years longer” is not a calendar span from year 0 to year
300. A century or millennium expression is not a point date; when expanded into
calendar bounds, the expansion needs a reproducible derivation, not a requirement
that the resulting four-digit years occur literally in the quote. Typed ordinal
periods now address that latter issue. Elapsed durations and untyped dispersion
still need dedicated representations; preserve their wording and a structured
gap/relative constraint rather than fabricating numerical precision.
