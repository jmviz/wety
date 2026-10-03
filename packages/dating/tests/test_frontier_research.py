import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from harness.frontier_tools import ResearchTools, check, sha, tool_response
from harness.frontier_research import REPO, clean_env, codex_command, prepare, router_run


class FrontierTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.brief = {
            "deadline_unix": time.time() + 120,
            "research_deadline_unix": time.time() + 60,
            "limits": {
                "searches": 2,
                "acquisitions": 2,
                "tool_calls": 20,
                "model_turns": 4,
                "openrouter_cost_stop_usd": 2,
            },
            "profile": {"backend": "codex", "model": "gpt-6-astra", "effort": "high"},
            "transport": {},
        }
        (self.root / "brief.json").write_text(json.dumps(self.brief))
        self.tools = ResearchTools(self.root)

    def test_failed_network_consumes_allowance(self):
        with patch.object(self.tools, "fetch", side_effect=ValueError("unavailable")):
            for _ in range(2):
                with self.assertRaisesRegex(ValueError, "unavailable"):
                    self.tools.call("search", {"query": "language dates"})
            with self.assertRaisesRegex(ValueError, "limit reached"):
                self.tools.call("search", {"query": "again"})
        self.assertEqual(self.tools.state()["searches"], 2)

    def test_failed_http_is_visible_without_exception_body_or_new_allowance(self):
        from urllib.error import HTTPError

        error = HTTPError("https://example.org", 403, "private response body", None, None)
        with patch.object(self.tools, "fetch", side_effect=error):
            result = tool_response(self.tools, "acquire", {"url": "https://example.org"})
        self.assertEqual(result["error"], {"type": "HTTPError", "http_status": 403})
        self.assertNotIn("private response", json.dumps(result))
        self.assertEqual(self.tools.state()["acquisitions"], 1)
        event = json.loads((self.root / "tool-events.jsonl").read_text().splitlines()[-1])
        self.assertEqual(event["failure"], result["error"])
        self.assertEqual(event["status"], "failed")

    def test_timeout_and_parse_failures_stay_distinct(self):
        from urllib.error import URLError

        failures = [
            (
                URLError(TimeoutError("private endpoint")),
                {"type": "URLError", "reason_type": "TimeoutError", "timeout": True},
            ),
            (json.JSONDecodeError("private payload", "", 0), {"type": "JSONDecodeError"}),
        ]
        for error, expected in failures:
            with patch.object(self.tools, "dispatch", side_effect=error):
                result = tool_response(self.tools, "status", {})
            self.assertEqual(result["error"], expected)

    def test_output_path_and_revisions(self):
        with self.assertRaises(ValueError):
            self.tools.call("write", {"name": "../README.md", "content": "bad"})
        self.tools.call("write", {"name": "report.md", "content": "first"})
        self.tools.call("write", {"name": "report.md", "content": "second"})
        self.assertEqual(next((self.root / "revisions").iterdir()).read_text(), "first")

    def test_large_discovery_and_status_metadata_are_compact_but_retained(self):
        from harness.frontier_tools import REPO

        # The real acquirer returns a repository path; use the same root for this fixture.
        with tempfile.TemporaryDirectory(dir=REPO / "data/workspaces/dating") as tmp:
            raw = Path(tmp) / "search.json"
            loc = {
                "landing_page_url": "https://example.org/paper",
                "pdf_url": "https://example.org/p.pdf",
                "source": {"large_irrelevant_metadata": "x" * 100000},
            }
            raw.write_text(
                json.dumps(
                    {
                        "results": [
                            {"title": "Study", "locations": [loc] * 20, "best_oa_location": loc}
                        ]
                    }
                )
            )
            digest = sha(raw)
            with patch.object(self.tools, "fetch", return_value=raw):
                result = self.tools.call("search", {"query": "Japonic", "index": "openalex"})
            self.assertLess(len(json.dumps(result)), 4000)
            self.assertEqual(result["results"][0]["locations_omitted"], 17)
            self.assertEqual(
                result["results"][0]["best_oa_location"]["pdf_url"], "https://example.org/p.pdf"
            )
            self.assertEqual(sha(raw), digest)
        state = self.tools.state()
        state["sources"] = {
            "s": {"source_id": "s", "url": "https://example.org/p", "headings": ["long"] * 10000}
        }
        (self.root / "tools-state.json").write_text(json.dumps(state))
        status = self.tools.call("status", {})
        self.assertNotIn("headings", status["sources"]["s"])
        self.assertEqual(len(self.tools.state()["sources"]["s"]["headings"]), 10000)

    def test_deadline_allows_delivery_not_more_research(self):
        self.tools.brief["research_deadline_unix"] = time.time() - 1
        with self.assertRaisesRegex(ValueError, "period ended"):
            self.tools.call("acquire", {"url": "https://example.org"})
        self.tools.call("write", {"name": "report.md", "content": "partial"})
        (self.root / "STOP").touch()
        with self.assertRaisesRegex(ValueError, "stopped"):
            self.tools.call("status", {})

    def test_quote_checks_detect_mutation_and_unknown_source(self):
        p = self.root / "evidence"
        p.mkdir()
        text = p / "text.txt"
        text.write_text("A root age of 5000 BP was estimated.")
        (self.root / "tools-state.json").write_text(
            json.dumps({"sources": {"s": {"text_file": "text.txt", "text_sha256": sha(text)}}})
        )
        row = {
            "id": "a",
            "source_id": "s",
            "quote": "5000 BP",
            "subject": "root",
            "event": "split",
            "role": "posterior",
            "method": "model",
            "native_time": {},
            "qualifications": [],
        }
        for name in ("sources.json", "self-check.json"):
            (self.root / name).write_text("[]")
        (self.root / "report.md").write_text("partial")
        (self.root / "assertions.jsonl").write_text(json.dumps(row))
        self.assertEqual(check(self.root)["errors"], [])
        text.write_text("changed")
        self.assertIn("changed text", str(check(self.root)["errors"]))
        row["source_id"] = []
        (self.root / "assertions.jsonl").write_text(json.dumps(row))
        self.assertIn("unknown retained source", str(check(self.root)["errors"]))

    def test_null_limits_are_uncapped(self):
        self.tools.brief["limits"].update(searches=None, acquisitions=None, tool_calls=None)
        with patch.object(self.tools, "fetch", side_effect=ValueError("unavailable")):
            for _ in range(30):
                with self.assertRaises(ValueError):
                    self.tools.call("acquire", {"url": "https://example.org"})
        state = self.tools.state()
        self.assertEqual((state["acquisitions"], state["tool_calls"]), (30, 30))
        # Deadlines still apply to uncapped runs.
        self.tools.brief["research_deadline_unix"] = time.time() - 1
        with self.assertRaisesRegex(ValueError, "period ended"):
            self.tools.call("search", {"query": "x"})

    def test_find_returns_rows_with_exact_offsets(self):
        p = self.root / "evidence"
        p.mkdir()
        text = p / "t.txt"
        rows = "ID,Name,First,Last\nolde1238,Old English (ca. 450-1100),450,1100\nmidd1317,Middle English,1100,1500\n"
        text.write_text(rows + "x" * 5000 + "Old English again" + "y" * 5000)
        (self.root / "tools-state.json").write_text(
            json.dumps({"sources": {"s": {"text_file": "t.txt", "text_sha256": sha(text)}}})
        )
        found = self.tools.call("find", {"source_id": "s", "query": "old english"})
        self.assertEqual(found["header"]["text"], "ID,Name,First,Last")
        first, long = found["matches"]
        self.assertEqual(first["line"]["text"], "olde1238,Old English (ca. 450-1100),450,1100")
        full = text.read_text()
        self.assertEqual(full[first["match_start"] : first["match_end"]], "Old English")
        line = first["line"]
        self.assertEqual(full[line["start"] : line["end"]], line["text"])
        # A very long line is clipped around the match, offsets still exact.
        self.assertLessEqual(len(long["line"]["text"]), 2000 + len("Old English again"))
        self.assertEqual(full[long["line"]["start"] : long["line"]["end"]], long["line"]["text"])
        found = self.tools.call("find", {"source_id": "s", "query": r"\d{4},1500", "regex": True})
        self.assertEqual([m["line"]["text"][:8] for m in found["matches"]], ["midd1317"])
        found = self.tools.call("find", {"source_id": "s", "query": "e", "max_matches": 1})
        self.assertTrue(found["truncated"])
        with self.assertRaises(ValueError):
            self.tools.call("find", {"source_id": "s", "query": ""})

    def test_codex_is_subscription_tool_enabled_single_model(self):
        with patch("shutil.which", return_value="/bin/codex"):
            command = codex_command(self.root, self.brief)
        text = " ".join(command)
        for required in (
            'forced_login_method="chatgpt"',
            "features.multi_agent=false",
            "mcp_servers.research.command=",
            'model_reasoning_effort="high"',
            'service_tier="default"',
            'mcp_servers.research.tools.research_action.approval_mode="approve"',
        ):
            self.assertIn(required, text)
        self.assertNotIn("--output-schema", command)
        with patch.dict(
            "os.environ",
            {
                "OPENAI_API_KEY": "secret",
                "ANTHROPIC_API_KEY": "secret",
                "OPENROUTER_API_KEY": "secret",
                "CODEX_HOME": "/auth",
            },
        ):
            env = clean_env("codex")
        self.assertNotIn("OPENAI_API_KEY", env)
        self.assertNotIn("OPENROUTER_API_KEY", env)
        self.assertEqual(env["CODEX_HOME"], "/auth")

    def test_router_preserves_reasoning_and_stops_on_missing_cost(self):
        from io import BytesIO

        (self.root / "prompt.txt").write_text("research")
        (self.root / "instructions.txt").write_text("contract")
        self.brief["profile"] = {
            "model": "openai/gpt-6-astra",
            "effort": "high",
            "max_output_tokens": 16000,
            "provider": {"require_parameters": True, "allow_fallbacks": False},
        }
        responses = [
            {
                "usage": {"cost": 0.1},
                "choices": [
                    {
                        "message": {
                            "role": "assistant",
                            "content": None,
                            "reasoning_details": [
                                {"type": "reasoning.encrypted", "data": "signed"}
                            ],
                            "tool_calls": [
                                {
                                    "id": "1",
                                    "type": "function",
                                    "function": {
                                        "name": "research_action",
                                        "arguments": json.dumps(
                                            {"action": "status", "arguments": "{}"}
                                        ),
                                    },
                                }
                            ],
                        }
                    }
                ],
            },
            {
                "usage": {"cost": 0.1},
                "choices": [{"message": {"role": "assistant", "content": "done"}}],
            },
        ]
        requests = []

        def post(req, **kwargs):
            requests.append(json.loads(req.data))
            return BytesIO(json.dumps(responses.pop(0)).encode())

        with (
            patch.dict("os.environ", {"OPENROUTER_API_KEY": "test"}),
            patch("urllib.request.urlopen", side_effect=post),
        ):
            router_run(self.root, self.brief)
        self.assertEqual(requests[1]["messages"][2]["reasoning_details"][0]["data"], "signed")
        self.assertEqual(requests[1]["messages"][3]["tool_call_id"], "1")
        self.assertEqual((self.root / "final.txt").read_text(), "done")
        self.assertAlmostEqual(
            json.loads((self.root / "usage.json").read_text())["reported_cost_usd"], 0.2
        )

    def test_claude_uses_subscription_and_only_shared_tool(self):
        import asyncio
        from claude_agent_sdk import ResultMessage
        from harness.frontier_research import claude_run

        (self.root / "prompt.txt").write_text("research")
        (self.root / "instructions.txt").write_text("contract")
        self.brief["profile"] = {"model": "claude-opus-5", "effort": "high"}
        captured = []

        async def query(**kwargs):
            captured.append(kwargs["options"])
            yield ResultMessage(
                subtype="success",
                duration_ms=1,
                duration_api_ms=1,
                is_error=False,
                num_turns=2,
                session_id="session",
                usage={"output_tokens": 10},
                total_cost_usd=0.2,
                result="done",
            )

        with (
            patch.dict(
                "os.environ",
                {"CLAUDE_CODE_OAUTH_TOKEN": "test", "ANTHROPIC_API_KEY": "must-remove"},
            ),
            patch("claude_agent_sdk.query", query),
        ):
            asyncio.run(claude_run(self.root, self.brief))
            import os

            self.assertNotIn("ANTHROPIC_API_KEY", os.environ)
        options = captured[0]
        self.assertEqual(options.allowed_tools, ["mcp__research__research_action"])
        self.assertEqual(options.thinking, {"type": "adaptive"})
        self.assertIsNone(options.fallback_model)
        usage = json.loads((self.root / "usage.json").read_text())
        self.assertIsNone(usage["cost_usd"])
        self.assertEqual(usage["nominal_sdk_cost_usd"], 0.2)

    def test_unknown_router_cost_prevents_second_paid_call(self):
        from io import BytesIO

        (self.root / "prompt.txt").write_text("research")
        (self.root / "instructions.txt").write_text("contract")
        self.brief["profile"] = {
            "model": "openai/gpt-6-astra",
            "effort": "high",
            "max_output_tokens": 16000,
            "provider": {"allow_fallbacks": False},
        }
        response = {
            "usage": {},
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": None,
                        "tool_calls": [
                            {
                                "id": "1",
                                "type": "function",
                                "function": {
                                    "name": "research_action",
                                    "arguments": json.dumps(
                                        {"action": "status", "arguments": "{}"}
                                    ),
                                },
                            }
                        ],
                    }
                }
            ],
        }
        with (
            patch.dict("os.environ", {"OPENROUTER_API_KEY": "test"}),
            patch(
                "urllib.request.urlopen", return_value=BytesIO(json.dumps(response).encode())
            ) as post,
        ):
            with self.assertRaisesRegex(RuntimeError, "cost unavailable"):
                router_run(self.root, self.brief)
        self.assertEqual(post.call_count, 1)
        self.assertFalse(json.loads((self.root / "usage.json").read_text())["cost_complete"])

    def prepare_in(self, repo, name, **kwargs):
        with patch("harness.frontier_research.REPO", repo):
            return prepare(repo / "data/workspaces/dating/_research" / name, "Pilot", **kwargs)

    def fake_repo(self):
        repo = self.root.resolve() / "repo"
        (repo / "data/workspaces/dating/_research").mkdir(parents=True)
        (repo / "packages/dating/prompts").mkdir(parents=True)
        for name in ("README.md", "prompts/single-frontier.md"):
            (repo / "packages/dating" / name).write_text(
                (REPO / "packages/dating" / name).read_text()
            )
        return repo

    def test_brief_option_replaces_family_template_and_is_frozen(self):
        repo = self.fake_repo()
        brief = repo / "packages/dating/prompts/period.md"
        brief.write_text("# Pilot\n\n```text\nVerify periods for {{FAMILY}}.\n```\nnotes\n")
        root = self.prepare_in(repo, "custom", brief_path=brief)
        prompt = (root / "prompt.txt").read_text()
        self.assertTrue(prompt.startswith("Verify periods for Pilot.\n\nFrozen run settings:"))
        self.assertNotIn("notes", prompt)
        frozen = json.loads((root / "brief.json").read_text())
        self.assertEqual(frozen["brief_file"], "packages/dating/prompts/period.md")
        self.assertEqual(frozen["brief_file_sha256"], sha(brief))
        # Tools, instructions and the run-time hash checks are those of a family run.
        default = self.prepare_in(repo, "default")
        self.assertIn(
            "Research chronological evidence for Pilot", (default / "prompt.txt").read_text()
        )
        self.assertEqual(
            (root / "instructions.txt").read_text(), (default / "instructions.txt").read_text()
        )
        self.assertEqual(
            json.loads((root / "prepared.json").read_text())["implementation"],
            json.loads((default / "prepared.json").read_text())["implementation"],
        )

    def test_brief_without_fence_is_used_whole(self):
        repo = self.fake_repo()
        brief = self.root / "plain.txt"
        brief.write_text("Whole brief for {{FAMILY}}.")
        root = self.prepare_in(repo, "plain", brief_path=brief)
        self.assertTrue((root / "prompt.txt").read_text().startswith("Whole brief for Pilot."))
        self.assertEqual(
            json.loads((root / "brief.json").read_text())["brief_file"], str(brief.resolve())
        )

    def test_mcp_wire_exposes_only_research_action(self):
        import asyncio, sys
        from mcp import ClientSession, StdioServerParameters
        from mcp.client.stdio import stdio_client

        async def exercise():
            async with stdio_client(
                StdioServerParameters(
                    command=sys.executable, args=["-m", "harness.frontier_tools", str(self.root)]
                )
            ) as (reader, writer):
                async with ClientSession(reader, writer) as session:
                    await session.initialize()
                    listing = await session.list_tools()
                    self.assertEqual([t.name for t in listing.tools], ["research_action"])
                    result = await session.call_tool(
                        "research_action", {"action": "status", "arguments": "{}"}
                    )
                    self.assertFalse(result.is_error)
                    self.assertEqual(json.loads(result.content[0].text)["tool_calls"], 1)
                    result = await session.call_tool(
                        "research_action", {"action": "unknown", "arguments": "{}"}
                    )
                    self.assertEqual(
                        json.loads(result.content[0].text)["error"]["type"], "ValueError"
                    )

        asyncio.run(exercise())


if __name__ == "__main__":
    unittest.main()
