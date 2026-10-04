"""Which undated codes to research next: `period_targets.json`.

A target is a Wiktionary code that names a past stage: a proto-language
(reconstructed), a variety (etymology-only code), or a language ISO 639-3 classes as
historical or extinct. Targets are ranked by how many items of the selected
etymology graph (`deploy/graph.json`) carry the code, so research goes first where
trees show the most nodes without a period. Counts are of item records only; a
variety's items are counted under its own code, as trees show it.

This ranks work; it is not evidence and never dates anything.
"""

import gzip
import json
import re
from collections import Counter
from pathlib import Path

from workflow_tools.paths import DATA, DATING_PRODUCT, REPO, selected

ISO_TYPES = {"H": "historical", "E": "extinct"}
LANG = re.compile(rb'"lang":(\d+)')


def lang_ids(languages):
    """The graph's numeric language ids, as wety_core assigns them: main codes in
    sorted code order (crates/core/src/languages.rs)."""
    ids = {}
    for code in sorted(languages, key=lambda c: c.encode()):
        main = languages[code]["mainCode"]
        if main not in ids:
            ids[main] = len(ids)
    return {i: main for main, i in ids.items()}


def graph_counts(path, chunk=1 << 24):
    counts = Counter()
    tail = b""
    with gzip.open(path, "rb") as f:
        while block := f.read(chunk):
            data = tail + block
            cut = data.rfind(b",")  # never split a "lang":N match across blocks
            counts.update(int(m) for m in LANG.findall(data[:cut]))
            tail = data[cut:]
    counts.update(int(m) for m in LANG.findall(tail))
    return counts


def iso_types():
    rows = (DATA / "workspaces/catalogs/sources/iso-639-3.tab").read_text().splitlines()[1:]
    return {r.split("\t")[0]: r.split("\t")[5] for r in rows if r.strip()}


def targets(graph_path=None):
    from . import glotto
    from .registry import current_bundle

    languages = json.loads(selected("wiktionary_languages").read_text())
    if graph_path is None:
        sel = json.loads((REPO / "deploy/graph.json").read_text())
        graph_path = (
            DATA
            / "artifacts/etymology-graph"
            / sel["artifact_id"].removeprefix("sha256:")
            / "files"
            / sel["file"]
        )
    counts = graph_counts(graph_path)
    by_code = {lang_ids(languages)[i]: n for i, n in counts.items()}
    g = glotto.load()
    iso = iso_types()
    periods = json.loads((Path(DATING_PRODUCT) / "language_periods.json").read_text())
    dated = {r["code"]: r["basis"] for r in periods["languages"]}
    bundle = current_bundle()
    claims = {c["id"]: c for c in bundle["claims"]}
    out = []
    for code, lang in languages.items():
        kind = lang["kind"]
        iso_type = ISO_TYPES.get(iso.get((g.wikt.get(code) or {}).get("iso639_3") or ""))
        if kind not in ("reconstructed", "etymology-only") and not iso_type:
            continue
        view = bundle["view"].get(code) or {}
        evidence = [claims[i] for i in view.get("evidence_ids", [])]
        out.append(
            {
                "code": code,
                "name": lang["canonicalName"],
                "kind": kind,
                "iso_type": iso_type,
                "parent": lang["nonEtymologyOnly"] if kind == "etymology-only" else None,
                "items": by_code.get(code, 0),
                "period": dated.get(code),
                "claims": len(evidence),
                "supported": sum(c["status"] == "supported" for c in evidence),
            }
        )
    out.sort(key=lambda t: (-t["items"], t["code"]))
    return {
        "graph": str(Path(graph_path).relative_to(REPO)),
        "generation": bundle.get("input_digest"),
        "targets": out,
    }


def write_targets(path=None, graph_path=None):
    from .io_utils import atomic_json

    result = targets(graph_path)
    path = Path(path or Path(DATING_PRODUCT) / "period_targets.json")
    atomic_json(path, result)
    rows = result["targets"]
    undated = [t for t in rows if not t["period"]]
    return {
        "path": str(path),
        "targets": len(rows),
        "dated": len(rows) - len(undated),
        "undated_with_items": sum(1 for t in undated if t["items"]),
        "top_undated": [f"{t['code']} ({t['name']}): {t['items']}" for t in undated[:25]],
    }
