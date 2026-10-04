"""LLM backends. One interface, four implementations, one config
(packages/dating/llm.config.json, or DATING_LLM_CONFIG):

    openrouter    OpenAI-compatible chat completions on OpenRouter (ZDR, provider
                  routing, structured outputs, caching, fallbacks, batch)
    anthropic     the Anthropic SDK directly (API key or `ant auth login`)
    claude-agent  the Claude Agent SDK (Claude Code subscription; OAuth token from
                  the macOS keychain or CLAUDE_CODE_OAUTH_TOKEN)
    codex         Codex CLI non-interactive mode (ChatGPT subscription; codex login)

All backends receive exactly the same (role, system, user, schema) and return an
LLMResult with a schema-validated `parsed` object — that is what keeps them in
sync: prompts and schemas live in llm.py only, backends never see anything
role-specific. `dating llm-check` runs one identical probe through every
backend and compares the parsed outputs.

Select with DATING_LLM_BACKEND=openrouter|anthropic|claude-agent|codex (default: openrouter).
"""

from __future__ import annotations

import copy
import json
import os
import re
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from pydantic import BaseModel

from .config import REPO_DIR, load_env
from .model_config import backend_config, role_config, read_config
from .io_utils import digest

CONFIG_PATH = os.path.join(REPO_DIR, "packages", "dating", "llm.config.json")


@dataclass
class LLMResult:
    parsed: Any
    text: str
    model: str
    backend: str
    usage: dict = field(default_factory=dict)
    cost_usd: float | None = None
    id: str | None = None
    raw: dict = field(default_factory=dict)

    def log_fields(self) -> dict:
        return {
            "backend": self.backend,
            "model": self.model,
            "message_id": self.id,
            "usage": self.usage,
            "cost_usd": self.cost_usd,
            **{
                k: self.raw[k]
                for k in (
                    "billing",
                    "effort",
                    "model_source",
                    "provider",
                    "attempt_count",
                    "cost_scope",
                )
                if k in self.raw
            },
        }


def load_config() -> dict:
    path = Path(os.environ.get("DATING_LLM_CONFIG", CONFIG_PATH)).expanduser()
    if not path.is_absolute():
        path = Path(REPO_DIR) / path
    return read_config(path)


# --------------------------------------------------------------------------- #
# strict JSON schema from a pydantic model
# --------------------------------------------------------------------------- #


def strict_schema(model: type[BaseModel]) -> dict:
    """Pydantic JSON schema -> strict structured-output schema: every property
    required (optional ones are already `anyOf [.., null]`), no additional
    properties, no defaults."""
    schema = copy.deepcopy(model.model_json_schema())

    def walk(node: Any) -> None:
        if isinstance(node, dict):
            if node.get("type") == "object" and "properties" in node:
                node["additionalProperties"] = False
                node["required"] = list(node["properties"].keys())
                for prop in node["properties"].values():
                    prop.pop("default", None)
                    # a field with a default but a non-nullable type must still be emitted; that is fine.
            for v in node.values():
                walk(v)
        elif isinstance(node, list):
            for v in node:
                walk(v)

    walk(schema)
    return schema


def wire_schema(model: type[BaseModel], profile: str = "canonical") -> dict:
    """Explicit provider compatibility projection; local validation stays canonical."""
    schema = strict_schema(model)
    if profile == "canonical":
        return schema
    if profile != "minimal":
        raise ValueError("unknown wire schema profile: " + profile)
    # Keep types, references, unions, required fields and enums. These constraints
    # are checked locally even when omitted from provider constrained decoding.
    omitted = {
        "title",
        "description",
        "minLength",
        "maxLength",
        "pattern",
        "format",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minItems",
        "maxItems",
        "uniqueItems",
    }

    def walk(node):
        if isinstance(node, dict):
            return {
                k: (
                    {name: walk(value) for name, value in v.items()}
                    if k in ("properties", "$defs")
                    else walk(v)
                )
                for k, v in node.items()
                if k not in omitted
            }
        if isinstance(node, list):
            return [walk(v) for v in node]
        return node

    return walk(schema)


