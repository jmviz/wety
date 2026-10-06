"""Display periods for Wiktionary codes: `language_periods.json`.

Only adopted editorial decisions on supported, exact `wikt:<code>` evidence produce a
period. Other supported exact claims for the same code are listed as alternatives,
each with its own source; nothing is averaged or merged across sources. Years are
astronomical (BCE n = 1 - n) and converted only for labels. Family split estimates are
separate records and never become a language's period. A composition window (texts
transmitted orally, known from much later copies) is shown, basis "composition", only
when no period or attestation is adopted. An adopted end of native transmission (the
last native speakers) is the end of an attestation window, basis "last_speaker".

A code with Wiktionary varieties (etymology-only codes such as Early Middle Japanese
under Old Japanese) also gets a `span`: from the earliest start to the latest end of
its own period and its varieties' periods, listing each component. The span is
derived here, never stored as evidence, and shown only beside a code's own period: a
span cannot tell a living parent (Georgian) from a historical one, so it never stands
in for a missing period. `partial` marks a span with undated varieties or unstated
endpoints, which may be wider than shown.
"""

import json
import re
from pathlib import Path

from workflow_tools.paths import DATING_PRODUCT

from .evidence import endpoint_bounds
from .registry import decision_claim_error, is_proto_language
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


def crown_end(claim):
    """A proto-language ends when its sampled descendants split: the crown age's
    interval (or its central value) is the uncertain end; no start is implied."""
    t = claim["time"]
    if t["form"] != "distribution":
        return event_range(claim)
    if t.get("lower") is not None and t.get("upper") is not None:
        return (t["lower"], t["upper"])
    mid = t.get("median") if t.get("median") is not None else t.get("mean")
    return (mid, mid)


def endpoint(r):
    if r is None:
        return None
    return {"earliest": r[0], "latest": r[1]}


def era_year(y, rounded=False):
    year, era = (1 - y, "BCE") if y <= 0 else (y, "CE")
    if rounded and year >= 1000:
        # "c. 4051 BCE" from 6000 BP is exact arithmetic but false precision.
        year = round(year, -1)
    return str(year), era


def counted_back(claim):
    """Whether a claim's years are computed from an age (BP, years ago, a crown age),
    so that labels round them, rather than printed by the source (Ottoman to 1928)."""
    if claim["event"] == "crown_divergence":
        return True
    return bool(
        re.search(r'"unit": "[^"]*(BP|ago)"', json.dumps(claim["time"].get("original", {})))
    )


def ordinal(n):
    suffix = "th" if 10 <= n % 100 <= 20 else {1: "st", 2: "nd", 3: "rd"}.get(n % 10, "th")
    return f"{n}{suffix}"


