"""Curated public release of dating data as linked data (Turtle and JSON-LD).

A release is built from what wety shows (`language_periods.json`: adopted periods,
their evidence, alternatives and revivals) plus the editorial ledger, never from the
working store, so superseded results and process records (runs, packets, digests,
paths) cannot reach it. Publishable fields are listed here explicitly.

Quotes are governed by each source's reuse rights, a separate ledger kind ("rights",
keyed by source id, so recording them does not change any decision's evidence digest):
- open: the quote is published as recorded;
- short_quote: published up to QUOTE_LIMIT characters;
- citation_only, or no rights recorded: the citation and locator only.
"""

from __future__ import annotations

import json
from pathlib import Path

from workflow_tools.paths import DATING_PRODUCT

BASE = "https://wety.org/id/"
REUSE = ("citation_only", "short_quote", "open")
QUOTE_LIMIT = 300
PREFIXES = {
    "wety": "https://wety.org/ns/dating#",
    "prov": "http://www.w3.org/ns/prov#",
    "dcterms": "http://purl.org/dc/terms/",
    "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
    "xsd": "http://www.w3.org/2001/XMLSchema#",
}
# Fields of an evidence entry that a release may carry; everything else is internal.
PUBLIC_EVIDENCE = ("event", "role", "method", "locator", "label", "node", "proto_mapping", "note")


def set_rights(source_id, reuse, basis, license=None, reviewer=None):
    from . import ledger
    from .store import load_sources

    if reuse not in REUSE:
        raise ValueError(f"reuse must be one of {REUSE}")
    if source_id not in load_sources():
        raise ValueError(f"unknown source {source_id}")
    if not (basis or "").strip():
        raise ValueError("say how the reuse rights are known")
    record = {"source_id": source_id, "reuse": reuse, "license": license, "basis": basis}
    ledger.append(
        [{"kind": "rights", "key": source_id, "op": "put", "record": record}],
        reason="source reuse rights",
        actor=reviewer,
    )
    return record


def rights():
    from . import ledger

    return ledger.current("rights")[0]


def quote_for(quote, reuse):
    """(published quote or None, how it was treated)."""
    if not quote:
        return None, "none"
    if reuse == "open":
        return quote, "full"
    if reuse == "short_quote":
        if len(quote) <= QUOTE_LIMIT:
            return quote, "full"
        return quote[: QUOTE_LIMIT - 1].rsplit(" ", 1)[0] + "…", "shortened"
    return None, "withheld"


