"""Display periods for Wiktionary codes: `language_periods.json`.

Only adopted editorial decisions on supported, exact `wikt:<code>` evidence produce a
period. Other supported exact claims for the same code are listed as alternatives,
each with its own source; nothing is averaged or merged across sources. Years are
astronomical (BCE n = 1 - n) and converted only for labels. Family split estimates are
separate records and never become a language's period.
"""

import json
from pathlib import Path

from workflow_tools.paths import DATING_PRODUCT

from .evidence import endpoint_bounds
from .time_interpretation import PERIOD_UNITS, period_bounds

SCHEMA_VERSION = 1
PERIOD_EVENTS = ("conventional_period", "documentary_sampling")
ATTESTATION_EVENTS = ("first_attestation", "last_attestation")
SPLIT_EVENTS = ("crown_divergence", "stem_divergence")


def short_source(source):
    """'Baugh & Cable 2013' style label from the registered source."""
    surnames = [a.split(",")[0].strip() for a in source.get("authors") or [] if a.strip()]
    if len(surnames) > 2:
        names = surnames[0] + " et al."
    else:
        names = " & ".join(surnames)
    label = names or source.get("title") or source.get("id", "")
    year = source.get("year")
    return f"{label} {year}" if year else label


def ranges(claim):
    """(start, end) for a period claim; each is (earliest, latest) or None.

    Century expressions keep their width ("5th–12th century" starts in 401–500);
    a stated year is a degenerate range. Unstated endpoints stay None.
    """
    t = claim["time"]
    o = t.get("original") or {}
    form = t["form"]
    if form == "point":
        return (t["value"], t["value"]), None
    if form == "bound":
        point = (t["value"], t["value"])
        return (None, point) if t["relation"] in ("before", "by") else (point, None)
    if "start" in o or "end" in o:
        return endpoint_bounds(o)
    if o.get("unit") in PERIOD_UNITS and o.get("lower") is not None and o.get("upper") is not None:
        a, b = sorted((period_bounds(o["lower"], o["unit"]), period_bounds(o["upper"], o["unit"])))
        return a, b
    start = (t["lower"], t["lower"]) if t["lower"] is not None else None
    end = (t["upper"], t["upper"]) if t["upper"] is not None else None
    return start, end


def event_range(claim):
    """One uncertain event (e.g. a first attestation) as (earliest, latest)."""
    t = claim["time"]
    if t["form"] == "point":
        return (t["value"], t["value"])
    if t["form"] == "bound":
        return (None, t["value"]) if t["relation"] in ("before", "by") else (t["value"], None)
    return (t["lower"], t["upper"])


def endpoint(r):
    if r is None:
        return None
    return {"earliest": r[0], "latest": r[1]}


def era_year(y):
    return (str(1 - y), "BCE") if y <= 0 else (str(y), "CE")


def ordinal(n):
    suffix = "th" if 10 <= n % 100 <= 20 else {1: "st", 2: "nd", 3: "rd"}.get(n % 10, "th")
    return f"{n}{suffix}"


