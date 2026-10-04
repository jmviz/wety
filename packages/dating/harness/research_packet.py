"""Reviewed research run -> editorial packet -> registry, reproducibly.

`build-packet RUN REVIEW` turns a frontier run's `assertions.jsonl` and a reviewer's
`review.json` into a packet (`packets/<id>/{packet.json, claims.json}`). It registers
the run's sources with their parsed-text hashes and copies those texts into the
retrieved-text store, which is what supported spans are validated against. Import
the packet with `import-evidence`, publish, then `adopt REVIEW` records the review's
editorial decisions against the current decision digests.

review.json:
  packet_id, claim_prefix, reviewer, scope, review_independence, retrieved
  sources: {run source_id: registry metadata (id, title, authors, year, doi, ...)}
  claims: [{assertion, status, rationale?, correct?, mapping?, mapping_basis?,
            corrections?, limitations?, quote?, start?, locator?}]
  adopt: {code: {assertion, rationale, reviewer?} or a list of them, one per event
         (e.g. first and last attestation)}, decision_reviewer
Every assertion of the run must appear in `claims`: rejecting one is a recorded
status (candidate), never a silent omission.
"""

import hashlib
import json
import shutil
from pathlib import Path

from workflow_tools.paths import REPO

from .config import CACHE_DIR, DATA_DIR
from .evidence import EvidenceClaim, normalize_time
from .io_utils import atomic_json, digest

LANGUAGES = REPO / "data/artifacts/wiktionary-export/legacy/files/languages.json"
COMPLETENESS = "Only the passages cited by the claims were checked; not an exhaustive reading."


def sha(data):
    return hashlib.sha256(data).hexdigest()


def registry_raw(native):
    """A researcher's native_time -> normalize_time input. No numbers change: only
    the registry's BCE ordering (numeric lower < upper, swapped on conversion)."""
    raw = {k: v for k, v in native.items() if k != "endpoint" and v is not None}
    if (
        raw.get("form") in ("duration", "event_range")
        and raw.get("unit") == "BCE"
        and raw.get("lower") is not None
        and raw.get("upper") is not None
    ):
        raw["lower"], raw["upper"] = sorted((raw["lower"], raw["upper"]))
    return raw


def locate(text, quote, item):
    start = text.find(quote)
    if start < 0:
        raise ValueError("quote not in retained text: " + item["assertion"])
    if "start" in item:
        start = item["start"]
    elif text.find(quote, start + 1) >= 0:
        raise ValueError("quote not unique; review must give start: " + item["assertion"])
    if text[start : start + len(quote)] != quote:
        raise ValueError("review start does not match quote: " + item["assertion"])
    return start


