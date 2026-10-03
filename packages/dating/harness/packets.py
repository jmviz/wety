"""Validate bounded editorial source packets separately from model extraction.

A packet's hashes and spans establish reproducibility, not historical correctness
or independent annotation. Editorial inputs belong in the dating workspace.
"""

import hashlib
import json
from pathlib import Path
from .config import DATA_DIR
from .evidence import EvidenceClaim, RelativeConstraint, normalize_time
from .store import load_sources
from workflow_tools.paths import resolve


def validate_packet(path, imported=None, imported_relations=None):
    path = Path(path)
    errors = []
    try:
        p = json.loads(path.read_text())
        if p["schema_version"] != 1:
            raise ValueError("unsupported packet version")
        if not p["reviewer"].strip():
            raise ValueError("packet reviewer required")
        claims_path = (path.parent / p["claims_file"]).resolve()
        if not claims_path.is_relative_to(path.parent.resolve()):
            raise ValueError("claims path escapes packet")
        raw = claims_path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != p["claims_sha256"]:
            errors.append("packet claims hash mismatch")
        for review_file in p.get("review_files", []):
            if (
                hashlib.sha256(resolve(review_file["path"]).read_bytes()).hexdigest()
                != review_file["sha256"]
            ):
                errors.append("packet review file hash mismatch " + review_file["path"])
        payload = json.loads(raw)
        sources = load_sources()
        texts = {}
        by_source = {}
        for s in p["source_reviews"]:
            sid = s["source_id"]
            if sid in by_source:
                errors.append("duplicate packet source " + sid)
            by_source[sid] = s
            if sources.get(sid, {}).get("sha256") != s["text_sha256"]:
                errors.append("packet source no longer matches registry " + sid)
            for kind in ("raw", "text"):
                data = resolve(s[kind + "_path"]).read_bytes()
                if hashlib.sha256(data).hexdigest() != s[kind + "_sha256"]:
                    errors.append("packet " + kind + " hash mismatch " + sid)
                if kind == "text":
                    texts[sid] = data.decode("utf-8")

        def spans(record):
            sid = record["source_id"]
            if sid not in by_source:
                errors.append("unregistered packet source " + sid)
                return
            if not record.get("spans"):
                errors.append("packet assertion missing spans")
                return
            for sp in record["spans"]:
                if (
                    sp["text_sha256"] != by_source[sid]["text_sha256"]
                    or sp["start"] < 0
                    or sp["end"] <= sp["start"]
                    or texts[sid][sp["start"] : sp["end"]] != sp["quote"]
                ):
                    errors.append("packet span mismatch " + record.get("id", "?"))

        claims = {}
        for raw in payload["claims"]:
            c = EvidenceClaim.model_validate(raw).model_dump(mode="json")
            if c["id"] in claims:
                errors.append("duplicate packet claim " + c["id"])
            claims[c["id"]] = c
            spans(c)
            if (
                c["time"]["normalization"] == "program_verified"
                and normalize_time(c["time"]["original"]).model_dump() != c["time"]
            ):
                errors.append("packet time normalization mismatch " + c["id"])
            if imported is not None and imported.get(c["id"]) != c:
                errors.append("packet claim missing or changed in registry " + c["id"])
        for s in by_source.values():
            actual = {c["id"] for c in claims.values() if c["source_id"] == s["source_id"]}
            if actual != set(s["claim_ids"]):
                errors.append("packet source claim inventory mismatch " + s["source_id"])
        finding_ids = [f["id"] for f in p["findings"]]
        if len(set(finding_ids)) != len(finding_ids):
            errors.append("duplicate packet finding")
        for c in claims.values():
            if c["provenance"].get("packet_id") != p["id"]:
                errors.append("claim belongs to another packet " + c["id"])
            if any(f not in finding_ids for f in c["provenance"].get("review_findings", [])):
                errors.append("claim references missing packet finding " + c["id"])
        for f in p["findings"]:
            if not f["interpretation"].strip():
                errors.append("empty packet interpretation")
            spans(f)
        if "contact_case_file" in p:
            from .contact_cases import validate_contact_case

            case_path = (path.parent / p["contact_case_file"]).resolve()
            if not case_path.is_relative_to(path.parent.resolve()):
                raise ValueError("contact case path escapes packet")
            case_raw = case_path.read_bytes()
            if hashlib.sha256(case_raw).hexdigest() != p["contact_case_sha256"]:
                errors.append("packet contact case hash mismatch")
            errors.extend(validate_contact_case(json.loads(case_raw), finding_ids))
            # Conditional cases cannot piggyback an unconditional registry import.
            if payload["claims"] or payload["relations"]:
                errors.append(
                    "contact scenario packet must not export registry claims or relations"
                )
        for raw in payload["relations"]:
            r = RelativeConstraint.model_validate(raw)
            if any(i not in claims for i in [r.before_event, r.after_event, *r.basis_claim_ids]):
                errors.append("packet relation lacks endpoint/basis claim")
            if (
                imported_relations is not None
                and r.model_dump(mode="json") not in imported_relations
            ):
                errors.append("packet relation missing or changed in registry")
        return {
            "packet_id": p["id"],
            "reviewer": p["reviewer"],
            "evaluation_split": p["evaluation_split"],
            "independent_review": p["independent_review"],
            "source_reviews": p["source_reviews"],
            "review_independence": p.get("review_independence"),
            "packet_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "finding_counts": {
                sid: sum(f["source_id"] == sid for f in p["findings"]) for sid in by_source
            },
            "claims": len(claims),
            "findings": len(p["findings"]),
            "relations": len(payload["relations"]),
            "errors": errors,
        }
    except (ValueError, KeyError, TypeError, OSError) as e:
        return {"packet_id": str(path), "source_reviews": [], "errors": [str(e)]}


def packet_summaries():
    return [
        validate_packet(p)
        for p in sorted((Path(DATA_DIR) / "_research/packets").glob("*/packet.json"))
    ]


def main():
    import argparse
    from .registry import load_evidence

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path")
    parser.add_argument("--require-imported", action="store_true")
    args = parser.parse_args()
    relations_path = Path(DATA_DIR) / "_constraints/relations.json"
    relations = json.loads(relations_path.read_text()) if relations_path.exists() else []
    result = validate_packet(
        args.path,
        {c["id"]: c for c in load_evidence()} if args.require_imported else None,
        relations if args.require_imported else None,
    )
    print(json.dumps({k: v for k, v in result.items() if k != "source_reviews"}, indent=2))
    raise SystemExit(bool(result["errors"]))


if __name__ == "__main__":
    main()
