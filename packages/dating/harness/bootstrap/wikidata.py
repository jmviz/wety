"""Wikidata temporal statements already in the crossref snapshot
(data/products/catalogs/sources/wikidata_items.json):

    P1249 time of earliest written record -> attestation_first
    P571 inception / P580 start time     -> expert_estimate (start)
    P576 dissolved / P582 end time        -> expert_estimate (end)

Wikidata statements are community-entered and mostly unreferenced, hence
`expert_estimate`; precision is unknown (Wikidata's own
precision field is not in the snapshot). Wikidata years are historical
(-0269 = 269 BCE) and are converted."""

from __future__ import annotations

import json
import os
import re

from .. import glotto
from ..config import now
from workflow_tools.paths import CATALOG_WORKSPACE

LANGCODES_DIR = str(CATALOG_WORKSPACE)
from ..schema import Claim, Source
from ..time_utils import historical_to_astronomical
from . import claim_id, interval, point, write_claims

SOURCE = Source(
    id="src_wikidata_snapshot",
    type="database",
    title="Wikidata (temporal statements on language items, snapshot taken for the langcodes crossref)",
    url="https://www.wikidata.org",
    license="CC0-1.0",
)

PROPS = {
    "earliest_written_record": ("attestation_first", "P1249"),
    "inception": ("expert_estimate", "P571"),
    "start_time": ("expert_estimate", "P580"),
    "dissolved": ("expert_estimate", "P576"),
    "end_time": ("expert_estimate", "P582"),
}


def _year(ts: str) -> int | None:
    m = re.match(r"^([+-]?\d+)-", ts)
    if not m:
        return None
    return historical_to_astronomical(int(m.group(1)))


def _precision(y: int) -> str:
    return "unknown"  # source precision was not retained in this snapshot


def run() -> dict:
    g = glotto.load()
    with open(os.path.join(LANGCODES_DIR, "sources", "wikidata_items.json"), encoding="utf-8") as f:
        items = json.load(f)
    with open(os.path.join(LANGCODES_DIR, "sources", "wikidata_codes.json"), encoding="utf-8") as f:
        codes = json.load(f)
    with open(os.path.join(LANGCODES_DIR, "sources", "manifest.json"), encoding="utf-8") as f:
        SOURCE.retrieved = json.load(f)["files"]["wikidata_items.json"]["retrieved"]
    # Catalog context only. Retain all conflicting destinations; never let
    # iteration order or a proto/family shortcut decide a QID's identity.
    from collections import defaultdict

    contexts = defaultdict(set)
    for q, d in codes.items():
        contexts[q].update(x for x in d.get("glottocode", []) if x in g.rows)
    for rec in g.wikt.values():
        q = rec.get("wikidata_resolved") or rec.get("wikidata")
        if q and rec.get("glottocode"):
            contexts[q].add(rec["glottocode"])
    q_to_g = {q: next(iter(gs)) for q, gs in contexts.items() if len(gs) == 1}

    claims: list[Claim] = []
    for q, d in items.items():
        gid = q_to_g.get(q)
        for key, (kind, prop) in PROPS.items():
            for ts in d.get(key, []):
                y = _year(ts)
                if y is None:
                    continue
                prec = _precision(y)
                is_start = key in ("earliest_written_record", "inception", "start_time")
                if kind == "expert_estimate":
                    t = interval(y, None, prec) if is_start else interval(None, y, prec)
                    t.conversion = "historical->astronomical"
                else:
                    t = point(y, prec, conversion="historical->astronomical")
                claims.append(
                    Claim(
                        id=claim_id(gid, SOURCE.id, f"{q}_{prop}_{ts}"),
                        languoid=gid,
                        languoid_name=d.get("label", q),
                        kind=kind,
                        time=t,
                        source=SOURCE.id,
                        evidence={
                            "subject_id": "wikidata:" + q,
                            "subject_type": "unresolved",
                            "raw_statement": ts,
                            "catalog_mapping": "unresolved",
                            "catalog_candidates": sorted(contexts.get(q, [])),
                            "catalog_mapping_basis": "Wikidata/source catalog association only",
                            "source_precision": "not captured in input snapshot",
                        },
                        locator=f"{q} {prop}",
                        quote=f"{d.get('label', q)}: {prop} = {ts}",
                        quote_verified=True,
                        extracted_by={"program": "harness.bootstrap.wikidata", "run": "bootstrap"},
                        notes=f"Wikidata {prop} ({key}); {'start' if is_start else 'end'} of the item's existence as Wikidata records it",
                    )
                )
    return write_claims(g, SOURCE, claims)
