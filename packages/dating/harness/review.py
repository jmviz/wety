"""Explicit, provenance-bearing editorial workflow; no numerical overrides."""

from workflow_tools.paths import DATING_PRODUCT, DATING_GENERATIONS, DATING_HISTORY

import json
from pathlib import Path
from .config import DATA_DIR
from .evidence import EvidenceClaim, Decision
from .registry import (
    build_bundle,
    bundle_decision_digest,
    decision_claim_error,
    default_scope_error,
    validate_span,
)
from .io_utils import digest, atomic_json


def import_evidence(path):
    payload = json.loads(Path(path).read_text())
    raws = payload.get("claims") if isinstance(payload, dict) else payload
    relations = payload.get("relations", []) if isinstance(payload, dict) else []
    if isinstance(payload, dict) and set(payload) - {"claims", "relations"}:
        raise ValueError("unknown import fields")
    if not isinstance(raws, list):
        raise ValueError("expected a list of evidence claims")
    bundle = build_bundle()
    existing = {c["id"]: c for c in bundle["claims"]}
    records = []
    for raw in raws:
        c = EvidenceClaim.model_validate(raw).model_dump(mode="json")
        if c["source_id"] not in bundle["sources"]:
            raise ValueError("register source before evidence")
        if c["id"] in existing and existing[c["id"]] != c:
            raise ValueError(
                "changed evidence requires a new ID, referencing the earlier claim as provenance"
            )
        if c["status"] == "supported":
            if not c["assessment"]["reviewer"] or not c["assessment"]["rationale"]:
                raise ValueError("support requires named reviewer and rationale")
            errors = validate_span(c)
            if errors:
                raise ValueError("; ".join(errors))
        records.append(c)
    trial = dict(bundle)
    trial["claims"] = bundle["claims"] + [c for c in records if c["id"] not in existing]
    trial["relations"] = bundle["relations"] + [
        r for r in relations if r not in bundle["relations"]
    ]
    from .registry import validate_bundle

    errors = validate_bundle(trial)
    if errors:
        raise ValueError("; ".join(errors))
    new = [c for c in records if c["id"] not in existing]
    # Validate the entire change before writes. Publication remains a separate,
    # atomic generation, so intermediate ingestion state cannot reach readers.
    if new or relations:
        atomic_json(
            Path(DATING_HISTORY)
            / "imports"
            / (digest({"claims": new, "relations": trial["relations"]}) + ".json"),
            {"claims": new, "relations": trial["relations"]},
        )
    if new:
        atomic_json(Path(DATA_DIR) / "_evidence" / ("review_" + digest(new) + ".json"), new)
    if relations:
        atomic_json(Path(DATA_DIR) / "_constraints/relations.json", trial["relations"])
    return {"imported": len(new), "publish_required": bool(new or relations)}


def decide(path):
    return decide_record(json.loads(Path(path).read_text()))


def decide_record(raw, bundle=None):
    bundle = bundle or build_bundle()
    by_id = {c["id"]: c for c in bundle["claims"]}
    decision = Decision.model_validate(raw)
    if not decision.reviewer.strip() or not decision.rationale.strip():
        raise ValueError("reviewer and rationale required")
    c = by_id.get(decision.claim_id)
    if decision.evidence_digest != bundle_decision_digest(bundle, decision.subject_id):
        raise ValueError("stale decision digest; run decision-digest for the subject")
    subject = bundle["view"].get(decision.subject_id.removeprefix("wikt:"))
    if not c or decision_claim_error(decision.model_dump(), c, subject):
        reason = decision_claim_error(decision.model_dump(), c, subject) if c else None
        raise ValueError(
            "claim not eligible for this subject and event" + (": " + reason if reason else "")
        )
    if reason := default_scope_error(c, bundle["research_graph"]):
        raise ValueError(reason)
    if any(i not in by_id or by_id[i]["status"] != "supported" for i in decision.dependencies):
        raise ValueError("unsupported dependency")
    dest = Path(DATA_DIR) / "_decisions/decisions.json"
    old = json.loads(dest.read_text()) if dest.exists() else []
    updated = [
        d for d in old if (d["subject_id"], d["event"]) != (decision.subject_id, decision.event)
    ] + [decision.model_dump()]
    atomic_json(
        Path(DATING_HISTORY) / "decisions" / (digest({"old": old, "new": updated}) + ".json"),
        {"old": old, "new": updated},
    )
    atomic_json(dest, updated)
    return {"adopted_claim": decision.claim_id, "publish_required": True}


def withdraw(subject_id, event, reviewer, rationale):
    """Remove an editorial decision, keeping it with the reason in withdrawals.json.

    A decision is withdrawn when better evidence of another event should be shown
    instead (a period decision otherwise takes precedence in the display). Its claim
    and evidence are untouched; adopting it again is an ordinary decision."""
    from datetime import datetime, timezone

    if not reviewer.strip() or not rationale.strip():
        raise ValueError("reviewer and rationale required")
    folder = Path(DATA_DIR) / "_decisions"
    dest = folder / "decisions.json"
    old = json.loads(dest.read_text()) if dest.exists() else []
    match = [d for d in old if (d["subject_id"], d["event"]) == (subject_id, event)]
    if not match:
        raise ValueError(f"no decision for {subject_id} {event}")
    updated = [d for d in old if d not in match]
    record = {
        "decision": match[0],
        "reviewer": reviewer,
        "rationale": rationale,
        "withdrawn_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
    }
    log = folder / "withdrawals.json"
    withdrawals = json.loads(log.read_text()) if log.exists() else []
    atomic_json(
        Path(DATING_HISTORY) / "decisions" / (digest({"old": old, "new": updated}) + ".json"),
        {"old": old, "new": updated, "withdrawal": record},
    )
    atomic_json(log, withdrawals + [record])
    atomic_json(dest, updated)
    return {"withdrawn_claim": match[0]["claim_id"], "publish_required": True}


def readopt(reviewer, note, subjects=None):
    """Re-adopt stale decisions unchanged, with `note` appended to their rationale.

    A decision goes stale when its subject's evidence, catalog mapping or
    DECISION_RULES change. When a reviewer has seen that change and keeps the choice,
    the same claim is recorded again against the current digest. Only decisions
    whose claim is still eligible are re-adopted; the others are returned unchanged
    for a new decision. `subjects` limits it to those subject ids."""
    if not reviewer.strip() or not note.strip():
        raise ValueError("reviewer and note required")
    bundle = build_bundle()
    done, left = [], []
    for r in bundle["rejected_decisions"]:
        d = r["decision"]
        if subjects is not None and d["subject_id"] not in subjects:
            continue
        if not r["reason"].startswith("stale decision"):
            left.append({"subject_id": d["subject_id"], "event": d["event"], "reason": r["reason"]})
            continue
        raw = {
            **{k: v for k, v in d.items() if k != "kind"},
            "evidence_digest": bundle_decision_digest(bundle, d["subject_id"]),
            "reviewer": reviewer,
            "rationale": d["rationale"] + " " + note,
        }
        try:
            decide_record(raw, bundle)
            done.append(f"{d['subject_id']}/{d['event']}")
        except ValueError as e:
            left.append({"subject_id": d["subject_id"], "event": d["event"], "reason": str(e)})
    return {"readopted": done, "needs_decision": left}