def build(run_dir, review, languages=None, texts_dir=None):
    """Pure: the packet, its claims and the source records to register."""
    run_dir = Path(run_dir)
    texts_dir = Path(texts_dir or Path(CACHE_DIR) / "texts")
    languages = languages or json.loads(LANGUAGES.read_text())
    rows = {
        r["id"]: r
        for r in (
            json.loads(x)
            for x in (run_dir / "assertions.jsonl").read_text().splitlines()
            if x.strip()
        )
    }
    reviewed = [item["assertion"] for item in review["claims"]]
    if len(set(reviewed)) != len(reviewed) or set(reviewed) != set(rows):
        raise ValueError("review must cover every run assertion exactly once")
    state = json.loads((run_dir / "tools-state.json").read_text())
    meta = review["sources"]
    run_path = (
        str(run_dir.resolve().relative_to(REPO))
        if run_dir.resolve().is_relative_to(REPO)
        else str(run_dir)
    )
    claims, source_reviews, registrations, texts = [], {}, {}, {}
    for item in review["claims"]:
        row = {**rows[item["assertion"]], **item.get("correct", {})}
        sid = row["source_id"]
        src = state["sources"][sid]
        text_path = run_dir / "evidence" / src["text_file"]
        text_bytes = text_path.read_bytes()
        tsha = sha(text_bytes)
        if tsha != src["text_sha256"]:
            raise ValueError("retained text changed: " + sid)
        quote = item.get("quote", row["quote"])
        start = locate(text_bytes.decode(), quote, item)
        reg_id = meta[sid]["id"]
        registrations[reg_id] = {
            **{k: v for k, v in meta[sid].items() if k != "id"},
            "id": reg_id,
            "url": src["url"],
            "sha256": tsha,
            "raw_sha256": src["raw_sha256"],
            "retrieval_quality": "fulltext_candidate",
            "retrieved": item.get("retrieved") or review["retrieved"],
        }
        if src.get("holding"):
            # A maintainer-supplied file: say so; never imply a download history.
            h = src["holding"]
            registrations[reg_id]["acquisition"] = {
                "origin": h["origin"],
                "holding": h["id"],
                "extent": h["extent"],
                "zotero": h.get("zotero"),
                "text_kind": src["kind"],
            }
        texts[tsha] = text_path
        lang = languages[row["code"]]
        status = item["status"]
        subject = {
            "id": "wikt:" + row["code"],
            "type": "reconstructed_stage"
            if lang["kind"] == "reconstructed"
            else "wiktionary_stage",
            "name": lang["canonicalName"],
            "mapping": item.get("mapping", row["subject"]["mapping"]),
            "taxa": [],
            "mapping_basis": item.get("mapping_basis", row["subject"].get("mapping_basis")),
            "source_label": row["subject"].get("source_label"),
            "mapping_certainty": "checked" if status == "supported" else "proposed",
        }
        locator = item.get("locator") or json.dumps(row.get("locator"), ensure_ascii=False)
        body = {
            "subject": subject,
            "event": row["event"],
            "role": row["role"],
            "method": row["method"],
            "time": normalize_time(registry_raw(row["native_time"])).model_dump(mode="json"),
            "source_id": reg_id,
            "spans": [
                {
                    "text_sha256": tsha,
                    "start": start,
                    "end": start + len(quote),
                    "quote": quote,
                    "locator": locator,
                }
            ],
        }
        # Scope evidence quoted once per catalog target is one claim per target.
        cid = (
            review["claim_prefix"]
            + digest({**body, "catalog_target": row["target"]} if row.get("target") else body)[:40]
        )
        claims.append(
            EvidenceClaim.model_validate(
                {
                    "id": cid,
                    **body,
                    "study_id": ("doi:" + meta[sid]["doi"])
                    if meta[sid].get("doi")
                    else "source:" + reg_id,
                    "assessment": {
                        "textual": "exact_span",
                        "semantic": "checked" if status == "supported" else "pending",
                        "mapping": "checked" if status == "supported" else "pending",
                        "limitations": row.get("qualifications", []) + item.get("limitations", []),
                        "reviewer": review["reviewer"] if status == "supported" else None,
                        "rationale": item.get("rationale"),
                    },
                    "provenance": {
                        "packet_id": review["packet_id"],
                        "assertion_id": row["id"],
                        "research_run": run_path,
                        "verifies": row.get("verifies", []),
                        **({"catalog_target": row["target"]} if row.get("target") else {}),
                        "review_corrections": item.get("corrections", []),
                        "review_findings": [],
                    },
                    "status": status,
                }
            ).model_dump(mode="json")
        )
        source_reviews.setdefault(
            reg_id,
            {
                "source_id": reg_id,
                "title": meta[sid].get("title"),
                "doi": meta[sid].get("doi", ""),
                "completeness": COMPLETENESS,
                "raw_path": src["raw_path"],
                "raw_sha256": src["raw_sha256"],
                "text_path": str((texts_dir / (tsha + ".txt")).relative_to(REPO))
                if (texts_dir / (tsha + ".txt")).is_relative_to(REPO)
                else str(texts_dir / (tsha + ".txt")),
                "text_sha256": tsha,
                "claim_ids": [],
                "status": "bounded_source_review",
                "review_scope": meta[sid].get("review_scope", "Passages cited by the claims only."),
            },
        )["claim_ids"].append(cid)
    claims_bytes = json.dumps(
        {"claims": claims, "relations": []}, indent=2, ensure_ascii=False
    ).encode()
    packet = {
        "schema_version": 1,
        "id": review["packet_id"],
        "reviewer": review["reviewer"],
        "scope": review["scope"],
        "claims_file": "claims.json",
        "claims_sha256": sha(claims_bytes),
        "source_reviews": list(source_reviews.values()),
        "findings": review.get("findings", []),
        "evaluation_split": "development",
        "independent_review": False,
        "review_independence": review["review_independence"],
        "historical_adjudication": "not_completed",
    }
    return {
        "packet": packet,
        "claims": claims,
        "claims_bytes": claims_bytes,
        "registrations": registrations,
        "texts": texts,
    }