# --------------------------------------------------------------------------- #
# OpenRouter
# --------------------------------------------------------------------------- #


class OpenRouterBackend:
    name = "openrouter"

    def __init__(self, config: dict | None = None) -> None:
        load_env()
        self.cfg = config or load_config()
        self.key = os.environ.get("OPENROUTER_API_KEY")
        if not self.key:
            raise RuntimeError("OPENROUTER_API_KEY is not set (put it in the repo .env)")

    def _role(self, role: str) -> dict:
        return role_config(self.cfg, role, self.name)

    def build_request(
        self,
        role: str,
        system: str,
        user: str,
        schema: type[BaseModel],
        session_id: str | None = None,
        batch: bool = False,
    ) -> dict:
        r = self._role(role)
        d = backend_config(self.cfg, self.name)["defaults"]
        model = r["batch_model"] if batch else r["model"]
        provider = {**d.get("provider", {}), **r.get("provider", {})}
        sys_content: Any = system
        if r.get("cache_system_prompt") and model.startswith("anthropic/"):
            sys_content = [{"type": "text", "text": system, "cache_control": {"type": "ephemeral"}}]
        body: dict = {
            "model": model,
            "messages": [
                {"role": "system", "content": sys_content},
                {"role": "user", "content": user},
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": schema.__name__,
                    "strict": True,
                    "schema": wire_schema(schema, r.get("schema_profile", "canonical")),
                },
            },
            "provider": provider,
            "max_tokens": r.get("max_tokens", 16000),
        }
        if d.get("temperature") is not None:
            body["temperature"] = d["temperature"]
        if r.get("reasoning"):
            body["reasoning"] = r["reasoning"]
        if not batch and r.get("fallbacks"):
            body["models"] = [model, *r["fallbacks"]]
        if d.get("usage"):
            body["usage"] = d["usage"]
        if d.get("plugins"):
            body["plugins"] = d["plugins"]
        if session_id and not batch:
            body["session_id"] = session_id[:256]
        return body

    def _post(self, path: str, body: dict, timeout: int = 600) -> dict:
        transport = backend_config(self.cfg, self.name)
        d = transport["defaults"]["retries"]
        headers = {
            "Authorization": f"Bearer {self.key}",
            "Content-Type": "application/json",
            **transport.get("headers", {}),
        }
        data = json.dumps(body).encode("utf-8")
        for attempt in range(d["max"] + 1):
            try:
                req = urllib.request.Request(
                    transport["base_url"].replace("/api/v1", "") + path,
                    data=data,
                    headers=headers,
                    method="POST",
                )
                with urllib.request.urlopen(req, timeout=timeout) as resp:
                    return json.load(resp)
            except urllib.error.HTTPError as e:
                text = e.read().decode("utf-8", errors="replace")[:500]
                if e.code in d["retry_on"] and attempt < d["max"]:
                    time.sleep(d["backoff_seconds"] * (2**attempt))
                    continue
                raise RuntimeError(f"OpenRouter HTTP {e.code}: {text}") from e
            except (urllib.error.URLError, TimeoutError) as e:
                if attempt < d["max"]:
                    time.sleep(d["backoff_seconds"] * (2**attempt))
                    continue
                raise
        raise RuntimeError("unreachable")

    def _get(self, path: str) -> dict:
        transport = backend_config(self.cfg, self.name)
        headers = {"Authorization": f"Bearer {self.key}", **transport.get("headers", {})}
        req = urllib.request.Request(
            transport["base_url"].replace("/api/v1", "") + path, headers=headers
        )
        with urllib.request.urlopen(req, timeout=120) as resp:
            return json.load(resp)

    @staticmethod
    def parse_response(resp: dict, schema: type[BaseModel]) -> tuple[Any, str]:
        choice = resp["choices"][0]
        if choice.get("error"):
            raise RuntimeError(f"provider error: {choice['error']}")
        text = choice["message"]["content"] or ""
        return schema.model_validate_json(text), text

    def complete(
        self,
        role: str,
        system: str,
        user: str,
        schema: type[BaseModel],
        session_id: str | None = None,
    ) -> LLMResult:
        self.last_failure = None
        result = self._complete(role, system, user, schema, session_id)
        self.last_failure = None
        return result

    def _complete(
        self,
        role: str,
        system: str,
        user: str,
        schema: type[BaseModel],
        session_id: str | None = None,
    ) -> LLMResult:
        body = self.build_request(role, system, user, schema, session_id)
        resp = self._post("/api/v1/chat/completions", body)
        attempts = [{"request": body, "response": resp}]
        self._retain_attempts(attempts)
        try:
            parsed, text = self.parse_response(resp, schema)
        except Exception as e:  # noqa: BLE001
            if not self._role(role).get("schema_repair", True):
                raise
            # one repair round: the same request plus the validation error
            body2 = copy.deepcopy(body)
            body2["messages"].append(
                {"role": "assistant", "content": resp["choices"][0]["message"].get("content") or ""}
            )
            body2["messages"].append(
                {
                    "role": "user",
                    "content": f"Your JSON did not validate against the schema: {e}. Return the corrected JSON only.",
                }
            )
            resp = self._post("/api/v1/chat/completions", body2)
            attempts.append({"request": body2, "response": resp})
            self._retain_attempts(attempts)
            parsed, text = self.parse_response(resp, schema)
        usages = [a["response"].get("usage", {}) or {} for a in attempts]
        usage = {
            k: sum(u[k] for u in usages)
            for k in ("prompt_tokens", "completion_tokens", "total_tokens")
            if all(isinstance(u.get(k), (int, float)) for u in usages)
        }
        if len(usages) == 1:
            usage = copy.deepcopy(usages[0])
        else:
            cached = [(u.get("prompt_tokens_details") or {}).get("cached_tokens") for u in usages]
            if all(isinstance(n, (int, float)) for n in cached):
                usage["prompt_tokens_details"] = {"cached_tokens": sum(cached)}
        cost = (
            sum(u["cost"] for u in usages)
            if all(isinstance(u.get("cost"), (int, float)) for u in usages)
            else None
        )
        usage["cost"] = cost
        return LLMResult(
            parsed=parsed,
            text=text,
            model=resp.get("model", body["model"]),
            backend=self.name,
            usage=usage,
            cost_usd=cost,
            id=resp.get("id"),
            raw={
                "provider": resp.get("provider"),
                "attempts": attempts,
                "attempt_count": len(attempts),
                "model_source": "response.model"
                if resp.get("model")
                else "requested; response model absent",
                "cost_scope": "sum of returned response costs including schema repair; transport retries unreported",
            },
        )

    def _retain_attempts(self, attempts):
        costs = [(a["response"].get("usage") or {}).get("cost") for a in attempts]
        last = attempts[-1]["response"]
        self.last_failure = {
            "text": last.get("choices", [{}])[0].get("message", {}).get("content"),
            "execution": {
                "backend": self.name,
                "model": last.get("model"),
                "provider": last.get("provider"),
                "cost_usd": sum(costs) if all(isinstance(c, (int, float)) for c in costs) else None,
                "cost_scope": "returned responses only; any later failed request charge remains unknown",
            },
            "execution_trace": {"attempts": copy.deepcopy(attempts)},
        }

    # ---- batch ------------------------------------------------------------- #

    def batch_submit(
        self, role: str, items: list[tuple[str, str, str]], schema: type[BaseModel]
    ) -> dict:
        """items: (custom_id, system, user). Returns the batch object (id, status)."""
        r = self._role(role)
        reqs = []
        for cid, system, user in items:
            body = self.build_request(role, system, user, schema, batch=True)
            body.pop("model")
            reqs.append({"custom_id": cid, "body": body})
        return self._post(
            "/api/beta/batches",
            {"endpoint": "/v1/chat/completions", "model": r["batch_model"], "requests": reqs},
        )

    def batch_status(self, batch_id: str) -> dict:
        return self._get(f"/api/beta/batches/{batch_id}")

    def batch_results(
        self, batch_id: str, schema: type[BaseModel]
    ) -> dict[str, LLMResult | Exception]:
        data = self._get(f"/api/beta/batches/{batch_id}")
        out: dict[str, LLMResult | Exception] = {}
        results = data.get("results") or data.get("data", {}).get("results") or []
        if not results and data.get("results_url"):
            req = urllib.request.Request(
                data["results_url"], headers={"Authorization": f"Bearer {self.key}"}
            )
            with urllib.request.urlopen(req, timeout=300) as resp:
                results = [
                    json.loads(line)
                    for line in resp.read().decode("utf-8").splitlines()
                    if line.strip()
                ]
        for item in results:
            cid = item.get("custom_id")
            if item.get("error") or not item.get("response"):
                out[cid] = RuntimeError(str(item.get("error")))
                continue
            resp = item["response"].get("body", item["response"])
            try:
                parsed, text = self.parse_response(resp, schema)
                usage = resp.get("usage", {}) or {}
                out[cid] = LLMResult(
                    parsed=parsed,
                    text=text,
                    model=resp.get("model", ""),
                    backend=self.name + ":batch",
                    usage=usage,
                    cost_usd=usage.get("cost"),
                    id=resp.get("id"),
                    raw={
                        "provider": resp.get("provider"),
                        "model_source": "response.model" if resp.get("model") else "unreported",
                        "attempts": [{"request": None, "response": resp}],
                        "cost_scope": "returned batch response; internal retries unreported",
                    },
                )
            except Exception as e:  # noqa: BLE001
                out[cid] = e
        return out


