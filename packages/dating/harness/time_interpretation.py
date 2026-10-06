"""Explicit interpretations of native dates; never rewrites historical evidence."""

import re
from typing import Literal
from pydantic import BaseModel, ConfigDict, model_validator

BP_UNITS = ("BP", "ka_BP", "century_BP")
AGE_UNITS = (*BP_UNITS, "years_ago", "ka_ago")
PERIOD_UNITS = ("century_BCE", "century_CE", "millennium_BCE", "millennium_CE")


class BPConvention(BaseModel):
    """A labeled reading of an age without a stated epoch as years before AD 1950: BP
    notation by convention, and (maintainer decision 2026-10-06) rough expert estimates
    stated only as "years ago", where the epoch moves the date by decades at most."""

    model_config = ConfigDict(extra="forbid")
    basis: Literal["conventional_bp_1950", "conventional_years_ago_1950"]
    notation: Literal["BP", "B.P.", "YBP", "cal BP", "ka BP", "century BP", "years ago", "ka ago"]
    epoch: Literal[1950] = 1950

    @model_validator(mode="after")
    def notation_matches_basis(self):
        relative = self.notation in ("years ago", "ka ago")
        if relative != (self.basis == "conventional_years_ago_1950"):
            raise ValueError("convention basis and notation disagree")
        return self


class UnresolvedCalendar(ValueError):
    """Valid native evidence without an authorized calendar interpretation."""


def calendar_epoch(raw):
    explicit = raw.get("present")
    interpretation = raw.get("epoch_interpretation")
    if interpretation:
        convention = BPConvention.model_validate(interpretation)
        units = (
            ("years_ago", "ka_ago")
            if convention.basis == "conventional_years_ago_1950"
            else BP_UNITS
        )
        if raw.get("unit") not in units:
            raise ValueError("BP interpretation requires a BP unit")
        if explicit is not None:
            raise ValueError(
                "keep source-explicit epoch separate; omit conventional interpretation"
            )
        return convention.epoch
    return explicit


def check_age_scale(raw):
    scale = raw.get("age_scale")
    if scale == "radiocarbon_uncalibrated":
        raise UnresolvedCalendar(
            "uncalibrated radiocarbon quantities require calibration, regardless of epoch notation"
        )
    if raw.get("unit") not in AGE_UNITS:
        if raw.get("epoch_interpretation"):
            raise ValueError("BP interpretation requires a BP unit")
        return
    if scale not in (None, "calendar", "radiocarbon_uncalibrated", "unknown"):
        raise ValueError("unknown age scale")
    if scale == "unknown":
        raise UnresolvedCalendar(
            "BP calendar conversion requires calendar age scale; radiocarbon calibration is not subtraction"
        )
    if raw.get("epoch_interpretation") and scale != "calendar":
        raise UnresolvedCalendar(
            "conventional BP interpretation also requires a justified calendar age scale"
        )
    # Missing metadata in old payloads is preserved, not retroactively endorsed.


def period_bounds(value, unit):
    if unit not in PERIOD_UNITS or value <= 0 or int(value) != value:
        raise ValueError("calendar period requires a positive integer ordinal and explicit era")
    width = 1000 if unit.startswith("millennium") else 100
    n = int(value)
    return (
        (1 - n * width, -(n - 1) * width)
        if unit.endswith("BCE")
        else ((n - 1) * width + 1, n * width)
    )


_ORDINALS = {
    "first": 1,
    "second": 2,
    "third": 3,
    "fourth": 4,
    "fifth": 5,
    "sixth": 6,
    "seventh": 7,
    "eighth": 8,
    "ninth": 9,
    "tenth": 10,
    "eleventh": 11,
    "twelfth": 12,
}
_PERIOD = re.compile(
    r"\b(?P<n>\d+(?:st|nd|rd|th)|"
    + "|".join(_ORDINALS)
    + r")\s+(?P<period>century|millennium)\s+(?P<era>B\.?C\.?E?\.?|C\.?E\.?|A\.?D\.?)",
    re.I,
)


def quote_periods(quote):
    values = []
    for m in _PERIOD.finditer(quote):
        n = m["n"].lower()
        number = _ORDINALS[n] if n in _ORDINALS else int(re.match(r"\d+", n)[0])
        era = "BCE" if m["era"].upper().startswith("B") else "CE"
        values.append((number, m["period"].lower() + "_" + era))
    return values


def native_numbers_match(native, quote):
    """Literal numbers or typed ordinals, never arbitrary model-expanded endpoints."""
    if native.get("epoch_interpretation") and not re.search(
        r"\bB\.?\s*P\.?(?!\w)|\bbefore present\b", quote, re.I
    ):
        return False
    # Publishers use thin/narrow/no-break spaces as thousands separators. Do
    # not join newlines: adjacent table cells must remain separate quantities.
    numbers = {
        float(re.sub(r"[, \u00a0\u2009\u202f]", "", n))
        for n in re.findall(r"(?<![\w.])\d+(?:[, \u00a0\u2009\u202f]\d{3})*(?:\.\d+)?(?!\w)", quote)
    }
    periods = quote_periods(quote)
    unit = native.get("unit")
    values = [
        native[k]
        for k in (
            "value",
            "mean",
            "median",
            "lower",
            "upper",
            "location",
            "standard_deviation",
            "support_lower",
            "support_upper",
        )
        if native.get(k) is not None
    ]
    if native.get("form") in ("elapsed_duration", "relative_offset") and native.get("value") == 1:
        singular = {"years": "year", "centuries": "century", "millennia": "millennium"}.get(unit)
        if singular and re.search(r"\b(?:a|an|one)\s+" + singular + r"\b", quote, re.I):
            numbers.add(1)
    if unit in PERIOD_UNITS:
        return all((v, unit) in periods for v in values)
    # Catch an ordinal copied into a year field, even though its numeral occurs.
    if unit in ("BCE", "CE") and any(
        (v, p) in periods for v in values for p in PERIOD_UNITS if p.endswith("_" + unit)
    ):
        return False
    return all(v in numbers for v in values)
