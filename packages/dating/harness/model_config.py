"""Dating-specific, backend-neutral model configuration.

Old dictionaries are accepted by accessors for embedded callers; on-disk configs
must use v2. Historical rationale is documentation, not executable selection.
"""

import copy
import json
from pathlib import Path

BACKEND_NAMES = {"openrouter", "anthropic", "claude-agent", "codex"}


def backend_config(cfg, backend):
    if cfg.get("schema_version") == 2:
        return cfg["backends"][backend]
    return cfg if backend == "openrouter" else cfg.get(backend.replace("-", "_"), {})


def role_config(cfg, role, backend):
    r = cfg["roles"][role]
    if cfg.get("schema_version") == 2:
        try:
            return r["backends"][backend]
        except KeyError:
            raise ValueError(
                f"configure roles.{role}.backends.{backend}.model in the dating LLM config"
            ) from None
    if backend in ("anthropic", "claude-agent"):
        return {"max_tokens": r.get("max_tokens", 16000), **r.get("claude", {})}
    return r.get("codex", {}) if backend == "codex" else r


def validate_config(cfg):
    if cfg.get("schema_version") != 2:
        raise ValueError(
            "dating LLM config requires schema_version 2; see packages/dating/llm.config.json"
        )
    backends = cfg.get("backends", {})
    if not backends or set(backends) - BACKEND_NAMES:
        raise ValueError("unknown or missing configured backends")
    default = cfg.get("default_backend")
    if default not in backends:
        raise ValueError("default backend is not configured")
    if not cfg.get("roles"):
        raise ValueError("model roles are required")
    for role, values in cfg["roles"].items():
        profiles = values.get("backends", {})
        if values.get("backend", default) not in profiles:
            raise ValueError(f"role {role} selected backend has no model settings")
        for backend, settings in profiles.items():
            if (
                backend not in backends
                or not isinstance(settings.get("model"), str)
                or not settings["model"].strip()
            ):
                raise ValueError(f"invalid model settings for {role}/{backend}")
            if "schema_profile" in settings and (
                backend != "openrouter"
                or settings["schema_profile"] not in ("canonical", "minimal")
            ):
                raise ValueError("schema_profile is an OpenRouter canonical/minimal wire setting")
    return cfg


def read_config(path):
    return validate_config(json.loads(Path(path).read_text()))


def migrate_config(old):
    """Pure conversion used by the one-time migration and regression checks."""
    cfg = {
        "schema_version": 2,
        "default_backend": "openrouter",
        "backends": {
            "openrouter": {
                k: copy.deepcopy(old[k]) for k in ("base_url", "headers", "defaults") if k in old
            },
            "anthropic": {},
            "claude-agent": copy.deepcopy(old.get("claude_agent", {})),
            "codex": copy.deepcopy(old.get("codex", {})),
        },
        "roles": {},
    }
    for role, settings in old["roles"].items():
        router = {
            k: copy.deepcopy(settings[k])
            for k in (
                "model",
                "fallbacks",
                "batch_model",
                "reasoning",
                "max_tokens",
                "cache_system_prompt",
                "provider",
            )
            if k in settings
        }
        claude = {
            k: copy.deepcopy(settings["claude"][k])
            for k in ("model", "effort")
            if k in settings.get("claude", {})
        }
        claude["max_tokens"] = settings.get("max_tokens", 16000)
        cfg["roles"][role] = {
            "backend": settings.get("backend", "openrouter"),
            "validation_status": settings.get(
                "validation_status", "provisional; not evaluated for dating"
            ),
            "backends": {
                "openrouter": router,
                "anthropic": copy.deepcopy(claude),
                "claude-agent": claude,
                "codex": copy.deepcopy(settings.get("codex", {})),
            },
        }
    return validate_config(cfg)
