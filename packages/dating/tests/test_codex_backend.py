"""Offline coverage of subscription auth, subprocess transport and result validation."""
import copy
import json
import os
from pathlib import Path
import subprocess
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from pydantic import BaseModel, ValidationError

from harness import cli, llm_backends, model_requests
from harness.llm_backends import CodexBackend, strict_schema


class Answer(BaseModel):
    answer: int
    note: str | None = None


class CodexBackendTests(unittest.TestCase):
    def setUp(self):
        self.config = {"roles": {"screen": {"codex": {"model": "test-model", "effort": "low"}}},
                       "codex": {"timeout_seconds": 45}}
        for target, kwargs in (
            ("harness.llm_backends.load_env", {}),
            ("harness.llm_backends.shutil.which", {"return_value": "/bin/codex"}),
        ):
            p = patch(target, **kwargs)
            p.start()
            self.addCleanup(p.stop)
        self.backend = CodexBackend(copy.deepcopy(self.config))
        self.events = [
            {"type": "thread.started", "thread_id": "test-thread"},
            {"type": "turn.started"},
            {"type": "item.completed", "item": {"type": "agent_message", "text": "commentary"}},
            {"type": "turn.completed", "usage": {"input_tokens": 30, "cached_input_tokens": 20, "output_tokens": 10}},
        ]
        self.output = '{"answer": 42, "note": null}'
        self.returncode = 0
        self.stderr = ""

    def fake_run(self, command, **kwargs):
        if command[1:3] == ["login", "status"]:
            return SimpleNamespace(returncode=0, stdout="", stderr="Logged in using ChatGPT\n")
        self.command, self.kwargs = command, kwargs
        self.workdir = Path(kwargs["cwd"])
        self.sent_schema = json.loads((self.workdir / "schema.json").read_text())
        self.sent_system = (self.workdir / "instructions.txt").read_text()
        if self.output is not None:
            (self.workdir / "result.json").write_text(self.output)
        return SimpleNamespace(returncode=self.returncode, stdout="\n".join(map(json.dumps, self.events)), stderr=self.stderr)

    def complete(self):
        return self.backend.complete("screen", 'system\n"quoted"', "user\n$(do not execute)", Answer, "family")

    def test_structured_transport_auth_isolation_and_usage(self):
        secret_env = {"OPENAI_API_KEY": "api-secret", "CODEX_API_KEY": "codex-secret",
                      "OPENAI_BASE_URL": "https://other.invalid", "OPENAI_FEDERATION_RULE_ID": "rule",
                      "CODEX_HOME": "/custom/codex"}
        with patch.dict(os.environ, secret_env), patch.object(llm_backends.subprocess, "run", side_effect=self.fake_run) as run:
            result = self.complete()
        self.assertEqual(run.call_count, 2)
        self.assertEqual(result.parsed.answer, 42)
        self.assertEqual(result.backend, "codex")
        self.assertEqual(result.model, "test-model")
        self.assertEqual(result.id, "test-thread")
        self.assertIsNone(result.cost_usd)
        self.assertEqual(result.usage["cached_input_tokens"], 20)
        self.assertEqual(self.sent_schema, strict_schema(Answer))
        self.assertEqual(self.sent_system, 'system\n"quoted"')
        self.assertEqual(self.kwargs["input"], "user\n$(do not execute)")
        self.assertEqual(self.kwargs["timeout"], 45)
        self.assertNotIn(self.kwargs["input"], self.command)
        self.assertEqual(self.kwargs["env"]["CODEX_HOME"], "/custom/codex")
        for key in secret_env.keys() - {"CODEX_HOME"}:
            self.assertNotIn(key, self.kwargs["env"])
        for flag in ("--ephemeral", "--ignore-user-config", "--output-schema", "read-only",
                     'forced_login_method="chatgpt"', 'model_provider="openai"',
                     'web_search="disabled"', 'features.shell_tool=false',
                     'features.plugins=false', 'approval_policy="never"',
                     'project_doc_max_bytes=0', 'model_reasoning_effort="low"'):
            self.assertIn(flag, self.command)
        self.assertFalse(self.workdir.exists())

    def test_api_login_and_missing_login_never_dispatch(self):
        for status in (SimpleNamespace(returncode=0, stdout="Logged in using an API key: secret", stderr=""),
                       SimpleNamespace(returncode=1, stdout="", stderr="Not logged in")):
            with self.subTest(status=status), patch.object(llm_backends.subprocess, "run", return_value=status) as run:
                with self.assertRaisesRegex(RuntimeError, "ChatGPT subscription login required") as caught:
                    self.complete()
                self.assertNotIn("secret", str(caught.exception))
                self.assertEqual(run.call_count, 1)

    def test_failures_never_fall_back_or_accept_partial_output(self):
        cases = [
            (1, [{"type": "error", "message": "Usage limit reached"}]),
            (0, [{"type": "turn.failed", "error": {"message": "rate limit"}}]),
            (0, [{"type": "thread.started", "thread_id": "partial"}]),
        ]
        for code, events in cases:
            self.returncode, self.events = code, events
            with self.subTest(code=code, events=events), patch.object(llm_backends.subprocess, "run", side_effect=self.fake_run), \
                    patch.object(llm_backends, "get_backend") as fallback:
                with self.assertRaisesRegex(RuntimeError, "no API fallback"):
                    self.complete()
                fallback.assert_not_called()
                self.assertFalse(self.workdir.exists())

    def test_retry_notice_followed_by_success_is_accepted(self):
        self.events.insert(1, {"type": "error", "message": "Reconnecting..."})
        with patch.object(llm_backends.subprocess, "run", side_effect=self.fake_run):
            self.assertEqual(self.complete().parsed.answer, 42)

    def test_missing_or_empty_final_output(self):
        for output in (None, "", "   "):
            self.output = output
            with self.subTest(output=output), patch.object(llm_backends.subprocess, "run", side_effect=self.fake_run):
                with self.assertRaisesRegex(RuntimeError, "without a structured result"):
                    self.complete()

    def test_invalid_schema_output_rejected(self):
        self.output = '{"answer": "not an integer"}'
        with patch.object(llm_backends.subprocess, "run", side_effect=self.fake_run):
            with self.assertRaises(ValidationError):
                self.complete()
        self.assertFalse(self.workdir.exists())
        self.assertEqual(self.backend.last_failure['text'],self.output)
        self.assertIsNone(self.backend.last_failure['execution']['cost_usd'])

    def test_invalid_event_stream(self):
        def malformed(command, **kwargs):
            result = self.fake_run(command, **kwargs)
            if command[1] == "exec":
                result.stdout = "not json"
            return result
        with patch.object(llm_backends.subprocess, "run", side_effect=malformed):
            with self.assertRaisesRegex(RuntimeError, "invalid JSON event"):
                self.complete()

    def test_completion_timeout_is_actionable_and_cleans_files(self):
        def timeout(command, **kwargs):
            if command[1] == "exec":
                self.workdir = Path(kwargs["cwd"])
                raise subprocess.TimeoutExpired(command, 45)
            return self.fake_run(command, **kwargs)
        with patch.object(llm_backends.subprocess, "run", side_effect=timeout):
            with self.assertRaisesRegex(RuntimeError, "timed out after 45s"):
                self.complete()
        self.assertFalse(self.workdir.exists())

    def test_missing_binary_and_role_settings(self):
        with patch.object(llm_backends.shutil, "which", return_value=None), self.assertRaisesRegex(RuntimeError, "CLI not found"):
            CodexBackend(self.config)
        self.backend.cfg["roles"]["screen"].pop("codex")
        with self.assertRaisesRegex(ValueError, "roles.screen.codex.model"):
            self.complete()

    def test_factory_supports_explicit_environment_and_per_role_selection(self):
        with patch.dict(llm_backends._instances, {}, clear=True), \
                patch.object(llm_backends, "load_config", return_value=self.config), \
                patch.dict(os.environ, {}, clear=True):
            self.config["roles"]["screen"]["backend"] = "codex"
            self.assertIsInstance(llm_backends.get_backend(role="screen"), CodexBackend)
            with patch.dict(os.environ, {"DATING_LLM_BACKEND": "codex"}):
                self.assertIs(llm_backends.get_backend(role="extract"), llm_backends.get_backend("codex"))

    def test_parity_check_includes_codex(self):
        with patch("harness.llm.llm_check", return_value=[]) as check, patch("builtins.print"):
            cli.cmd_llm_check(SimpleNamespace(backends=[]))
        self.assertIn("codex", check.call_args.args[0])

    def test_snapshot_retains_codex_model_and_effort(self):
        with patch.object(model_requests, "_save", side_effect=lambda kind, payload: payload):
            request = model_requests.freeze_request("screen", "family", "item", "system", "user", Answer, self.backend)
        self.assertEqual(request["requested_execution"]["role_settings"],
                         {"model": "test-model", "effort": "low"})


if __name__ == "__main__":
    unittest.main()