# --------------------------------------------------------------------------- #
# Anthropic SDK
# --------------------------------------------------------------------------- #


class AnthropicBackend:
    name = "anthropic"

    def __init__(self, config: dict | None = None) -> None:
        import anthropic

        load_env()
        self.cfg = config or load_config()
        self.client = anthropic.Anthropic()

    def complete(
        self,
        role: str,
        system: str,
        user: str,
        schema: type[BaseModel],
        session_id: str | None = None,
    ) -> LLMResult:
        r = role_config(self.cfg, role, self.name)
        model = r["model"]
        effort = r.get("effort")
        kwargs: dict = dict(
            model=model,
            max_tokens=r.get("max_tokens", 16000),
            system=[{"type": "text", "text": system, "cache_control": {"type": "ephemeral"}}],
            messages=[{"role": "user", "content": user}],
            output_format=schema,
        )
        if model.startswith("claude-haiku"):
            resp = self.client.messages.parse(**kwargs)
        else:
            kwargs["thinking"] = {"type": "adaptive"}
            if effort:
                kwargs["output_config"] = {"effort": effort}
            with self.client.messages.stream(**kwargs) as stream:
                resp = stream.get_final_message()
        text = next((b.text for b in resp.content if b.type == "text"), "")
        u = resp.usage
        usage = {
            "prompt_tokens": u.input_tokens,
            "completion_tokens": u.output_tokens,
            "cache_read": getattr(u, "cache_read_input_tokens", None),
            "cache_write": getattr(u, "cache_creation_input_tokens", None),
        }
        return LLMResult(
            parsed=resp.parsed_output,
            text=text,
            model=resp.model,
            backend=self.name,
            usage=usage,
            id=resp.id,
        )


