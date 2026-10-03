"""Parsing and converting dates into the harness's calendar convention.

All years are astronomical integers (year 0 = 1 BCE; 500 BCE = -499).

    bce(500)            -> -499
    from_bp(2450, 2000) -> -450         (years before a stated present)
    parse_era("c. 450 – 1150")          -> interval
    parse_era("8th century BC")         -> interval with precision 'century'
"""

from __future__ import annotations

import re
from dataclasses import dataclass, asdict


def bce(n: int) -> int:
    """Historical n BCE -> astronomical year."""
    return 1 - n


def from_bp(years_bp: float, present: int | None = None) -> int:
    if present is None:
        raise ValueError("BP conversion requires an explicit epoch")
    return int(round(present - years_bp))


def historical_to_astronomical(y: int) -> int:
    """Convert a 'historical' negative year (-100 = 100 BCE, no year 0) to astronomical."""
    return y + 1 if y < 0 else y


@dataclass
class Interval:
    earliest: int | None
    latest: int | None
    precision: str  # year | decade | century | millennium | unknown
    raw: str
    note: str | None = None

    def to_dict(self) -> dict:
        return {k: v for k, v in asdict(self).items() if v is not None}


_ORD = r"(\d{1,2})(?:st|nd|rd|th)"
_BC = r"\b(?:BCE?|B\.C\.E?\.?)(?!\w)"
_AD = r"\b(?:CE|AD|A\.D\.)(?!\w)"


def _century_bounds(n: int, bc: bool) -> tuple[int, int]:
    if bc:
        # 8th century BC = 800–701 BCE
        return bce(n * 100), bce((n - 1) * 100 + 1)
    return (n - 1) * 100 + 1, n * 100


def _millennium_bounds(n: int, bc: bool) -> tuple[int, int]:
    if bc:
        return bce(n * 1000), bce((n - 1) * 1000 + 1)
    return (n - 1) * 1000 + 1, n * 1000


def parse_point(s: str, default_bc: bool = False) -> tuple[int, int, str] | None:
    """Parse one date expression -> (earliest, latest, precision), or None."""
    t = s.strip().replace("–", "-").replace("—", "-")
    t = re.sub(r"^(c\.|ca\.|circa|about|approx\.?|around|~)\s*", "", t, flags=re.I).strip()
    m = re.fullmatch(
        rf"{_ORD}\s*(?:-|to)\s*{_ORD}\s+(century|centuries|millennium|millennia)\s*({_BC}|{_AD})?",
        t,
        re.I,
    )
    if m:
        a, b = int(m.group(1)), int(m.group(2))
        bc = bool(m.group(4) and re.match(_BC, m.group(4), re.I)) or (not m.group(4) and default_bc)
        f = _millennium_bounds if m.group(3).lower().startswith("mill") else _century_bounds
        lo, hi = f(a, bc), f(b, bc)
        return (
            min(lo[0], hi[0]),
            max(lo[1], hi[1]),
            "century" if f is _century_bounds else "millennium",
        )
    m = re.fullmatch(
        rf"(?:early|late|mid|mid-|middle)?\s*{_ORD}\s+(century|millennium)\s*({_BC}|{_AD})?",
        t,
        re.I,
    )
    if m:
        n = int(m.group(1))
        bc = bool(m.group(3) and re.match(_BC, m.group(3), re.I)) or (not m.group(3) and default_bc)
        lo, hi = (_millennium_bounds if m.group(2).lower() == "millennium" else _century_bounds)(
            n, bc
        )
        return lo, hi, "millennium" if m.group(2).lower() == "millennium" else "century"
    m = re.fullmatch(
        rf"{_ORD}\s*({_BC}|{_AD})", t, re.I
    )  # "3rd BCE": a bare ordinal with an era = century
    if m:
        lo, hi = _century_bounds(int(m.group(1)), bool(re.match(_BC, m.group(2), re.I)))
        return lo, hi, "century"
    m = re.fullmatch(rf"(\d{{1,4}})s?\s*({_BC}|{_AD})?", t, re.I)
    if m:
        y = int(m.group(1))
        bc = bool(m.group(2) and re.match(_BC, m.group(2), re.I)) or (not m.group(2) and default_bc)
        y = bce(y) if bc else y
        if t.rstrip(" BCEAD.").endswith("s"):
            return y, y + 9, "decade"
        return y, y, "year"
    m = re.fullmatch(r"(\d+(?:\.\d+)?)\s*(kya|ka|kyr|thousand years ago)", t, re.I)
    if m:
        return None  # no explicit epoch
    m = re.fullmatch(r"(\d+)\s*(?:years?\s*)?(?:BP|before present|years ago|ya)", t, re.I)
    if m:
        return None  # no explicit epoch
    return None