def describe(r):
    """(text, era) for an endpoint range: a year, a century, or a year span."""
    lo, hi = r
    if lo is None or hi is None:
        return era_year(hi if lo is None else lo)
    if lo == hi:
        return era_year(lo)
    if hi - lo == 99 and (lo - 1) % 100 == 0 and lo > 0:
        return ordinal(hi // 100) + " century", "CE"
    if hi - lo == 99 and hi % 100 == 0 and hi <= 0:
        return ordinal((1 - lo) // 100) + " century", "BCE"
    (a, era_a), (b, era_b) = era_year(lo), era_year(hi)
    return (f"{a}/{b}", era_a) if era_a == era_b else (f"{a} {era_a}/{b}", era_b)


def label(start, end, ongoing=False, approximate=False):
    c = "c. " if approximate else ""
    if start is None and end is None:
        return None
    if end is None:
        text, era = describe(start)
        return f"{c}{text} {era}–present" if ongoing else f"from {c}{text} {era}"
    if start is None:
        text, era = describe(end)
        return f"until {c}{text} {era}"
    (a, era_a), (b, era_b) = describe(start), describe(end)
    if era_a == era_b and "century" not in a + b:
        return f"{c}{a}–{b} {era_b}"
    if era_a == era_b:
        return f"{c}{a.removesuffix(' century')}–{b} {era_b}"
    return f"{c}{a} {era_a} – {b} {era_b}"


def basis(claims, reconstructed):
    if reconstructed or any(c["role"] in ("hypothesis", "posterior") for c in claims):
        return "estimated"
    if all(c["event"] == "conventional_period" for c in claims):
        return "conventional"
    return "attested"


def evidence_entry(claim, sources):
    source = sources.get(claim["source_id"], {})
    span = (claim.get("spans") or [{}])[0]
    return {
        "claim_id": claim["id"],
        "source": short_source(source),
        "role": claim["role"],
        "event": claim["event"],
        "citation": source.get("citation") or source.get("title"),
        "url": source.get("url"),
        "quote": span.get("quote"),
        "locator": span.get("locator"),
    }


def interval(claims):
    """spoken interval from the claims of one code: a period, or two attestation events."""
    by_event = {c["event"]: c for c in claims}
    period = next((by_event[e] for e in PERIOD_EVENTS if e in by_event), None)
    if period:
        start, end = ranges(period)
        used = [period]
        ongoing = period["time"]["ongoing"]
        approximate = bool(period["time"].get("original", {}).get("approximate"))
    else:
        first, last = (by_event.get(e) for e in ATTESTATION_EVENTS)
        used = [c for c in (first, last) if c]
        if not used:
            return None
        start = event_range(first) if first else None
        end = event_range(last) if last else None
        ongoing = False
        approximate = any(c["time"].get("original", {}).get("approximate") for c in used)
    return {
        "start": start,
        "end": end,
        "ongoing": ongoing,
        "approximate": approximate,
        "claims": used,
    }


def period_record(code, claims, alternatives, view, sources):
    reconstructed = view.get("kind") == "reconstructed"
    spoken = interval(claims)
    if spoken is None:
        return None
    b = basis(spoken["claims"], reconstructed)
    return {
        "code": code,
        "name": view.get("name"),
        "kind": "proto"
        if reconstructed
        else "living"
        if spoken["ongoing"]
        else "attested"
        if b == "attested"
        else "stage",
        "spoken": {
            "start": endpoint(spoken["start"]),
            "end": endpoint(spoken["end"]),
            "ongoing": spoken["ongoing"],
        },
        "basis": b,
        "label": label(spoken["start"], spoken["end"], spoken["ongoing"], spoken["approximate"]),
        "evidence": [evidence_entry(c, sources) for c in spoken["claims"]],
        "alternatives": [alternative(c, reconstructed, sources) for c in alternatives],
    }


def alternative(claim, reconstructed, sources):
    if claim["event"] in ATTESTATION_EVENTS:
        r = event_range(claim)
        start, end = (r, None) if claim["event"] == "first_attestation" else (None, r)
    else:
        start, end = ranges(claim)
    approximate = bool(claim["time"].get("original", {}).get("approximate"))
    return {
        **evidence_entry(claim, sources),
        "basis": basis([claim], reconstructed),
        "start": endpoint(start),
        "end": endpoint(end),
        "label": label(start, end, claim["time"]["ongoing"], approximate),
    }


def family_split(claim, sources):
    t = claim["time"]
    o = t.get("original") or {}
    return {
        "claim_id": claim["id"],
        "node": claim["subject"]["name"],
        "subject_id": claim["subject"]["id"],
        "event": claim["event"],
        "estimate": {
            k: t[k] for k in ("mean", "median", "lower", "upper", "interval_type", "probability")
        },
        "epoch": o.get("present") or (o.get("epoch_interpretation") or {}).get("epoch"),
        "analysis": claim.get("analysis_id"),
        "source": short_source(sources.get(claim["source_id"], {})),
        "role": claim["role"],
    }


def export_periods(bundle, generation=None):
    claims = {c["id"]: c for c in bundle["claims"]}
    sources = bundle["sources"]
    adopted = {}
    for d in bundle["decisions"]:
        c = claims[d["claim_id"]]
        # The registry rejects ineligible decisions at build time; re-check the
        # display preconditions so this export can never widen them.
        if (
            not d["subject_id"].startswith("wikt:")
            or c["status"] != "supported"
            or c["subject"]["id"] != d["subject_id"]
            or c["subject"]["mapping"] != "exact"
        ):
            continue
        adopted.setdefault(d["subject_id"][5:], []).append(c)
    languages = []
    for code in sorted(adopted):
        view = bundle["view"].get(code)
        if view is None:
            continue
        used = {c["id"] for c in adopted[code]}
        others = [
            claims[i]
            for i in view["evidence_ids"]
            if i not in used
            and claims[i]["status"] == "supported"
            and claims[i]["subject"]["mapping"] == "exact"
        ]
        record = period_record(code, adopted[code], others, view, sources)
        if record:
            languages.append(record)
    splits = [
        family_split(c, sources)
        for c in bundle["claims"]
        if c["status"] == "supported" and c["event"] in SPLIT_EVENTS
    ]
    return {
        "schema_version": SCHEMA_VERSION,
        "generation": generation,
        "years": "astronomical (BCE n = 1 - n); labels use BCE/CE",
        "languages": languages,
        "family_splits": splits,
    }


def write_periods(path=None):
    from .io_utils import atomic_json
    from .registry import current_bundle

    pointer = json.loads((Path(DATING_PRODUCT) / "CURRENT.json").read_text())
    result = export_periods(current_bundle(), pointer["generation"])
    path = Path(path or Path(DATING_PRODUCT) / "language_periods.json")
    atomic_json(path, result)
    return {
        "path": str(path),
        "languages": len(result["languages"]),
        "family_splits": len(result["family_splits"]),
        "generation": result["generation"],
    }