# --------------------------------------------------------------------------- #
# Claude Agent SDK (subscription)
# --------------------------------------------------------------------------- #


class ClaudeAgentBackend:
    name = "claude-agent"

    def __init__(self, config: dict | None = None) -> None:
        load_env()
        self.cfg = config or load_config()
        self._ensure_token()

    @staticmethod
    def _ensure_token() -> None:
        """CLAUDE_CODE_OAUTH_TOKEN from the environment, else the macOS keychain
        entry created by `security add-generic-password -s claude-code-oauth-token`."""
        if os.environ.get("CLAUDE_CODE_OAUTH_TOKEN"):
            return
        try:
            import keyring

            token = keyring.get_password("claude-code-oauth-token", os.environ.get("USER", ""))
        except Exception:  # noqa: BLE001
            token = None
        if token:
            os.environ["CLAUDE_CODE_OAUTH_TOKEN"] = token
        # otherwise the SDK falls back to whatever `claude` is logged in with

    def complete(
        self,
        role: str,
        system: str,
        user: str,
        schema: type[BaseModel],
        session_id: str | None = None,
    ) -> LLMResult:
        """One structured call. Subscription usage limits are handled per the
        `backends.claude-agent` block: wait until the window resets (default),
        fall back to another backend, or fail."""
        import asyncio

        self.last_failure = None

        from claude_agent_sdk import ClaudeAgentOptions, RateLimitEvent, ResultMessage, query

        r = role_config(self.cfg, role, self.name)
        model = r["model"]
        effort = r.get("effort")
        policy = backend_config(self.cfg, self.name)
        options = ClaudeAgentOptions(
            system_prompt=system,
            model=model,
            tools=[],
            allowed_tools=[],
            disallowed_tools=[
                "Bash",
                "Read",
                "Write",
                "Edit",
                "Glob",
                "Grep",
                "WebFetch",
                "WebSearch",
                "Task",
                "Agent",
            ],
            permission_mode="default",
            max_turns=3,
            setting_sources=[],
            cwd=REPO_DIR,
            output_format={"type": "json_schema", "schema": strict_schema(schema)},
            effort=effort,
        )

        async def run() -> tuple[ResultMessage, dict | None]:
            last: ResultMessage | None = None
            limit: dict | None = None
            with tempfile.TemporaryDirectory(prefix="wety-claude-") as directory:
                options.cwd = directory
                async for msg in query(prompt=user, options=options):
                    if isinstance(msg, RateLimitEvent):
                        info = msg.rate_limit_info
                        limit = {
                            "status": info.status,
                            "resets_at": info.resets_at,
                            "type": info.rate_limit_type,
                            "utilization": info.utilization,
                        }
                    elif isinstance(msg, ResultMessage):
                        last = msg
            if last is None:
                raise RuntimeError("claude-agent: no result message")
            return last, limit

        max_wait = int(policy.get("max_wait_seconds", 6 * 3600))
        for attempt in range(int(policy.get("max_attempts", 6))):
            try:
                res, limit = asyncio.run(run())
            except Exception as e:  # noqa: BLE001
                # the CLI exits non-zero on a hard limit, e.g.
                # "You've hit your session limit · resets 2:30am (America/New_York)"
                if _LIMIT_RE.search(str(e)):
                    res, limit = (
                        None,
                        {
                            "status": "rejected",
                            "resets_at": _parse_reset(str(e)),
                            "error": str(e)[:300],
                        },
                    )
                else:
                    raise
            hit = (limit or {}).get("status") == "rejected" or (
                res is not None
                and res.is_error
                and (res.api_error_status == 429 or _LIMIT_RE.search(res.result or ""))
            )
            if hit and res is not None and not (limit or {}).get("resets_at"):
                limit = {
                    **(limit or {}),
                    "status": "rejected",
                    "resets_at": _parse_reset(res.result or ""),
                }
            if not hit:
                break
            what = policy.get("on_usage_limit", "wait")
            self.last_limit = limit
            if (
                what == "fallback"
                and policy.get("fallback_backend")
                and policy["fallback_backend"] != self.name
            ):
                print(
                    f"  claude-agent: usage limit hit; falling back to {policy['fallback_backend']} for this call",
                    flush=True,
                )
                return get_backend(policy["fallback_backend"]).complete(
                    role, system, user, schema, session_id
                )
            if what != "wait":
                raise RuntimeError(f"claude-agent: usage limit hit ({limit})")
            resets = (limit or {}).get("resets_at")
            wait = (resets - time.time() + 30) if resets else 15 * 60 * (attempt + 1)
            wait = max(60, min(wait, max_wait))
            print(
                f"  claude-agent: usage limit hit ({(limit or {}).get('type')}); waiting {wait / 60:.0f} min (resets_at={resets}); progress so far is saved",
                flush=True,
            )
            time.sleep(wait)
        else:
            raise RuntimeError("claude-agent: usage limit still in force after retries")
        if res.is_error:
            raise RuntimeError(f"claude-agent error: {res.subtype} {res.errors or res.result}")
        data = res.structured_output
        try:
            if data is None:
                data = json.loads(res.result or "")
            parsed = schema.model_validate(data)
        except Exception:
            self.last_failure = {
                "text": json.dumps(data) if data is not None else res.result,
                "execution": {
                    "backend": self.name,
                    "model": model,
                    "usage": res.usage or {},
                    "cost_usd": res.total_cost_usd,
                    "message_id": res.session_id,
                    "billing": "claude_subscription",
                    "cost_scope": "SDK nominal estimate; not subscription cash expenditure",
                },
                "execution_trace": {
                    "num_turns": res.num_turns,
                    "model_usage": res.model_usage,
                    "rate_limit": limit,
                },
            }
            raise
        return LLMResult(
            parsed=parsed,
            text=json.dumps(data),
            model=model,
            backend=self.name,
            usage=res.usage or {},
            cost_usd=res.total_cost_usd,
            id=res.session_id,
            raw={
                "num_turns": res.num_turns,
                "model_usage": res.model_usage,
                "rate_limit": limit,
                "model_source": "requested; SDK model_usage retained separately",
                "effort": effort,
                "billing": "claude_subscription",
                "cost_scope": "SDK estimate; not per-call subscription cash expenditure",
            },
        )