_WORD_ORD = {
    "first": "1st",
    "second": "2nd",
    "third": "3rd",
    "fourth": "4th",
    "fifth": "5th",
    "sixth": "6th",
    "seventh": "7th",
    "eighth": "8th",
    "ninth": "9th",
    "tenth": "10th",
    "eleventh": "11th",
    "twelfth": "12th",
}
_MONTHS = r"(?:January|February|March|April|May|June|July|August|September|October|November|December|Jan|Feb|Mar|Apr|Jun|Jul|Aug|Sep|Sept|Oct|Nov|Dec)\.?"


def clean_era(text: str) -> str:
    """Strip wikitext and prose from an infobox era/extinct field, leaving the date expression."""
    t = text
    t = re.sub(r"<ref[^>]*/>", " ", t)
    t = re.sub(r"<ref[^>]*>.*?</ref>", " ", t, flags=re.S)
    t = re.sub(r"<!--.*?-->", " ", t, flags=re.S)
    t = re.sub(r"<[^>]+>", " ", t)
    t = re.sub(
        r"\{\{\s*(?:circa|c\.|ca\.)\s*\|\s*([^{}|]*)\|\s*([^{}|=]*)(?:\|[^{}]*)?\}\}",
        r"c. \1 - \2",
        t,
        flags=re.I,
    )  # {{circa|1600|1200 BC}}
    t = re.sub(
        r"\{\{\s*(?:circa|c\.|ca\.)\s*\|\s*([^{}|]*)(?:\|[^{}]*)?\}\}", r"c. \1", t, flags=re.I
    )
    for _ in range(3):
        t = re.sub(r"\{\{[^{}]*\}\}", " ", t)
    t = re.sub(r"\[\[([^\]|]*\|)?([^\]]*)\]\]", r"\2", t)
    t = re.sub(r"'{2,}", "", t)
    t = (
        t.replace("\u2013", "-")
        .replace("\u2014", "-")
        .replace("\u2212", "-")
        .replace("\u00a0", " ")
    )
    t = re.sub(r"\bc\.\s*", "", t, flags=re.I)
    t = re.sub(r",?\s*with the death of.*$", "", t, flags=re.I | re.S)
    t = re.sub(r"\s*[;|].*$", "", t, flags=re.S)  # second clause / stray params
    t = re.sub(r"\((?![^)]*\d{3})[^)]*\)", " ", t)  # parentheticals without a year
    t = re.sub(r"\(([^)]*)\)", r" \1 ", t)  # parentheticals with a year: keep text
    t = re.sub(r"\b(\d{1,3}),(\d{3})\b", r"\1\2", t)  # 1,000 -> 1000
    t = re.sub(rf"\b(\d{{1,2}})\s+{_MONTHS}\s+(\d{{3,4}})\b", r"\2", t)  # 6 August 1995 -> 1995
    t = re.sub(
        rf"\b{_MONTHS}\s+\d{{1,2}},?\s+(\d{{3,4}})\b", r"\1", t
    )  # September 20, 2025 -> 2025
    t = re.sub(rf"\b{_MONTHS}\s+(\d{{3,4}})\b", r"\1", t)
    t = re.sub(
        r"\b(first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth|eleventh|twelfth)\b",
        lambda m: _WORD_ORD[m.group(1).lower()],
        t,
        flags=re.I,
    )
    t = re.sub(
        r"\b(?:the|end of|beginning of|start of|turn of the|possibly|probably|perhaps|approximately|about|around|roughly)\b",
        " ",
        t,
        flags=re.I,
    )
    t = re.sub(r"\b(cent|c)\.", "century", t)
    t = re.sub(r"\s+", " ", t).strip(" .,?:")
    return t