def build(periods=None):
    """The release graph: {"nodes": {iri: {property: [values]}}, "stats": {...}}.
    Values are {"@id": iri} or {"@value": v, "@type": xsd type?}."""
    from . import ledger
    from .registry import load_evidence
    from .store import load_sources

    periods = periods or json.loads((Path(DATING_PRODUCT) / "language_periods.json").read_text())
    sources = load_sources()
    claims = {c["id"]: c for c in load_evidence()}
    reuse = {k: v["reuse"] for k, v in rights().items()}
    decisions = {}
    for d in ledger.current("decision")[0].values():
        decisions.setdefault(d["subject_id"].removeprefix("wikt:"), []).append(d)
    nodes, stats = {}, {"full": 0, "shortened": 0, "withheld": 0, "none": 0}

    def add(iri, prop, value):
        nodes.setdefault(iri, {}).setdefault(prop, [])
        if value not in nodes[iri][prop]:
            nodes[iri][prop].append(value)

    lit = lambda v, t=None: {"@value": v, **({"@type": t} if t else {})}
    ref = lambda iri: {"@id": iri}

    def source_node(sid):
        iri = BASE + "source/" + sid
        if iri in nodes:
            return iri
        s = sources.get(sid, {})
        add(iri, "@type", ref("dcterms:BibliographicResource"))
        for prop, key in (
            ("dcterms:bibliographicCitation", "citation"),
            ("dcterms:title", "title"),
        ):
            if s.get(key):
                add(iri, prop, lit(s[key]))
        for a in s.get("authors") or []:
            add(iri, "dcterms:creator", lit(a))
        if s.get("year"):
            add(iri, "dcterms:date", lit(str(s["year"]), "xsd:gYear"))
        if s.get("url"):
            add(iri, "wety:url", lit(s["url"]))
        add(iri, "wety:reuse", lit(reuse.get(sid, "unassessed")))
        return iri

    def evidence_node(e, kind):
        iri = BASE + "claim/" + e["claim_id"]
        c = claims.get(e["claim_id"], {})
        add(iri, "@type", ref("wety:Evidence"))
        add(iri, "@type", ref("prov:Entity"))
        add(iri, "wety:shownAs", lit(kind))
        for k in PUBLIC_EVIDENCE:
            if e.get(k):
                add(iri, "wety:" + k, lit(e[k] if isinstance(e[k], str) else json.dumps(e[k])))
        if c.get("source_id"):
            add(iri, "dcterms:source", ref(source_node(c["source_id"])))
            quote, how = quote_for(e.get("quote"), reuse.get(c["source_id"]))
            stats[how] += 1
            if quote:
                add(iri, "wety:quote", lit(quote))
        return iri

    for r in periods["languages"]:
        code = r["code"]
        lang, period = BASE + "language/" + code, BASE + "period/" + code
        add(lang, "rdfs:label", lit(r.get("name") or code))
        add(lang, "wety:code", lit(code))
        add(period, "@type", ref("wety:LanguagePeriod"))
        add(period, "wety:language", ref(lang))
        for prop, key in (("wety:label", "label"), ("wety:basis", "basis"), ("wety:kind", "kind")):
            add(period, prop, lit(r[key]))
        spoken = r.get("spoken") or {}
        for side in ("start", "end"):
            for edge in ("earliest", "latest"):
                v = (spoken.get(side) or {}).get(edge)
                if v is not None:
                    # Astronomical years: 1 BCE is 0.
                    add(period, f"wety:{side}{edge.capitalize()}", lit(v, "xsd:integer"))
        if spoken.get("ongoing"):
            add(period, "wety:ongoing", lit(True, "xsd:boolean"))
        if r.get("estimate"):
            add(period, "wety:estimate", lit(r["estimate"]["text"]))
            add(period, "wety:estimateStatistic", lit(r["estimate"]["statistic"]))
            add(period, "wety:estimateInterval", lit(r["estimate"]["interval"]))
        for e in r.get("evidence", []):
            add(period, "prov:wasDerivedFrom", ref(evidence_node(e, "evidence")))
        for e in r.get("alternatives", []):
            add(period, "wety:alternative", ref(evidence_node(e, "alternative")))
        for e in r.get("revival", []):
            add(period, "wety:revival", ref(evidence_node(e, "revival")))
        for d in decisions.get(code, []):
            add(period, "wety:rationale", lit(d["rationale"]))
            # The reviewer field describes the reviewing session, not a stable agent.
            add(period, "wety:reviewedBy", lit(d["reviewer"]))
    return {"nodes": nodes, "stats": {"periods": len(periods["languages"]), "quotes": stats}}


def _term(v):
    if "@id" in v:
        iri = v["@id"]
        return iri if ":" in iri and not iri.startswith("http") else f"<{iri}>"
    value = v["@value"]
    if isinstance(value, bool):
        return "true" if value else "false"
    text = json.dumps(value if isinstance(value, str) else str(value), ensure_ascii=False)
    return text + (f"^^{v['@type']}" if v.get("@type") else "")


def turtle(graph):
    lines = [f"@prefix {p}: <{iri}> ." for p, iri in PREFIXES.items()] + [""]
    for iri, props in sorted(graph["nodes"].items()):
        body = []
        for prop, values in props.items():
            name = "a" if prop == "@type" else prop
            body.append(f"    {name} " + ", ".join(_term(v) for v in values))
        lines.append(f"<{iri}>\n" + " ;\n".join(body) + " .\n")
    return "\n".join(lines)


def jsonld(graph):
    nodes = []
    for iri, props in sorted(graph["nodes"].items()):
        node = {"@id": iri}
        for prop, values in props.items():
            node[prop] = [v["@id"] if prop == "@type" else v for v in values]
        nodes.append(node)
    return {"@context": PREFIXES, "@graph": nodes}


def write_release(out=None):
    periods = json.loads((Path(DATING_PRODUCT) / "language_periods.json").read_text())
    graph = build(periods)
    out = Path(out or Path(DATING_PRODUCT) / "release")
    out.mkdir(parents=True, exist_ok=True)
    (out / "dating.ttl").write_text(turtle(graph))
    (out / "dating.jsonld").write_text(json.dumps(jsonld(graph), ensure_ascii=False, indent=1))
    manifest = {
        "schema_version": 1,
        "generation": periods.get("generation"),
        "base": BASE,
        "quote_limit": QUOTE_LIMIT,
        **graph["stats"],
        "files": ["dating.ttl", "dating.jsonld"],
    }
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2))
    return manifest