_LIMIT_RE = re.compile(
    r"hit your (?:session|usage|weekly|daily)? ?limit|usage limit|rate limit|\b429\b|limit reached|out of (?:usage|credits)",
    re.I,
)


def _parse_reset(msg: str) -> int | None:
    """'resets 2:30am (America/New_York)' / 'resets at 14:00 (UTC)' -> unix timestamp of the next such time."""
    import datetime as dt
    from zoneinfo import ZoneInfo

    m = re.search(
        r"resets?(?: at)?\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\s*(?:\(([A-Za-z_]+/[A-Za-z_]+|UTC)\))?",
        msg,
        re.I,
    )
    if not m:
        return None
    hour, minute = int(m.group(1)), int(m.group(2) or 0)
    if m.group(3):
        ap = m.group(3).lower()
        hour = hour % 12 + (12 if ap == "pm" else 0)
    tz = ZoneInfo(m.group(4)) if m.group(4) else dt.datetime.now().astimezone().tzinfo
    now_ = dt.datetime.now(tz)
    target = now_.replace(hour=hour, minute=minute, second=0, microsecond=0)
    if target <= now_:
        target += dt.timedelta(days=1)
    return int(target.timestamp())


# --------------------------------------------------------------------------- #
# Codex CLI (ChatGPT subscription)
# --------------------------------------------------------------------------- #


