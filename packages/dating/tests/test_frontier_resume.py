import asyncio
import json
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

from claude_agent_sdk import (
    SystemMessage,
    RateLimitEvent,
    RateLimitInfo,
    ResultMessage,
    AssistantMessage,
)
from harness import frontier_research as frontier
from harness.frontier_tools import sha


class ResumeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.brief = {
            "profile": {"backend": "claude-agent", "model": "claude-opus-5", "effort": "high"},
            "transport": {},
            "limits": {"model_turns": 10},
            "research_seconds": 100,
            "delivery_seconds": 50,
            "deadline_unix": 0,
            "research_deadline_unix": 0,
            "billing": "claude_subscription",
        }
        for name, content in [
            ("prompt.txt", "original research"),
            ("instructions.txt", "contract"),
            ("brief.json", json.dumps(self.brief)),
        ]:
            (self.root / name).write_text(content)
        prepared = {
            name + "_sha256": sha(self.root / (name + (".json" if name == "brief" else ".txt")))
            for name in ("brief", "prompt", "instructions")
        }
        prepared["implementation"] = {
            name: sha(Path(frontier.__file__).with_name(name))
            for name in ("frontier_research.py", "frontier_tools.py")
        }
        (self.root / "prepared.json").write_text(json.dumps(prepared))

    def test_warning_continues_rejection_stops_and_exact_session_resumes(self):
        calls = []
        after_rejection = []

        async def query(**kwargs):
            calls.append(kwargs)
            yield SystemMessage(subtype="init", data={"session_id": "saved-session"})
            if len(calls) == 1:
                yield RateLimitEvent(
                    RateLimitInfo(
                        status="allowed_warning", utilization=0.83, rate_limit_type="five_hour"
                    ),
                    uuid="w",
                    session_id="saved-session",
                )
                yield RateLimitEvent(
                    RateLimitInfo(status="rejected", resets_at=123, rate_limit_type="five_hour"),
                    uuid="r",
                    session_id="saved-session",
                )
                after_rejection.append(True)
            else:
                yield ResultMessage(
                    subtype="success",
                    duration_ms=1,
                    duration_api_ms=1,
                    is_error=False,
                    num_turns=2,
                    session_id="saved-session",
                    usage={"output_tokens": 9},
                    result="done",
                )

        with (
            patch.dict("os.environ", {"CLAUDE_CODE_OAUTH_TOKEN": "fake"}),
            patch("claude_agent_sdk.query", query),
        ):
            asyncio.run(frontier.claude_run(self.root, self.brief))
            state = frontier.read_json(self.root / "claude-state.json")
            self.assertEqual(state["status"], "paused_rate_limit")
            self.assertEqual(state["resume_after"], 123)
            self.assertEqual(state["session_id"], "saved-session")
            self.assertFalse(after_rejection)
            (self.root / "dispatch.json").write_text(
                json.dumps({"attempt": 2, "resume_session_id": "saved-session"})
            )
            asyncio.run(frontier.claude_run(self.root, self.brief))
        self.assertEqual(calls[1]["options"].resume, "saved-session")
        self.assertEqual(calls[1]["options"].max_turns, 9)
        self.assertNotIn("original research", calls[1]["prompt"])
        self.assertEqual(len(frontier.read_json(self.root / "usage.json")["attempts"]), 2)
        self.assertIsNone(frontier.read_json(self.root / "usage.json")["attempts"][0]["usage"])
        self.assertEqual(frontier.read_json(self.root / "claude-state.json")["turns_used"], 3)
        self.assertEqual(len((self.root / "events.jsonl").read_text().splitlines()), 5)

    def test_error_result_retains_session_even_without_init(self):
        async def query(**kwargs):
            yield ResultMessage(
                subtype="error_during_execution",
                duration_ms=1,
                duration_api_ms=1,
                is_error=True,
                num_turns=1,
                session_id="saved-on-error",
                api_error_status=429,
                result="Usage limit reached",
            )

        with (
            patch.dict("os.environ", {"CLAUDE_CODE_OAUTH_TOKEN": "fake"}),
            patch("claude_agent_sdk.query", query),
        ):
            asyncio.run(frontier.claude_run(self.root, self.brief))
        state = frontier.read_json(self.root / "claude-state.json")
        self.assertEqual(state["status"], "paused_rate_limit")
        self.assertEqual(state["session_id"], "saved-on-error")
        self.assertIsNone(state["resume_after"])

    def test_assistant_frames_count_once_and_ids_survive_resume(self):
        calls = []

        async def query(**kwargs):
            calls.append(kwargs)
            yield SystemMessage(subtype="init", data={"session_id": "same"})
            for _ in range(3):
                yield AssistantMessage(content=[], model="test", message_id="first")
            if len(calls) == 1:
                yield AssistantMessage(
                    content=[], model="test"
                )  # Missing ID still costs a reservation.
                yield RateLimitEvent(
                    RateLimitInfo(status="rejected", resets_at=123), uuid="r", session_id="same"
                )
            else:
                yield AssistantMessage(content=[], model="test", message_id="second")
                yield ResultMessage(
                    subtype="success",
                    duration_ms=1,
                    duration_api_ms=1,
                    is_error=False,
                    num_turns=1,
                    session_id="same",
                    result="done",
                )

        with (
            patch.dict("os.environ", {"CLAUDE_CODE_OAUTH_TOKEN": "fake"}),
            patch("claude_agent_sdk.query", query),
        ):
            asyncio.run(frontier.claude_run(self.root, self.brief))
            self.assertEqual(frontier.read_json(self.root / "claude-state.json")["turns_used"], 2)
            (self.root / "dispatch.json").write_text(
                json.dumps({"attempt": 2, "resume_session_id": "same"})
            )
            asyncio.run(frontier.claude_run(self.root, self.brief))
        state = frontier.read_json(self.root / "claude-state.json")
        self.assertEqual(calls[1]["options"].max_turns, 8)
        self.assertEqual(state["turns_used"], 3)
        self.assertEqual(state["assistant_message_ids"], ["first", "second"])

    def test_resume_preserves_counters_outputs_and_active_budget(self):
        (self.root / "tools-state.json").write_text(
            json.dumps({"searches": 2, "acquisitions": 3, "sources": {}})
        )
        (self.root / "report.md").write_text("existing partial findings")
        original = (self.root / "tools-state.json").read_bytes()
        calls = []
        with patch("harness.frontier_research.time.time", return_value=100) as clock:

            def child(command, timeout):
                calls.append(timeout)
                dispatch = frontier.read_json(self.root / "dispatch.json")
                clock.return_value += 5
                (self.root / "claude-state.json").write_text(
                    json.dumps(
                        {
                            "status": "paused_rate_limit",
                            "session_id": "same",
                            "attempt": dispatch["attempt"],
                            "turns_used": len(calls),
                            "resume_after": 110,
                        }
                    )
                )
                return subprocess.CompletedProcess(command, 0, "", "")

            with patch("harness.frontier_research.run_process_group", child):
                first = frontier.run(self.root)
                self.assertEqual(first["active_seconds_used"], 5)
                with self.assertRaisesRegex(ValueError, "reset time"):
                    frontier.run(self.root, resume=True)
                clock.return_value = 5000
                second = frontier.run(self.root, resume=True)
        self.assertEqual(calls, [150, 145])
        self.assertEqual(second["active_seconds_used"], 10)
        self.assertEqual(second["attempts"][1]["resumed_session_id"], "same")
        self.assertEqual((self.root / "tools-state.json").read_bytes(), original)
        self.assertEqual((self.root / "report.md").read_text(), "existing partial findings")
        brief = frontier.read_json(self.root / "brief.json")
        self.assertEqual(brief["research_deadline_unix"], 5095)
        self.assertTrue((self.root / "attempts/001/receipt.json").is_file())
        self.assertTrue((self.root / "attempts/002/receipt.json").is_file())
        with self.assertRaisesRegex(ValueError, "already attempted"):
            frontier.run(self.root)
        (self.root / "report.md").write_text("changed")
        with self.assertRaisesRegex(ValueError, "output changed"):
            frontier.run(self.root, resume=True)

    def test_non_limit_failure_cannot_resume(self):
        (self.root / "run.json").write_text(json.dumps({"status": "failed"}))
        with self.assertRaisesRegex(ValueError, "usage-limit pause"):
            frontier.run(self.root, resume=True)

    def test_wait_uses_no_model_probes_then_resumes_once(self):
        (self.root / "claude-state.json").write_text(
            json.dumps({"session_id": "same", "resume_after": 160})
        )
        with patch("harness.frontier_research.time.time", return_value=100) as clock:

            def sleep(seconds):
                self.assertLessEqual(seconds, 30)
                clock.return_value += seconds

            with (
                patch("harness.frontier_research.time.sleep", side_effect=sleep),
                patch("harness.frontier_research.run", return_value={"status": "completed"}) as run,
            ):
                receipt = frontier.wait_and_resume(
                    self.root, {"status": "paused_rate_limit"}, max_wait_seconds=100
                )
        self.assertEqual(receipt["status"], "completed")
        run.assert_called_once_with(self.root, resume=True)
        self.assertEqual(clock.return_value, 175)
        self.assertEqual(frontier.read_json(self.root / "wait.json")["status"], "finished")

    def test_unknown_reset_does_not_probe(self):
        (self.root / "claude-state.json").write_text(
            json.dumps({"session_id": "same", "resume_after": None})
        )
        with patch("harness.frontier_research.run") as run:
            frontier.wait_and_resume(self.root, {"status": "paused_rate_limit"})
        run.assert_not_called()
        self.assertEqual(
            frontier.read_json(self.root / "wait.json")["status"], "needs_reset_time_or_session"
        )


if __name__ == "__main__":
    unittest.main()
