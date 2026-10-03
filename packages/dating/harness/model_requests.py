"""Immutable, replayable model inputs and outputs, without credentials.

These are exact application prompts and schemas, not provider wire transcripts.
Requested settings and reported execution identity are deliberately separate.
"""

import json
from pathlib import Path
import re

from .config import DATA_DIR
from .io_utils import atomic_json, digest, locked
from .llm_backends import strict_schema, wire_schema
from .model_config import role_config, backend_config

REQUEST_DIR = Path(DATA_DIR) / "_requests"


def _path(kind, ref):
    if not isinstance(ref, str) or not re.fullmatch(r"[0-9a-f]{64}", ref):
        raise ValueError("invalid model snapshot reference")
    return REQUEST_DIR / kind / (ref + ".json")


def _save(kind, payload):
    ref = digest(payload)
    path = _path(kind, ref)
    with locked(path):
        if path.exists():
            if json.loads(path.read_text()) != payload:
                raise ValueError("model snapshot content mismatch")
        else:
            atomic_json(path, payload)
    return ref


def load_request(ref):
    payload = json.loads(_path("inputs", ref).read_text())
    if digest(payload) != ref or payload.get("version") != 1:
        raise ValueError("model request snapshot digest/version mismatch")
    return payload


def freeze_request(
    role, family, item, system, user, schema, backend, *, run=None, batch=False, context=None
):
    cfg = getattr(backend, "cfg", {})
    role_cfg = role_config(cfg, role, backend.name)
    # Never serialize the backend/client, headers, environment, or arbitrary config.
    keys = (
        "model",
        "batch_model",
        "fallbacks",
        "reasoning",
        "effort",
        "max_tokens",
        "cache_system_prompt",
        "provider",
        "schema_repair",
        "schema_profile",
    )
    settings = {k: role_cfg[k] for k in keys if k in role_cfg}
    transport = backend_config(cfg, backend.name)
    defaults = {
        k: transport.get("defaults", {})[k]
        for k in ("provider", "temperature", "usage", "plugins", "retries")
        if k in transport.get("defaults", {})
    }
    runtime = {
        k: transport[k]
        for k in (
            "executable",
            "auth_credentials_store",
            "timeout_seconds",
            "on_usage_limit",
            "max_wait_seconds",
            "max_attempts",
            "fallback_backend",
        )
        if k in transport
    }
    payload = {
        "version": 1,
        "role": role,
        "family": family,
        "item": item,
        "run": run,
        "system": system,
        "user": user,
        "schema": strict_schema(schema),
        "prompt_sha256": digest({"system": system, "user": user}),
        "requested_execution": {
            "backend": backend.name,
            "batch": batch,
            "role_settings": settings,
            "defaults": defaults,
            "runtime_policy": runtime,
            "config_sha256": digest(cfg),
            "settings_scope": "selected requested settings; actual provider/fallback effort may be unreported",
        },
        "context": context or {},
    }
    if backend.name == "openrouter" and role_cfg.get("schema_profile", "canonical") != "canonical":
        payload["wire_schema"] = wire_schema(schema, role_cfg["schema_profile"])
    return _save("inputs", payload)


def record_response(request_ref, result):
    load_request(request_ref)
    return _save(
        "outputs",
        {
            "version": 1,
            "request_ref": request_ref,
            "text": result.text,
            "parsed": result.parsed.model_dump(mode="json"),
            "reported_execution": result.log_fields(),
            "execution_trace": {
                k: result.raw[k]
                for k in ("attempts", "model_usage", "num_turns", "rate_limit")
                if k in result.raw
            },
        },
    )


def record_failure(request_ref, error, retained=None):
    """Retain explicitly selected response evidence; never serialize a client/env."""
    load_request(request_ref)
    retained = retained or {}
    return _save(
        "failures",
        {
            "version": 1,
            "request_ref": request_ref,
            "error_type": type(error).__name__,
            "error": str(error),
            **{k: retained[k] for k in ("text", "execution", "execution_trace") if k in retained},
        },
    )


def recover_failed_extraction(request_ref, failure_ref):
    """Retain sidecar proposals; never replace a successful historical extraction."""
    from .extraction_recovery import inspect_response, source_segment

    request = load_request(request_ref)
    failure = json.loads(_path("failures", failure_ref).read_text())
    if digest(failure) != failure_ref or failure["request_ref"] != request_ref:
        raise ValueError("failure identity mismatch")
    if request["role"] != "extract" or not failure.get("text"):
        return None
    evidence = request.get("context", {}).get("evidence_text")
    if evidence is None:
        return None
    from .llm import ExtractedClaim, ResearchGap, ResearchConflict

    validator_inputs = {
        "modules": {
            name: Path(__file__).with_name(name).read_text()
            for name in (
                "extraction_recovery.py",
                "llm.py",
                "time_interpretation.py",
                "schema.py",
                "evidence.py",
            )
        },
        "schemas": {
            model.__name__: model.model_json_schema()
            for model in (ExtractedClaim, ResearchGap, ResearchConflict)
        },
    }
    validator_ref = _save("validators", {"version": 1, **validator_inputs})
    recovery = inspect_response(failure["text"], [source_segment(evidence, request["item"])])
    return _save(
        "recoveries",
        {
            "version": 1,
            "request_ref": request_ref,
            "failure_ref": failure_ref,
            "validator_ref": validator_ref,
            "validator_sha256": digest(
                Path(__file__).with_name("extraction_recovery.py").read_text()
            ),
            "recovery": recovery,
            "status": "proposals_only_incomplete_extraction",
        },
    )