def codex_executable(name="bundled"):
    """The Codex CLI. "bundled" (the default) is the version-pinned binary of the
    openai-codex-cli-bin dependency, so runs never depend on whichever `codex` is on
    PATH; any other value is a command name or path. Login is shared (CODEX_HOME)."""
    if name == "bundled":
        try:
            from codex_cli_bin import bundled_codex_path

            return str(bundled_codex_path())
        except (ImportError, FileNotFoundError):
            raise RuntimeError(
                "codex: the bundled CLI (openai-codex-cli-bin) is missing; run `uv sync`"
            ) from None
    found = shutil.which(name)
    if not found:
        raise RuntimeError(
            f"codex: CLI {name!r} not found; use the bundled CLI or install @openai/codex, then run `codex login` with ChatGPT"
        )
    return found


def claude_executable():
    """The Claude Code CLI the Agent SDK bundles (the SDK runs it too), else PATH."""
    import claude_agent_sdk

    bundled = Path(claude_agent_sdk.__file__).parent / "_bundled" / "claude"
    return str(bundled) if bundled.exists() else shutil.which("claude") or "claude"


class CodexBackend:
    """Stateless structured calls through the supported `codex exec` interface.

    Authentication and token refresh belong to Codex. Never read/copy its OAuth
    tokens or fall back to a paid API. A session_id labels harness work only;
    each completion starts a fresh, ephemeral Codex thread.
    """

    name = "codex"

    def __init__(self, config: dict | None = None) -> None:
        load_env()
        self.cfg = config or load_config()
        self.executable = codex_executable(
            backend_config(self.cfg, self.name).get("executable", "bundled")
        )

    def complete(
        self,
        role: str,
        system: str,
        user: str,
        schema: type[BaseModel],
        session_id: str | None = None,
    ) -> LLMResult:
        self.last_failure = None
        r = role_config(self.cfg, role, self.name)
        model = r.get("model")
        if not model:
            raise ValueError(
                f"codex: configure roles.{role}.backends.codex.model in the dating LLM config (legacy roles.{role}.codex.model)"
            )
        policy = backend_config(self.cfg, self.name)
        timeout = float(policy.get("timeout_seconds", 600))
        if timeout <= 0:
            raise ValueError("codex: timeout_seconds must be positive")
        # Keep CODEX_HOME and the ordinary login cache, but prevent API-key and
        # provider overrides inherited from the harness environment.
        env = {
            k: v
            for k, v in os.environ.items()
            if k
            not in {
                "OPENAI_API_KEY",
                "CODEX_API_KEY",
                "OPENAI_BASE_URL",
                "OPENAI_FEDERATION_RULE_ID",
            }
        }
        auth_store = policy.get("auth_credentials_store", "file")
        if auth_store not in {"file", "keyring", "auto"}:
            raise ValueError("codex: auth_credentials_store must be file, keyring, or auto")
        auth_config = ["-c", f"cli_auth_credentials_store={json.dumps(auth_store)}"]
        try:
            status = subprocess.run(
                [self.executable, "login", "status", *auth_config],
                env=env,
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=30,
            )
        except subprocess.TimeoutExpired:
            raise RuntimeError(
                "codex: login status timed out; check `codex login status`"
            ) from None
        # Do not print login output: API-key status can contain credential hints.
        if (
            status.returncode
            or "logged in using chatgpt" not in (status.stdout + status.stderr).lower()
        ):
            raise RuntimeError(
                "codex: ChatGPT subscription login required; run `codex login` (or `codex login --device-auth`), then `codex login status`. API-key login is not accepted."
            )

        with tempfile.TemporaryDirectory(prefix="wety-codex-") as directory:
            root = Path(directory)
            schema_path = root / "schema.json"
            instructions_path = root / "instructions.txt"
            output_path = root / "result.json"
            schema_path.write_text(json.dumps(strict_schema(schema)), encoding="utf-8")
            instructions_path.write_text(system, encoding="utf-8")
            command = [
                self.executable,
                "exec",
                "--json",
                "--ephemeral",
                "--ignore-user-config",
                "--skip-git-repo-check",
                "--sandbox",
                "read-only",
                "--color",
                "never",
                "--cd",
                directory,
                "--model",
                model,
                "--output-schema",
                str(schema_path),
                "--output-last-message",
                str(output_path),
                *auth_config,
            ]
            overrides = {
                "forced_login_method": "chatgpt",
                "model_provider": "openai",
                "approval_policy": "never",
                "web_search": "disabled",
                "project_doc_max_bytes": 0,
                "model_instructions_file": str(instructions_path),
                "features.shell_tool": False,
                "features.plugins": False,
                "features.apps": False,
                "features.multi_agent": False,
                "features.browser_use": False,
                "features.computer_use": False,
                "features.image_generation": False,
                "features.hooks": False,
                "features.memories": False,
                "tools.view_image": False,
            }
            if r.get("effort"):
                overrides["model_reasoning_effort"] = r["effort"]
            for key, value in overrides.items():
                command.extend(["-c", f"{key}={json.dumps(value)}"])
            # stdin handles large source texts without shell interpolation or
            # exposing them in the process's command-line arguments.
            try:
                proc = subprocess.run(
                    command + ["-"],
                    input=user,
                    cwd=directory,
                    env=env,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    timeout=timeout,
                )
            except subprocess.TimeoutExpired:
                raise RuntimeError(
                    f"codex: completion timed out after {timeout:g}s; no API fallback was attempted"
                ) from None
            events = []
            for line in proc.stdout.splitlines():
                if line.strip():
                    try:
                        event = json.loads(line)
                    except json.JSONDecodeError:
                        raise RuntimeError(
                            "codex: invalid JSON event stream; check/update the Codex CLI"
                        ) from None
                    if not isinstance(event, dict):
                        raise RuntimeError("codex: invalid JSON event object")
                    events.append(event)
            failures = [e for e in events if e.get("type") == "turn.failed"]
            completed = [e for e in events if e.get("type") == "turn.completed"]
            if proc.returncode or failures or not completed:
                errors = failures or [e for e in events if e.get("type") == "error"]
                detail = json.dumps(errors[-1]) if errors else proc.stderr.strip()
                raise RuntimeError(
                    f"codex: completion failed (exit {proc.returncode}); no API fallback was attempted: {detail[-1000:]}"
                )
            if not output_path.is_file() or not (
                text := output_path.read_text(encoding="utf-8").strip()
            ):
                raise RuntimeError("codex: completed without a structured result")
            usage = completed[-1].get("usage") or {}
            thread_id = next(
                (e.get("thread_id") for e in events if e.get("type") == "thread.started"), None
            )
            try:
                parsed = schema.model_validate_json(text)
            except Exception:
                self.last_failure = {
                    "text": text,
                    "execution": {
                        "backend": self.name,
                        "model": model,
                        "usage": usage,
                        "message_id": thread_id,
                        "cost_usd": None,
                        "billing": "chatgpt_subscription",
                        "model_source": "requested; resolved version unreported",
                    },
                    "execution_trace": {"events": events},
                }
                raise
            return LLMResult(
                parsed=parsed,
                text=text,
                model=model,
                backend=self.name,
                usage=usage,
                id=thread_id,
                cost_usd=None,
                raw={
                    "billing": "chatgpt_subscription",
                    "effort": r.get("effort"),
                    "model_source": "requested; codex exec does not report a resolved model",
                },
            )


BACKENDS = {
    "openrouter": OpenRouterBackend,
    "anthropic": AnthropicBackend,
    "claude-agent": ClaudeAgentBackend,
    "codex": CodexBackend,
}
_instances: dict[tuple[str, str], Any] = {}


def get_backend(name: str | None = None, role: str | None = None):
    """DATING_LLM_BACKEND overrides everything; else the role's `backend` in the config; else openrouter."""
    cfg = load_config()
    if name is None:
        name = os.environ.get("DATING_LLM_BACKEND")
    if name is None and role:
        name = cfg["roles"].get(role, {}).get("backend")
    name = name or cfg.get("default_backend", "openrouter")
    if name not in BACKENDS:
        raise SystemExit(f"unknown backend {name!r}; choose from {', '.join(BACKENDS)}")
    key = (name, digest(cfg))
    if key not in _instances:
        _instances[key] = BACKENDS[name](config=copy.deepcopy(cfg))
    return _instances[key]
