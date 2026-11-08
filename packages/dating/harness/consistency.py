"""Coherence of displayed dates across a family.

Defaults are chosen code by code, so neighbouring nodes of one family can be dated by
different studies whose methods differ systematically (a Bayesian phylogeny's crown
ages against a specialist's estimate or a glottochronological date). Two safeguards:

* Ordering. Along Wiktionary's ancestry, a proto-language may not end after its
  descendant ends, nor after its descendant begins (is attested, or its own period
  starts). Only clear conflicts count: intervals that do not overlap. The catalog tree
  is used to check what wety displays, never to infer a date.
* Chronology frames. For a family (the subtree under a root code) the reviewer chooses
  one frame: one analysis, or one author's consistent system, named by study ids (ledger
  kind `frame`, keyed by the root). Where the frame has an adopted estimate for a code,
  that estimate is displayed and other adopted estimates become alternatives; a code the
  frame covers but whose default comes from elsewhere is an error unless the frame lists
  it as an exception with a reason. Codes the frame does not date keep other sources and
  are marked as outside it.
"""

from __future__ import annotations

ESTIMATE_ROLES = ("hypothesis", "posterior")
FRAME_EVENTS = ("crown_divergence", "conventional_period")


def ancestors(langs, code):
    """Wiktionary ancestry of a code, nearest first. A variety (etymology-only code)
    without ancestors of its own descends from its parent language's ancestors."""
    x = langs.get(code) or {}
    first = (x.get("ancestors") or [None])[0]
    if not first and x.get("kind") == "etymology-only" and x.get("nonEtymologyOnly"):
        return ancestors(langs, x["nonEtymologyOnly"])
    out = []
    while first and first not in out:
        out.append(first)
        first = ((langs.get(first) or {}).get("ancestors") or [None])[0]
    return out


def frame_root(langs, code, frames):
    """The frame whose root is the code itself or its nearest framed ancestor."""
    for c in [code, *ancestors(langs, code)]:
        if c in frames:
            return c
    return None


def in_frame(claim, frame):
    return claim["study_id"] in frame["study_ids"]


def is_estimate(claim):
    return claim["role"] in ESTIMATE_ROLES


def ordering_errors(languages, langs):
    by = {r["code"]: r for r in languages}
    errors = []
    for r in languages:
        start, end = r["spoken"].get("start"), r["spoken"].get("end")
        for a in ancestors(langs, r["code"]):
            p = by.get(a)
            if not p or p["kind"] != "proto" or not p["spoken"].get("end"):
                continue
            earliest = p["spoken"]["end"].get("earliest")
            if earliest is None:
                continue
            for what, x in (("ends", end), ("begins", start)):
                latest = (x or {}).get("latest")
                if latest is not None and earliest > latest:
                    errors.append(
                        f"{a} ({p['label']}, {p['evidence'][0]['source']}) ends after its "
                        f"descendant {r['code']} {what} ({r['label']}, {r['evidence'][0]['source']})"
                    )
    return errors


def frame_errors(bundle, languages, langs):
    """Codes a frame dates whose displayed estimate comes from another study."""
    frames = bundle.get("frames") or {}
    by_id = {c["id"]: c for c in bundle["claims"]}
    by = {r["code"]: r for r in languages}
    errors = []
    from .registry import is_proto_language

    for d in bundle["decisions"]:
        code = d["subject_id"].removeprefix("wikt:")
        root = frame_root(langs, code, frames)
        if not is_proto_language(bundle["view"].get(code)):
            continue
        if not root or code in frames[root].get("exceptions", {}):
            continue
        frame = frames[root]
        offered = [by_id[d["claim_id"]]] + [
            by_id[a["claim_id"]]
            for a in d.get("proto_alternatives") or []
            if a["claim_id"] in by_id
        ]
        covered = any(in_frame(c, frame) and c["event"] in FRAME_EVENTS for c in offered)
        shown = [by_id[e["claim_id"]] for e in (by.get(code) or {}).get("evidence", [])]
        if covered and any(is_estimate(c) and not in_frame(c, frame) for c in shown):
            errors.append(
                f"{code}: the {frame['name']} frame ({root}) dates it, but its displayed "
                "estimate comes from another study; adopt the frame's claim or list the code "
                "as an exception with a reason"
            )
    return sorted(set(errors))


def consistency_errors(bundle, languages=None, langs=None):
    from workflow_tools.paths import selected
    import json

    from .periods import export_periods

    langs = langs or json.loads(selected("wiktionary_languages").read_text())
    languages = languages or export_periods(bundle, langs=langs)["languages"]
    return ordering_errors(languages, langs) + frame_errors(bundle, languages, langs)


def report(bundle, langs):
    """Per family root with two or more dated estimates: its frame and, for each dated
    code, the study of its displayed estimate and whether it is in the frame."""
    from .periods import export_periods

    languages = export_periods(bundle, langs=langs)["languages"]
    frames = bundle.get("frames") or {}
    by_id = {c["id"]: c for c in bundle["claims"]}
    estimated = [r for r in languages if r["basis"] == "estimated"]
    dated = {r["code"] for r in estimated}
    families = {}
    for r in estimated:
        top = r["code"]
        for a in ancestors(langs, r["code"]):
            if a in dated:
                top = a
        root = frame_root(langs, r["code"], frames) or top
        families.setdefault(root, []).append(r)
    out = []
    for root, rs in sorted(families.items(), key=lambda x: -len(x[1])):
        if len(rs) < 2:
            continue
        frame = frames.get(root)
        rows = []
        for r in sorted(rs, key=lambda r: r["code"]):
            studies = sorted({by_id[e["claim_id"]]["study_id"] for e in r["evidence"]})
            rows.append(
                {
                    "code": r["code"],
                    "label": r["label"],
                    "source": r["evidence"][0]["source"],
                    "studies": studies,
                    "in_frame": (r.get("frame") or {}).get("in_frame"),
                }
            )
        out.append(
            {
                "root": root,
                "frame": frame and {k: frame[k] for k in ("name", "study_ids")},
                "studies": len({s for row in rows for s in row["studies"]}),
                "codes": rows,
            }
        )
    errors = ordering_errors(languages, langs) + frame_errors(bundle, languages, langs)
    return {"errors": errors, "families": out}
