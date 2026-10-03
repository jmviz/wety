"""OpenRouter Batch API flow for the latency-insensitive stages (screen,
extract): submit the same request bodies on the `:batch` model variant at 50%
price, record the batch id under <family>/batches.json, poll later with
`dating batch-poll FAMILY`, and feed results through the very same
post-processing as the synchronous path."""

from __future__ import annotations
from .io_utils import atomic_json

import json
import os

from .config import family_dir, now
from .store import append_log
from .io_utils import digest


def request_matches(batch, cid, system, user):
    return batch.get("request_digests", {}).get(cid) == digest({"system": system, "user": user})


def _batches_path(family: str) -> str:
    return os.path.join(family_dir(family), "batches.json")


def _load(family: str) -> list[dict]:
    p = _batches_path(family)
    if not os.path.exists(p):
        return []
    with open(p, encoding="utf-8") as f:
        return json.load(f)


def _save(family: str, batches: list[dict]) -> None:
    from .io_utils import atomic_json

    atomic_json(_batches_path(family), batches)


def submit(
    family: str,
    role: str,
    items: list[tuple[str, str, str]],
    schema,
    backend,
    run: str | None = None,
    chunk: int = 200,
    request_refs=None,
) -> int:
    from .model_requests import freeze_request, load_request
    from .llm_backends import strict_schema

    if role not in ("screen", "extract") or chunk <= 0:
        raise ValueError("invalid batch role or chunk size")
    if len({cid for cid, _, _ in items}) != len(items):
        raise ValueError("duplicate batch item identity")
    if not hasattr(backend, "batch_submit"):
        raise SystemExit(
            f"backend {backend.name} has no batch API; use DATING_LLM_BACKEND=openrouter for --batch"
        )
    batches = _load(family)
    n = 0
    for i in range(0, len(items), chunk):
        part = items[i : i + chunk]
        from .llm import screen_system

        refs = {}
        for cid, system, user in part:
            ref = (request_refs or {}).get(cid)
            if not ref:
                if role == "extract":
                    raise ValueError("extraction batch requires frozen evidence context")
                ref = freeze_request(
                    role, family, cid, system, user, schema, backend, run=run, batch=True
                )
            request = load_request(ref)
            if (
                request["role"],
                request["family"],
                request["item"],
                request["system"],
                request["user"],
                request["schema"],
            ) != (role, family, cid, system, user, strict_schema(schema)):
                raise ValueError("batch request snapshot mismatch")
            if (
                not request["requested_execution"]["batch"]
                or request["requested_execution"]["backend"] != backend.name
            ):
                raise ValueError("batch execution snapshot mismatch")
            refs[cid] = ref
        resp = backend.batch_submit(role, part, schema)
        bid = resp.get("id") or resp.get("data", {}).get("id")
        batches.append(
            {
                "id": bid,
                "role": role,
                "run": run,
                "submitted": now(),
                "items": [cid for cid, _, _ in part],
                "screen_policy_digest": digest(screen_system([])),
                "request_systems": {cid: system for cid, system, _ in part},
                "request_refs": refs,
                "request_digests": {
                    cid: digest({"system": system, "user": user}) for cid, system, user in part
                },
                "status": resp.get("status", "submitted"),
            }
        )
        append_log(
            family,
            f"{role}_log.jsonl",
            {"batch_submitted": bid, "role": role, "run": run, "items": len(part)},
        )
        n += len(part)
    _save(family, batches)
    return n