def parse_era(text: str) -> Interval | None:
    """Parse a Wikipedia-style era / extinct string into an interval. Conservative:
    returns None when the text is not clearly a date or date range."""
    raw = text
    # Multi-stage prose and qualified estimates need semantic review, not truncation.
    if ";" in text or re.search(
        r"\b(possibly|probably|perhaps|early|late|mid|circa|about|around|roughly)\b|\bc\.",
        text,
        re.I,
    ):
        return None
    t = clean_era(text)
    if (
        not t
        or re.search(r"\b(unknown|unattested|liturgical|no longer)\b", t, re.I)
        and not re.search(r"\d", t)
    ):
        return None
    present = bool(re.search(r"\b(present|today|current|now|ongoing)\b", t, re.I))
    # "between X and Y"
    m = re.match(r"^between\s+(.+?)\s+and\s+(.+)$", t, re.I)
    if m:
        t = f"{m.group(1)} - {m.group(2)}"
    # bounds: "after X" -> earliest; "by / before / until X" -> latest
    m = re.match(r"^(after|since|from)\s+(.+)$", t, re.I)
    if m and not re.search(r"\s(-|to|until|till|through)\s", m.group(2)):
        p = parse_point(m.group(2))
        return Interval(p[0], None, p[2], raw, note="lower bound only") if p else None
    m = re.match(r"^(by|before|until|till)\s+(.+)$", t, re.I)
    if m:
        p = parse_point(m.group(2))
        return Interval(None, p[1], p[2], raw, note="upper bound only") if p else None
    # range
    m = re.match(
        r"^(?:attested\s+)?(?:from\s+)?(.+?)\s*(?:-|\bto\b|\buntil\b|\btill\b|\bthrough\b)\s*(.+)$",
        t,
        re.I,
    )
    if m and not re.search(r"\d\s*-\s*\d\s*-", t):
        left, right = m.group(1), m.group(2)
        ab = re.fullmatch(r"(\d{2})(s?)", right.strip())
        lm = re.search(r"(\d{2})\d{2}s?$", left.strip())
        if ab and lm:
            right = lm.group(1) + ab.group(1) + ab.group(2)
        r = (
            None
            if present and re.search(r"\b(present|today|current|now|ongoing)\b", right, re.I)
            else parse_point(right)
        )
        l_bc = bool(r and r[0] < 0 and not re.search(_AD, left, re.I))
        l = parse_point(left, default_bc=l_bc)
        if l and (r or present):
            iv = Interval(
                l[0], r[1] if r else None, min(l[2], r[2], key=_prec_rank) if r else l[2], raw
            )
            if iv.latest is not None and iv.earliest > iv.latest:
                return None
            return iv
    p = parse_point(re.sub(r"^(attested|from|since|by|after|before)\s+", "", t, flags=re.I))
    if p:
        return Interval(
            p[0], None if present else p[1], p[2], raw, note="open-ended" if present else None
        )
    # last resort: drop a trailing parenthetical ("Late 19th century (possibly around 1890)")
    if "(" in text:
        iv = parse_era(re.sub(r"\s*\(.*$", "", text, flags=re.S))
        if iv:
            iv.raw = raw
            return iv
    return None


def _prec_rank(p: str) -> int:
    return {"year": 0, "decade": 1, "century": 2, "millennium": 3, "unknown": 4}[p]


def fmt_year(y: int | None) -> str:
    if y is None:
        return "…"
    return f"{1 - y} BCE" if y <= 0 else f"{y} CE"
