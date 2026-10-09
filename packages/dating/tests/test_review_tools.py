import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from harness import review_tools
from harness.briefs import proto_line


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value if isinstance(value, str) else json.dumps(value))


class ReviewToolTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.run = Path(tmp.name) / "proto-test-run"
        write(
            self.run / "brief.json",
            {
                "profile_name": "codex-sol",
                "profile": {"backend": "codex", "model": "gpt-6.1-sol", "effort": "high"},
                "research_seconds": 2700,
                "delivery_seconds": 600,
            },
        )
        write(self.run / "run.json", {"attempt_started": 1791334974.48, "status": "completed"})
        text = "Proto-X broke up about 3000 years ago. Proto-X broke up about 3000 years ago."
        write(self.run / "evidence/s1.txt", text)
        write(
            self.run / "tools-state.json", {"sources": {"s1": {"text_file": "s1.txt", "url": "u"}}}
        )
        row = {
            "id": "a1",
            "code": "xx-pro",
            "source_id": "s1",
            "quote": "Proto-X broke up about 3000 years ago.",
            "locator": {"start": 39},
            "subject": {"code": "wikt:xx-pro", "mapping": "exact"},
            "event": "crown_divergence",
            "role": "hypothesis",
            "native_time": {"form": "point", "unit": "years_ago", "value": 3000},
            "proposed_status": "candidate",
            "citation": "Auderset, Sandra, Simon J. Greenhill & Eric W. Campbell. 2023. A title. Journal.",
        }
        write(self.run / "assertions.jsonl", json.dumps(row) + "\n")

    def test_researcher_comes_from_the_frozen_brief(self):
        who = review_tools.researcher(self.run)
        self.assertEqual(who["model"], "gpt-6.1-sol")
        self.assertEqual(
            who["label"], "gpt-6.1-sol (high effort, Codex CLI on ChatGPT subscription)"
        )

    def test_citations_parse_to_authors_and_year(self):
        authors, year = review_tools.parse_citation(
            "Auderset, Sandra, Simon J. Greenhill & Eric W. Campbell. 2023. Title."
        )
        self.assertEqual(
            (authors, year), (["Sandra Auderset", "Simon J. Greenhill", "Eric W. Campbell"], 2023)
        )
        self.assertEqual(
            review_tools.parse_citation("Rau, Felix & Paul Sidwell. 2019. T.")[0],
            ["Felix Rau", "Paul Sidwell"],
        )
        self.assertIsNone(
            review_tools.parse_citation("Project. n.d. Intro (accessed 2026-10-06).")[1]
        )

    def test_review_names_the_actual_researcher_and_locates_repeated_quotes(self):
        with (
            patch.object(review_tools, "_known_sources", return_value={}),
            patch("harness.store.load_sources", return_value={}),
        ):
            with self.assertRaisesRegex(ValueError, "reviewer"):
                review_tools.review_from_judgments(self.run, {}, reviewer=" ")
            review_tools.review_from_judgments(
                self.run,
                {"status": {"a1": "supported"}, "adopt": {"xx-pro": "a1"}},
                reviewer="Reviewer R",
            )
        rv = json.loads((self.run / "coordinator-review/review.json").read_text())
        self.assertTrue(rv["reviewer"].startswith("gpt-6.1-sol (high effort"))
        self.assertIn("reviewed by Reviewer R", rv["reviewer"])
        self.assertEqual(
            rv["decision_reviewer"], "Reviewer R, reviewer of gpt-6.1-sol's batch proto-test-run"
        )
        self.assertTrue(rv["retrieved"].startswith("2026-10-07"))
        (c,) = rv["claims"]
        self.assertEqual((c["status"], c["start"]), ("supported", 39))
        self.assertIn("status candidate -> supported (reviewer).", c["corrections"])
        self.assertEqual(rv["sources"]["s1"]["authors"][0], "Sandra Auderset")
        self.assertEqual(rv["adopt"]["xx-pro"]["assertion"], "a1")

    def test_clade_adoption_finds_the_exact_taxon_set(self):
        clades = [
            {"id": "c1", "subject": {"taxa": ["A", "B"]}},
            {"id": "c2", "subject": {"taxa": ["A", "B", "C"]}},
        ]
        recorded = []
        with tempfile.TemporaryDirectory() as d:
            write(Path(d) / "_evidence/tree.json", clades)
            with (
                patch.object(review_tools, "DATA_DIR", d),
                patch("harness.registry.build_bundle", return_value={}),
                patch("harness.registry.bundle_decision_digest", return_value="dg"),
                patch(
                    "harness.review.decide_record",
                    side_effect=lambda raw, b: recorded.append(raw) or {"ok": 1},
                ),
            ):
                spec = {
                    "reviewer": "R",
                    "decisions": [
                        {
                            "code": "xx-pro",
                            "analysis": "tree",
                            "taxa": ["B", "A"],
                            "proto_mapping": "A and B: the first split.",
                            "rationale": "Why.",
                            "alternatives": [
                                {"analysis": "tree", "taxa": "root", "proto_mapping": "All."}
                            ],
                        }
                    ],
                }
                review_tools.adopt_clades(spec)
                spec["decisions"][0]["taxa"] = ["A"]
                with self.assertRaisesRegex(ValueError, "0 clades"):
                    review_tools.adopt_clades(spec)
        (raw,) = recorded
        self.assertEqual((raw["claim_id"], raw["evidence_digest"]), ("c1", "dg"))
        self.assertEqual(raw["proto_alternatives"][0]["claim_id"], "c2")

    def test_proto_brief_lines_mark_asjp_only_codes(self):
        langs = {"xx-pro": {"canonicalName": "Proto-X"}}
        periods = {"xx-pro": {"evidence": [{"source": "Eric W. Holman et al. 2011"}]}}
        line = proto_line("xx-pro", "Smith 2001", langs, {"xx-pro": {"items": 7}}, periods)
        self.assertEqual(line, "xx-pro Proto-X (7); ASJP only. Leads: Smith 2001")


if __name__ == "__main__":
    unittest.main()
