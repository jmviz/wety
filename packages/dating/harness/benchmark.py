"""Publication-fidelity checks; explicitly not historical-truth validation.

Optional predictions are an object keyed by semantic fixture ID, with exactly the
fields in `expected`. They must be produced separately; this command never calls
an LLM or silently substitutes the reference labels for predictions.
"""

from workflow_tools.paths import DATING_PRODUCT, DATING_GENERATIONS, DATING_HISTORY

import json
from pathlib import Path
from workflow_tools.paths import REPO, historical
from .config import DATING_DIR, DATA_DIR
from .tree_ingest import summarize
from .io_utils import atomic_json, digest


def semantic_metrics(pred, semantics):
    expected = {c["id"]: c["expected"] for c in semantics}
    if not isinstance(pred, dict) or any(not isinstance(v, dict) for v in pred.values()):
        raise ValueError("predictions must map case IDs to field objects")
    unknown = set(pred) - set(expected)
    if unknown:
        raise ValueError("unknown prediction IDs: " + str(sorted(unknown)))
    fields = {k for e in expected.values() for k in e}
    return {
        "status": "development_evaluation",
        "prediction_sha256": digest(pred),
        "cases": len(expected),
        "submitted": len(pred),
        "exact_match": sum(pred.get(k) == v for k, v in expected.items()) / len(expected),
        "field_accuracy": {
            f: sum(
                k in pred and f in pred[k] and pred[k][f] == v[f]
                for k, v in expected.items()
                if f in v
            )
            / sum(f in v for v in expected.values())
            for f in sorted(fields)
        },
        "limitation": "Small source-grounded development set, not held out and not specialist adjudicated.",
    }


def evaluate(predictions=None):
    root = Path(DATING_DIR)
    results = []
    cases = json.loads((root / "benchmarks/published_roots.json").read_text())
    for case in cases:
        try:
            manifest = json.loads(historical(case["manifest"]).read_text())
            claims = summarize(manifest)
            crown = max(claims, key=lambda c: len(c["subject"]["taxa"]))
            native = crown["time"]["original"]
            factor = {"BP": 1, "ka_BP": 1000, "century_BP": 100}[native["unit"]]
            observed = {k: native[k] * factor for k in case["expected_years_bp"]}
            passed = (
                len(crown["subject"]["taxa"]) == case["expected_taxa"]
                and native["interval_type"] == case["interval_type"]
                and all(
                    abs(v - case["expected_years_bp"][k]) <= case["tolerance_years"]
                    for k, v in observed.items()
                )
            )
            results.append(
                {
                    "id": case["id"],
                    "passed": passed,
                    "observed_years_bp": observed,
                    "expected_years_bp": case["expected_years_bp"],
                    "taxa": len(crown["subject"]["taxa"]),
                    "calendar_epoch": manifest.get("present"),
                }
            )
        except (ValueError, KeyError, OSError) as e:
            results.append({"id": case["id"], "passed": False, "error": str(e)})
    semantics = json.loads((root / "benchmarks/semantic_cases.json").read_text())
    metrics = {
        "status": "not_measured",
        "reason": "No independent model predictions supplied; development labels are not a held-out expert benchmark.",
    }
    if predictions:
        metrics = semantic_metrics(json.loads(Path(predictions).read_text()), semantics)
    result = {
        "passed": all(r["passed"] for r in results),
        "scope": "Reproduction of published source statistics, units, interval labels and sample scope; not proof of historical dates.",
        "publication_checks": results,
        "semantic_metrics": metrics,
    }
    atomic_json(Path(DATING_PRODUCT) / "benchmark.json", result)
    return result