def poll(family: str) -> list[dict]:
    from .llm import (
        Extraction,
        ScreenDecision,
        _apply_screen,
        _ingest_extraction,
        extract_system,
        extract_user,
        screen_system,
        screen_user,
    )
    from .llm_backends import get_backend
    from .retrieve import text_for
    from .model_requests import load_request, record_response
    from .llm_backends import strict_schema

    backend = get_backend("openrouter")
    batches = _load(family)
    out = []
    path = os.path.join(family_dir(family), "candidates.json")
    with open(path, encoding="utf-8") as f:
        cands = json.load(f)
    by_key = {c["key"]: c for c in cands}
    by_src = {c.get("source_id"): c for c in cands if c.get("source_id")}
    for b in batches:
        if b.get("status") in ("completed", "failed", "expired", "cancelled") and b.get("ingested"):
            continue
        st = backend.batch_status(b["id"])
        b["status"] = st.get("status", st.get("data", {}).get("status"))
        summary = {"id": b["id"], "role": b["role"], "status": b["status"]}
        if b["status"] == "completed":
            schema = ScreenDecision if b["role"] == "screen" else Extraction
            results = backend.batch_results(b["id"], schema)
            stats = {"ok": 0, "error": 0, "claims": 0, "verified": 0, "unverified": 0, "placed": 0}
            for missing in set(b["items"]) - set(results):
                stats["error"] += 1
                append_log(
                    family,
                    f"{b['role']}_log.jsonl",
                    {"batch": b["id"], "item": missing, "error": "missing batch result; resubmit"},
                )
            for cid, res in results.items():
                if isinstance(res, Exception):
                    stats["error"] += 1
                    append_log(
                        family,
                        f"{b['role']}_log.jsonl",
                        {"batch": b["id"], "item": cid, "error": str(res)},
                    )
                    continue
                try:
                    if cid not in b["items"]:
                        raise ValueError("unexpected batch result identity")
                    ref = b.get("request_refs", {}).get(cid)
                    request = load_request(ref)
                    if (request["family"], request["role"], request["item"], request["schema"]) != (
                        family,
                        b["role"],
                        cid,
                        strict_schema(schema),
                    ):
                        raise ValueError("batch request identity/schema mismatch; resubmit")
                    if request["prompt_sha256"] != b.get("request_digests", {}).get(cid):
                        raise ValueError("batch request fingerprint mismatch; resubmit")
                    response_ref = record_response(ref, res)
                    # Preserve even stale output, but never activate it as current extraction.
                    append_log(
                        family,
                        f"{b['role']}_log.jsonl",
                        {
                            "batch": b["id"],
                            "item": cid,
                            "request_ref": ref,
                            "response_ref": response_ref,
                        },
                    )
                except (ValueError, OSError, KeyError) as e:
                    stats["error"] += 1
                    append_log(
                        family,
                        f"{b['role']}_log.jsonl",
                        {"batch": b["id"], "item": cid, "error": str(e)},
                    )
                    continue
                if b["role"] == "screen":
                    c = by_key.get(cid)
                    if c:
                        # Legacy batches have no auditable prompt; do not label their
                        # responses with the current screening policy.
                        if b.get("screen_policy_digest") != digest(
                            screen_system([])
                        ) or not request_matches(
                            b, cid, b.get("request_systems", {}).get(cid), screen_user(c)
                        ):
                            stats["error"] += 1
                            append_log(
                                family,
                                "screen_log.jsonl",
                                {
                                    "batch": b["id"],
                                    "item": cid,
                                    "error": "stale or unrecorded batch request; resubmit",
                                },
                            )
                            continue
                        _apply_screen(c, res)
                        c["screen"].update(request_ref=ref, response_ref=response_ref)
                        stats["ok"] += 1
                        append_log(
                            family,
                            "screen_log.jsonl",
                            {
                                "key": cid,
                                "title": c.get("title"),
                                "screen": c["screen"],
                                **res.log_fields(),
                            },
                        )
                else:
                    c = by_src.get(cid)
                    if c:
                        if not request_matches(
                            b, cid, extract_system(family), extract_user(c, text_for(c) or "")
                        ):
                            stats["error"] += 1
                            append_log(
                                family,
                                "extract_log.jsonl",
                                {
                                    "batch": b["id"],
                                    "item": cid,
                                    "error": "stale or unrecorded batch request; resubmit",
                                },
                            )
                            continue
                        _ingest_extraction(family, c, res, ref, b.get("run") or "batch", stats)
                        stats["ok"] += 1
                if (by_key if b["role"] == "screen" else by_src).get(cid) is None:
                    stats["error"] += 1
                    append_log(
                        family,
                        f"{b['role']}_log.jsonl",
                        {
                            "batch": b["id"],
                            "item": cid,
                            "error": "candidate no longer present; result retained without ingestion",
                        },
                    )
            b["ingested"] = now()
            summary.update(stats)
        out.append(summary)
    atomic_json(path, cands)
    _save(family, batches)
    return out