def describe(r, rounded=False):
    """(text, era) for an endpoint range: a year, a century, or a year span."""
    lo, hi = r
    if lo is None or hi is None:
        return era_year(hi if lo is None else lo, rounded)
    if lo == hi:
        return era_year(lo, rounded)
    if hi - lo == 999 and (lo - 1) % 1000 == 0 and lo > 0:
        return ordinal(hi // 1000) + " millennium", "CE"
    if hi - lo == 999 and hi % 1000 == 0 and hi <= 0:
        return ordinal((1 - lo) // 1000) + " millennium", "BCE"
    if hi - lo == 199 and (lo - 1) % 100 == 0 and lo > 0:  # "the 6th or 7th century"
        return f"{ordinal(hi // 100 - 1)}/{ordinal(hi // 100)} century", "CE"
    if hi - lo == 199 and hi % 100 == 0 and hi <= 0:
        return f"{ordinal((1 - lo) // 100)}/{ordinal((1 - lo) // 100 - 1)} century", "BCE"
    if hi - lo == 99 and (lo - 1) % 100 == 0 and lo > 0:
        return ordinal(hi // 100) + " century", "CE"
    if hi - lo == 99 and hi % 100 == 0 and hi <= 0:
        return ordinal((1 - lo) // 100) + " century", "BCE"
    (a, era_a), (b, era_b) = era_year(lo, rounded), era_year(hi, rounded)
    return (f"{a}/{b}", era_a) if era_a == era_b else (f"{a} {era_a}/{b}", era_b)


def label(start, end, ongoing=False, approximate=False, rounded=False):
    c = "c. " if approximate else ""
    if start is None and end is None:
        return None
    if end is None:
        text, era = describe(start, rounded)
        return f"{c}{text} {era}–present" if ongoing else f"from {c}{text} {era}"
    if start is None:
        text, era = describe(end, rounded)
        return f"until {c}{text} {era}"
    if start == end:  # one stated unit, e.g. "the 3rd millennium"
        text, era = describe(start, rounded)
        return f"{c}{text} {era}"
    (a, era_a), (b, era_b) = describe(start, rounded), describe(end, rounded)
    for unit in (" century", " millennium"):
        if era_a == era_b and a.endswith(unit) and b.endswith(unit):
            return f"{c}{a.removesuffix(unit)}–{b} {era_b}"
    if era_a == era_b and "century" not in a + b and "millennium" not in a + b:
        return f"{c}{a}–{b} {era_b}"
    if era_a == era_b and ("century" in a or "millennium" in a):  # "19th century – 1200 BCE"
        return f"{c}{a} – {b} {era_b}"
    return f"{c}{a} {era_a} – {b} {era_b}"


def basis(claims, reconstructed):
    if not reconstructed and all(c["event"] == "composition" for c in claims):
        return "composition"
    if not reconstructed and any(c["event"] == "native_transmission_end" for c in claims):
        return "last_speaker"
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
        "method": claim.get("method"),
        "citation": source.get("citation") or source.get("title"),
        "url": source.get("url"),
        "quote": span.get("quote"),
        "locator": span.get("locator"),
        **(
            {
                "node": claim["subject"]["name"],
                "proto_mapping": claim["proto_mapping"],
                **({"note": claim["proto_note"]} if claim.get("proto_note") else {}),
            }
            if claim.get("proto_mapping")
            else {}
        ),
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
    elif "crown_divergence" in by_event:
        crown = by_event["crown_divergence"]
        start, end = None, crown_end(crown)
        used = [crown]
        ongoing = False
        approximate = True
    else:
        first, last = (by_event.get(e) for e in ATTESTATION_EVENTS)
        # A language ends when its last native speakers die (Dalmatian, 1898): an
        # adopted end of native transmission replaces the last attestation as the end.
        last = by_event.get("native_transmission_end") or last
        used = [c for c in (first, last) if c]
        if not used and "composition" in by_event:
            # Texts transmitted orally (the Vedas, the Avesta) survive only in much
            # later copies: when composed is the best date, shown as its own basis.
            comp = by_event["composition"]
            start, end = ranges(comp)
            if comp["time"]["form"] == "point":  # "took shape around 1500 BC"
                end = start
            return {
                "start": start,
                "end": end,
                "ongoing": False,
                "approximate": bool(comp["time"].get("original", {}).get("approximate")),
                "rounded": counted_back(comp),
                "claims": [comp],
            }
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
        "rounded": any(counted_back(c) for c in used),
        "claims": used,
    }


def period_record(code, claims, alternatives, view, sources):
    reconstructed = view.get("kind") == "reconstructed" or is_proto_language(view)
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
        "label": label(
            spoken["start"],
            spoken["end"],
            spoken["ongoing"],
            spoken["approximate"],
            spoken["rounded"],
        ),
        "_rounded": spoken["rounded"],
        "evidence": [evidence_entry(c, sources) for c in spoken["claims"]],
        # An adopted claim not used for the display (a crown age beside a stated
        # period) is still shown, as an alternative.
        "alternatives": distinct(
            [
                alternative(c, reconstructed, sources)
                for c in [c for c in claims if c not in spoken["claims"]] + alternatives
            ]
        ),
    }


def distinct(alternatives):
    """One entry per source, page and date: two runs may import the same statement."""
    seen, out = set(), []
    for a in alternatives:
        key = (a["source"], a["locator"], a["label"], a["basis"])
        if key not in seen:
            seen.add(key)
            out.append(a)
    return out


def alternative(claim, reconstructed, sources):
    if claim["event"] in (*ATTESTATION_EVENTS, "native_transmission_end"):
        r = event_range(claim)
        start, end = (r, None) if claim["event"] == "first_attestation" else (None, r)
    elif claim["event"] == "crown_divergence":
        start, end = None, crown_end(claim)
    else:
        start, end = ranges(claim)
    approximate = claim["event"] == "crown_divergence" or bool(
        claim["time"].get("original", {}).get("approximate")
    )
    return {
        **evidence_entry(claim, sources),
        "basis": basis([claim], reconstructed),
        "start": endpoint(start),
        "end": endpoint(end),
        "label": label(start, end, claim["time"]["ongoing"], approximate, counted_back(claim)),
    }


def varieties_by_parent(view):
    """Wiktionary varieties (etymology-only codes) of each code, from the catalog's
    parent links. Not genetic ancestry: the code its entries are filed under."""
    out = {}
    for code, v in view.items():
        for link in (v.get("mapping") or {}).get("links", []):
            target = link.get("target", "")
            if link.get("relation") == "parent_context" and target.startswith("wikt:lang:"):
                out.setdefault(target.removeprefix("wikt:lang:"), []).append(code)
    return {k: sorted(v) for k, v in out.items()}


def span(own, varieties, undated):
    """Earliest start to latest end over a code's own record and its dated varieties."""
    parts = ([own] if own else []) + varieties
    starts = [p["spoken"]["start"] for p in parts]
    ends = [p["spoken"]["end"] for p in parts]
    ongoing = any(p["spoken"]["ongoing"] for p in parts)
    known_starts = [s for s in starts if s]
    known_ends = [e for e in ends if e]

    def lowest(rs, key):
        vals = [r[key] for r in rs if r[key] is not None]
        return min(vals) if vals else None

    def highest(rs, key):
        vals = [r[key] for r in rs if r[key] is not None]
        return max(vals) if vals else None

    start = (
        (lowest(known_starts, "earliest"), lowest(known_starts, "latest")) if known_starts else None
    )
    end = (
        None
        if ongoing or not known_ends
        else (highest(known_ends, "earliest"), highest(known_ends, "latest"))
    )
    approximate = any("c. " in p["label"] for p in parts)  # also "until c. …"
    rounded = any(p["_rounded"] for p in parts)
    partial = bool(undated) or None in starts or (not ongoing and None in ends)
    return {
        "start": endpoint(start),
        "end": endpoint(end),
        "ongoing": ongoing,
        "label": label(start, end, ongoing, approximate, rounded),
        "partial": partial,
        "components": [
            {
                "code": p["code"],
                "name": p["name"],
                "label": p["label"],
                "basis": p["basis"],
                "own": p is own,
            }
            for p in parts
        ],
        "undated": undated,
    }


def add_spans(languages, view):
    records = {r["code"]: r for r in languages}
    for parent, codes in sorted(varieties_by_parent(view).items()):
        dated = [records[c] for c in codes if c in records and "span" not in records[c]]
        if not dated or parent not in view:
            continue
        own = records.get(parent)
        if own:
            own["span"] = span(own, dated, [c for c in codes if c not in records])
    for r in languages:
        del r["_rounded"]  # internal: whether the label rounds years
    languages.sort(key=lambda r: r["code"])


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
    extra = {}  # reviewed alternative crown ages, shown beside a proto default
    for d in bundle["decisions"]:
        c = claims[d["claim_id"]]
        # The registry rejects ineligible decisions at build time; re-check the
        # display preconditions so this export can never widen them.
        view = bundle["view"].get(d["subject_id"].removeprefix("wikt:")) or {}
        if (
            not d["subject_id"].startswith("wikt:")
            or c["subject"]["mapping"] != "exact"
            or decision_claim_error(d, c, view)
        ):
            continue
        if d["event"] == "crown_divergence" and c["subject"].get("type") == "sampled_clade":
            c = {**c, "proto_mapping": d["proto_mapping"]}
            for alt in d.get("proto_alternatives") or []:
                a = claims.get(alt["claim_id"])
                if a and a["status"] == "supported" and a["event"] == "crown_divergence":
                    shown = {**a, "proto_mapping": alt["proto_mapping"]}
                    if alt.get("note"):
                        shown["proto_note"] = alt["note"]
                    extra.setdefault(d["subject_id"][5:], []).append(shown)
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
        record = period_record(code, adopted[code], extra.get(code, []) + others, view, sources)
        if record:
            languages.append(record)
    add_spans(languages, bundle["view"])
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
