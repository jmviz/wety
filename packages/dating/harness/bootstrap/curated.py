"""Hand-curated conventional periodizations (data/curated/periods.json), each
citing a standard handbook. Ingested as `conventional_period` claims that rank
above Glottolog's and Wikipedia's in synthesis (SOURCE_RANK)."""

from __future__ import annotations

import json
import os

from .. import glotto
from ..config import DATA_DIR, now
from ..schema import Claim, Source, TimeValue
from ..store import upsert_source
from . import claim_id, write_claims

PATH = os.path.join(DATA_DIR, "curated", "periods.json")


def run() -> dict:
    g = glotto.load()
    with open(PATH, encoding="utf-8") as f:
        data = json.load(f)
    sources = {sid: Source(id=sid, retrieved=now(), **s) for sid, s in data["sources"].items()}
    for s in sources.values():
        upsert_source(s)
    by_source: dict[str, list[Claim]] = {sid: [] for sid in sources}
    for p in data["periods"]:
        gid = p["glottocode"]
        if gid not in g.rows:
            print(f"  curated: unknown glottocode {gid}")
            continue
        by_source[p["source"]].append(
            Claim(
                id=claim_id(gid, p["source"], "period"),
                languoid=gid,
                languoid_name=g.rows[gid]["name"],
                kind="conventional_period",
                time=TimeValue(
                    earliest=p["earliest"],
                    latest=p["latest"],
                    open_end=p["latest"] is None,
                    precision=p.get("precision", "century"),
                ),
                source=p["source"],
                locator="periodization as summarized in the citation",
                quote=p["quote"],
                quote_verified=None,
                extracted_by={"program": "harness.bootstrap.curated", "run": "bootstrap"},
                notes=(f"curated: {p['name']}. " + (p.get("note") or "")).strip(),
            )
        )
    total = 0
    for sid, claims in by_source.items():
        s = write_claims(g, sources[sid], claims)
        total += s["claims"]
    return {"source": "curated", "claims": total, "sources": len(sources)}
