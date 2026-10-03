import copy
import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from pydantic import BaseModel

from harness import llm_backends, model_requests
from harness.io_utils import digest
from harness.model_config import backend_config, role_config, migrate_config, validate_config

ROOT = Path(llm_backends.REPO_DIR)


class Answer(BaseModel):
    answer: int


class ModelConfigTests(unittest.TestCase):
    def setUp(self):
        self.old = json.loads(
            (ROOT / "packages/dating/docs/archive/llm-config-2026-09-20.json").read_text()
        )
        self.cfg = migrate_config(self.old)

    def test_migration_preserves_all_backend_models_efforts_and_selection(self):
        for role, old in self.old["roles"].items():
            for backend in ("openrouter", "anthropic", "claude-agent", "codex"):
                before = role_config(self.old, role, backend)
                after = role_config(self.cfg, role, backend)
                for field in (
                    "model",
                    "effort",
                    "reasoning",
                    "max_tokens",
                    "fallbacks",
                    "batch_model",
                    "provider",
                ):
                    self.assertEqual(before.get(field), after.get(field), (role, backend, field))
            self.assertEqual(self.cfg["roles"][role]["backend"], old.get("backend", "openrouter"))
        self.assertEqual(backend_config(self.cfg, "codex"), self.old["codex"])
        self.assertEqual(backend_config(self.cfg, "claude-agent"), self.old["claude_agent"])
        self.assertNotIn("rationale", self.cfg)

    def test_active_config_is_dating_specific_and_overridable(self):
        with patch.dict(os.environ, {}, clear=True):
            active = llm_backends.load_config()
        self.assertEqual(active["schema_version"], 2)
        self.assertFalse((ROOT / "openrouter.config.json").exists())
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "trial.json"
            self.cfg["default_backend"] = "codex"
            p.write_text(json.dumps(self.cfg))
            with patch.dict(os.environ, {"DATING_LLM_CONFIG": str(p)}):
                self.assertEqual(llm_backends.load_config()["default_backend"], "codex")

    def test_invalid_selection_and_legacy_disk_schema_fail(self):
        with self.assertRaisesRegex(ValueError, "schema_version 2"):
            validate_config(self.old)
        self.cfg["roles"]["extract"]["backend"] = "not-a-backend"
        with self.assertRaisesRegex(ValueError, "no model settings"):
            validate_config(self.cfg)

    def test_backend_cache_does_not_reuse_another_experiment_config(self):
        class Backend:
            def __init__(self, config):
                self.cfg = config

        with (
            patch.dict(llm_backends.BACKENDS, {"codex": Backend}),
            patch.dict(llm_backends._instances, {}, clear=True),
            patch.object(llm_backends, "load_config", side_effect=lambda: self.cfg),
        ):
            first = llm_backends.get_backend("codex")
            self.assertIs(first, llm_backends.get_backend("codex"))
            self.cfg["roles"]["extract"]["backends"]["codex"]["effort"] = "high"
            second = llm_backends.get_backend("codex")
            self.assertIsNot(first, second)
            self.assertEqual(role_config(first.cfg, "extract", "codex")["effort"], "medium")
            self.assertEqual(role_config(second.cfg, "extract", "codex")["effort"], "high")

    def test_anthropic_and_agent_roles_are_independent(self):
        self.cfg["roles"]["extract"]["backends"]["anthropic"]["model"] = "api-only-model"
        self.assertNotEqual(
            role_config(self.cfg, "extract", "anthropic")["model"],
            role_config(self.cfg, "extract", "claude-agent")["model"],
        )

    def test_openrouter_wire_request_is_unchanged_by_layout_migration(self):
        with (
            patch.object(llm_backends, "load_env"),
            patch.dict(os.environ, {"OPENROUTER_API_KEY": "test-only"}),
        ):
            before = llm_backends.OpenRouterBackend(self.old)
            after = llm_backends.OpenRouterBackend(self.cfg)
            for role in self.cfg["roles"]:
                for batch in (True, False):
                    self.assertEqual(
                        before.build_request(role, "system", "user", Answer, batch=batch),
                        after.build_request(role, "system", "user", Answer, batch=batch),
                    )

    def test_snapshot_records_only_selected_role_and_config_identity(self):
        backend = SimpleNamespace(name="codex", cfg=self.cfg)
        with patch.object(model_requests, "_save", side_effect=lambda kind, p: p):
            request = model_requests.freeze_request(
                "extract", "f", "i", "system", "user", Answer, backend
            )
        execution = request["requested_execution"]
        self.assertEqual(execution["role_settings"], role_config(self.cfg, "extract", "codex"))
        self.assertEqual(execution["config_sha256"], digest(self.cfg))
        self.assertEqual(execution["runtime_policy"]["timeout_seconds"], 600)
        self.assertNotIn("provider", execution["defaults"])

    def test_codex_requested_identity_is_not_mislabeled_as_resolved_model(self):
        r = llm_backends.LLMResult(
            parsed=Answer(answer=1),
            text="{}",
            model="requested",
            backend="codex",
            raw={
                "model_source": "requested only",
                "effort": "high",
                "billing": "chatgpt_subscription",
                "secret": "not logged",
            },
        )
        self.assertEqual(r.log_fields()["model_source"], "requested only")
        self.assertNotIn("secret", r.log_fields())

    def test_claude_agent_uses_temporary_working_directory(self):
        class Result:
            pass

        result = Result()
        for key, value in {
            "is_error": False,
            "structured_output": {"answer": 42},
            "usage": {},
            "total_cost_usd": 0,
            "session_id": "fixture",
            "num_turns": 1,
            "model_usage": {},
        }.items():
            setattr(result, key, value)
        visited = []

        async def query(prompt, options):
            visited.append(options.cwd)
            self.assertTrue(Path(options.cwd).is_dir())
            self.assertNotEqual(options.cwd, str(ROOT / "dating/cache"))
            self.assertEqual(options.tools, [])
            self.assertEqual(options.setting_sources, [])
            yield result

        sdk = SimpleNamespace(
            ClaudeAgentOptions=lambda **k: SimpleNamespace(**k),
            RateLimitEvent=type("Limit", (), {}),
            ResultMessage=Result,
            query=query,
        )
        with (
            patch.dict(sys.modules, {"claude_agent_sdk": sdk}),
            patch.object(llm_backends, "load_env"),
            patch.object(llm_backends.ClaudeAgentBackend, "_ensure_token"),
        ):
            backend = llm_backends.ClaudeAgentBackend(self.cfg)
            self.assertEqual(
                backend.complete("extract", "system", "user", Answer).parsed.answer, 42
            )
        self.assertEqual(len(visited), 1)
        self.assertFalse(Path(visited[0]).exists())

    def test_openrouter_repair_retains_both_attempts_and_counts_both_costs(self):
        responses = [
            {
                "model": "first-model",
                "provider": "first-provider",
                "choices": [{"message": {"content": '{"answer":"invalid"}'}}],
                "usage": {
                    "prompt_tokens": 4,
                    "completion_tokens": 2,
                    "cost": 0.1,
                    "prompt_tokens_details": {"cached_tokens": 1},
                },
            },
            {
                "model": "second-model",
                "provider": "second-provider",
                "choices": [{"message": {"content": '{"answer":42}'}}],
                "usage": {
                    "prompt_tokens": 6,
                    "completion_tokens": 3,
                    "cost": 0.2,
                    "prompt_tokens_details": {"cached_tokens": 2},
                },
            },
        ]
        with (
            patch.object(llm_backends, "load_env"),
            patch.dict(os.environ, {"OPENROUTER_API_KEY": "test-only"}),
        ):
            backend = llm_backends.OpenRouterBackend(self.cfg)
        with patch.object(backend, "_post", side_effect=responses):
            result = backend.complete("extract", "system", "user", Answer)
        self.assertEqual(result.parsed.answer, 42)
        self.assertEqual(result.usage["prompt_tokens"], 10)
        self.assertEqual(result.usage["prompt_tokens_details"]["cached_tokens"], 3)
        self.assertAlmostEqual(result.cost_usd, 0.3)
        self.assertEqual(result.log_fields()["attempt_count"], 2)
        self.assertEqual(result.log_fields()["provider"], "second-provider")
        self.assertEqual(result.raw["attempts"][0]["response"], responses[0])
        self.assertEqual(len(result.raw["attempts"][0]["request"]["messages"]), 2)
        self.assertEqual(len(result.raw["attempts"][1]["request"]["messages"]), 4)
        with (
            patch.object(model_requests, "load_request"),
            patch.object(model_requests, "_save", side_effect=lambda kind, p: p),
        ):
            retained = model_requests.record_response("reference", result)
        self.assertEqual(len(retained["execution_trace"]["attempts"]), 2)

    def test_missing_response_cost_remains_unknown(self):
        response = {
            "choices": [{"message": {"content": '{"answer":42}'}}],
            "usage": {"prompt_tokens": 4},
        }
        with (
            patch.object(llm_backends, "load_env"),
            patch.dict(os.environ, {"OPENROUTER_API_KEY": "test-only"}),
        ):
            backend = llm_backends.OpenRouterBackend(self.cfg)
        with patch.object(backend, "_post", return_value=response):
            result = backend.complete("screen", "system", "user", Answer)
        self.assertIsNone(result.cost_usd)
        self.assertIn("requested", result.log_fields()["model_source"])

    def test_failed_repair_retains_outputs_and_reported_cost(self):
        response = {
            "model": "test",
            "choices": [{"message": {"content": '{"answer":"invalid"}'}}],
            "usage": {"cost": 0.1},
        }
        with (
            patch.object(llm_backends, "load_env"),
            patch.dict(os.environ, {"OPENROUTER_API_KEY": "not-to-be-retained"}),
        ):
            backend = llm_backends.OpenRouterBackend(self.cfg)
        with patch.object(backend, "_post", return_value=response), self.assertRaises(ValueError):
            backend.complete("extract", "system", "user", Answer)
        self.assertEqual(len(backend.last_failure["execution_trace"]["attempts"]), 2)
        self.assertEqual(backend.last_failure["execution"]["cost_usd"], 0.2)

    def test_compact_transport_can_disable_whole_response_repair(self):
        self.cfg["roles"]["extract"]["backends"]["openrouter"]["schema_repair"] = False
        response = {
            "choices": [{"message": {"content": '{"answer":"invalid"}'}}],
            "usage": {"cost": 0.1},
        }
        with (
            patch.object(llm_backends, "load_env"),
            patch.dict(os.environ, {"OPENROUTER_API_KEY": "test-only"}),
        ):
            backend = llm_backends.OpenRouterBackend(self.cfg)
        with (
            patch.object(backend, "_post", return_value=response) as post,
            self.assertRaises(ValueError),
        ):
            backend.complete("extract", "system", "user", Answer)
        self.assertEqual(post.call_count, 1)
        self.assertEqual(backend.last_failure["text"], '{"answer":"invalid"}')
        self.assertNotIn("not-to-be-retained", json.dumps(backend.last_failure))

    def test_failed_claude_validation_retains_usage_and_unparsed_output(self):
        class Result:
            pass

        result = Result()
        for key, value in {
            "is_error": False,
            "structured_output": {"answer": "invalid"},
            "usage": {"output_tokens": 12},
            "total_cost_usd": 0.25,
            "session_id": "fixture",
            "num_turns": 1,
            "model_usage": {},
        }.items():
            setattr(result, key, value)

        async def query(**kwargs):
            yield result

        sdk = SimpleNamespace(
            ClaudeAgentOptions=lambda **k: SimpleNamespace(**k),
            RateLimitEvent=type("Limit", (), {}),
            ResultMessage=Result,
            query=query,
        )
        with (
            patch.dict(sys.modules, {"claude_agent_sdk": sdk}),
            patch.object(llm_backends, "load_env"),
            patch.object(llm_backends.ClaudeAgentBackend, "_ensure_token"),
        ):
            backend = llm_backends.ClaudeAgentBackend(self.cfg)
            with self.assertRaises(ValueError):
                backend.complete("extract", "system", "user", Answer)
        self.assertEqual(json.loads(backend.last_failure["text"]), {"answer": "invalid"})
        self.assertEqual(backend.last_failure["execution"]["cost_usd"], 0.25)
        with (
            patch.object(model_requests, "load_request"),
            patch.object(model_requests, "_save", side_effect=lambda k, p: p),
        ):
            saved = model_requests.record_failure(
                "request", ValueError("invalid"), backend.last_failure
            )
        self.assertEqual(saved["execution"]["usage"]["output_tokens"], 12)


if __name__ == "__main__":
    unittest.main()
