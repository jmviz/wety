"""Pilot completeness is measured against named sources, not successful downloads."""

import hashlib
import json
from pathlib import Path
from collections import Counter
from .config import DATA_DIR, CACHE_DIR
from .retrieve import assess_text, RETRIEVAL_VERSION


def source_state(c):
    r = c.get("retrieved") or {}
    sha = r.get("sha256")
    text = None
    raw_ok = False
    cache_state = "absent"
    if sha:
        p = Path(CACHE_DIR) / "texts" / (sha + ".txt")
        if p.exists():
            cache_state = (
                "verified" if hashlib.sha256(p.read_bytes()).hexdigest() == sha else "hash_mismatch"
            )
            if cache_state == "verified":
                text = p.read_text()
    raw = r.get("raw_sha256")
    if raw:
        p = Path(CACHE_DIR) / "raw" / raw
        raw_ok = p.exists() and hashlib.sha256(p.read_bytes()).hexdigest() == raw
    quality = "missing"
    if text:
        quality = r.get("quality") or "legacy_unverified"
        if r.get("kind") == "s2-snippets":
            quality = "excerpt"
        elif (
            r.get("version") == RETRIEVAL_VERSION
            and assess_text(text, r.get("kind")) != "fulltext_candidate"
        ):
            quality = "partial_or_landing"
    elif c.get("abstract"):
        quality = "abstract_only"
    if cache_state == "hash_mismatch":
        quality = "cached_text_hash_mismatch"
    ex = c.get("extracted") or {}
    if not ex:
        extraction = "not_extracted"
    elif ex.get("input_kind") == "abstract_or_dataset_only":
        extraction = "abstract_or_dataset_only_input"
    elif not ex.get("retrieved_text_sha256"):
        extraction = "legacy_input_unverified"
    elif ex["retrieved_text_sha256"] != sha:
        extraction = "stale_input"
    else:
        extraction = "current_input"
    return {
        "retrieval": quality,
        "cached_text_state": cache_state,
        "text_hash_verified": bool(text),
        "raw_hash_verified": raw_ok,
        "extraction": extraction,
        "extracted_claim_count": ex.get("claims"),
        "zero_claims_is_evidence_of_absence": False,
        "text_sha256": sha,
    }


def pilot_coverage():
    from .packets import packet_summaries

    packets = packet_summaries()
    path = Path(DATA_DIR) / "_research/pilot_sources.json"
    targets = json.loads(path.read_text()) if path.exists() else []
    rows = []
    for target in targets:
        candidates_path = Path(DATA_DIR) / target["family"] / "candidates.json"
        cs = json.loads(candidates_path.read_text()) if candidates_path.exists() else []
        matches = [c for c in cs if (c.get("doi") or "").lower() == target["doi"].lower()]
        states = [source_state(c) for c in matches]
        reviews = [
            {
                "packet_id": p["packet_id"],
                "packet_sha256": p["packet_sha256"],
                "scope": s["review_scope"],
                "completeness": s["completeness"],
                "independent_review": p["independent_review"],
                "claim_count": len(s["claim_ids"]),
                "review_independence": p.get("review_independence"),
                "finding_count": p["finding_counts"][s["source_id"]],
            }
            for p in packets
            if not p["errors"]
            for s in p["source_reviews"]
            if s["doi"].lower() == target["doi"].lower()
        ]
        rows.append(
            {
                **target,
                "discovered": bool(matches),
                "encounters": states,
                "editorial_reviews": reviews,
            }
        )
    return {
        "scope": "Named primary works and methodological sources; fulltext_candidate is not a completeness adjudication.",
        "targets": rows,
        "target_count": len(rows),
        "missing_discovery": sum(not r["discovered"] for r in rows),
        "retrieval_encounters": dict(
            Counter(s["retrieval"] for r in rows for s in r["encounters"])
        ),
        # The bibliography is a minimum roster, not a filter on editorial inputs.
        # Bind every valid packet to the bundle, including conditional case hashes.
        "editorial_packets": [
            {
                "packet_id": p["packet_id"],
                "packet_sha256": p["packet_sha256"],
                "source_ids": [s["source_id"] for s in p["source_reviews"]],
                "evaluation_split": p["evaluation_split"],
                "independent_review": p["independent_review"],
                "review_independence": p.get("review_independence"),
                "claims": p["claims"],
                "findings": p["findings"],
                "relations": p["relations"],
            }
            for p in packets
            if not p["errors"]
        ],
        "packet_errors": {p["packet_id"]: p["errors"] for p in packets if p["errors"]},
        "extraction_accuracy": "not_measured",
    }