def write(result, packet_dir=None, texts_dir=None):
    from .store import load_sources, save_sources

    texts_dir = Path(texts_dir or Path(CACHE_DIR) / "texts")
    packet_dir = Path(packet_dir or Path(DATA_DIR) / "_research/packets" / result["packet"]["id"])
    sources = load_sources()
    for reg_id, record in result["registrations"].items():
        old = sources.get(reg_id)
        if old and old.get("sha256") != record["sha256"]:
            raise ValueError("registered source has different text; use a new source id: " + reg_id)
        if not old:
            sources[reg_id] = record
    texts_dir.mkdir(parents=True, exist_ok=True)
    for tsha, path in result["texts"].items():
        if not (texts_dir / (tsha + ".txt")).exists():
            shutil.copyfile(path, texts_dir / (tsha + ".txt"))
    save_sources(sources)
    packet_dir.mkdir(parents=True, exist_ok=True)
    (packet_dir / "claims.json").write_bytes(result["claims_bytes"])
    atomic_json(packet_dir / "packet.json", result["packet"])
    return {
        "packet": str(packet_dir),
        "claims": len(result["claims"]),
        "supported": sum(c["status"] == "supported" for c in result["claims"]),
        "sources": len(result["registrations"]),
    }


def adopt(review):
    """Record the review's editorial defaults against current decision digests.

    A code takes one choice, or a list of choices with distinct events (e.g. first and
    last attestation); results are keyed by code, or code/event for a list."""
    from .registry import build_bundle, bundle_decision_digest
    from .review import decide_record

    bundle = build_bundle()
    by_assertion = {
        c["provenance"].get("assertion_id"): c
        for c in bundle["claims"]
        if c["provenance"].get("packet_id") == review["packet_id"]
    }
    current = {(d["subject_id"], d["event"]): d for d in bundle["decisions"]}
    results = {}
    for code, choices in review.get("adopt", {}).items():
        listed = isinstance(choices, list)
        for choice in choices if listed else [choices]:
            claim = by_assertion.get(choice["assertion"])
            if not claim:
                raise ValueError("adopted assertion not imported: " + choice["assertion"])
            if claim["subject"]["id"] != "wikt:" + code:
                raise ValueError("adopted claim is about another code: " + code)
            key = f"{code}/{claim['event']}" if listed else code
            if key in results:
                raise ValueError("one adopted claim per code and event: " + key)
            decision = {
                "subject_id": claim["subject"]["id"],
                "event": claim["event"],
                "claim_id": claim["id"],
                "evidence_digest": bundle_decision_digest(bundle, claim["subject"]["id"]),
                "reviewer": choice.get("reviewer") or review["decision_reviewer"],
                "rationale": choice["rationale"],
                "dependencies": [],
            }
            existing = current.get((decision["subject_id"], decision["event"]))
            if existing and {**existing, "kind": None} == {**decision, "kind": None}:
                results[key] = "unchanged"
                continue
            decide_record(decision, bundle)
            results[key] = claim["id"]
    return results
