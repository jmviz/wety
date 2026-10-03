"""Input loss must be observable independently of the model's output."""

import unittest
from unittest.mock import patch
from harness import datasets, llm


class InputCoverageTests(unittest.TestCase):
    def candidate(self):
        return {
            "title": "Study",
            "year": 2020,
            "datasets": [
                {
                    "url": "https://example.test/deposit",
                    "skipped": [{"name": "full.zip", "why": "size"}],
                    "files": [
                        {"name": n, "path": n}
                        for n in ("a.txt", "b.txt", "c.txt", "tree.nex", "missing.pdf")
                    ],
                }
            ],
        }

    def bundle(self):
        return datasets.dataset_bundle(self.candidate())

    def test_every_known_file_is_accounted_for_after_budget_exhaustion(self):
        with (
            patch.object(datasets, "historical", side_effect=lambda p: p),
            patch.object(
                datasets,
                "_read_text",
                side_effect=lambda p: None if p == "missing.pdf" else "abcdef",
            ),
            patch.object(datasets, "MAX_TEXT_PER_FILE", 4),
            patch.object(datasets, "MAX_TEXT_PER_SOURCE", 6),
        ):
            text, rows = self.bundle()
        self.assertEqual(
            [r["status"] for r in rows],
            [
                "not_retrieved",
                "truncated",
                "truncated",
                "budget_omitted",
                "tree_not_in_text_prompt",
                "unreadable_or_empty",
            ],
        )
        self.assertEqual([r["included_chars"] for r in rows], [0, 4, 2, 0, 0, 0])
        self.assertIn("b.txt", text)
        self.assertNotIn("c.txt", text)
        self.assertEqual(rows[1]["available_chars"], 6)
        self.assertNotEqual(rows[1]["text_sha256"], rows[1]["included_text_sha256"])

    def test_main_truncation_and_empty_text_are_explicit(self):
        evidence, coverage = llm.extraction_input({}, "x" * 900001)
        self.assertEqual(len(evidence), 900000)
        self.assertEqual(coverage["main_text"]["status"], "truncated")
        self.assertEqual(llm.extraction_input({}, "")[1]["main_text"]["status"], "empty")

    def test_metadata_is_in_prompt_but_not_quote_verification_evidence(self):
        c = {
            "title": "Study",
            "datasets": [
                {"url": "https://example.test", "skipped": [{"name": "unseen.pdf", "why": "size"}]}
            ],
        }
        evidence, coverage = llm.extraction_input(c, "Actual source.")
        prompt = llm.extract_user(c, "Actual source.")
        self.assertIn("unseen.pdf", prompt)
        self.assertNotIn("unseen.pdf", evidence)
        self.assertFalse(llm.verify_quote("unseen.pdf", evidence))
        self.assertEqual(coverage["attachments"][0]["status"], "not_retrieved")

    def test_omission_changes_prompt_even_when_included_evidence_is_identical(self):
        c = {
            "datasets": [
                {"url": "https://example.test", "skipped": [{"name": "unseen.pdf", "why": "size"}]}
            ]
        }
        self.assertEqual(
            llm.extraction_input(c, "Source.")[0], llm.extraction_input({}, "Source.")[0]
        )
        self.assertNotEqual(llm.extract_user(c, "Source."), llm.extract_user({}, "Source."))


if __name__ == "__main__":
    unittest.main()
