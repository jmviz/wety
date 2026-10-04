import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from harness import review


class WithdrawTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        for name, value in (
            ("DATA_DIR", str(self.root)),
            ("DATING_HISTORY", self.root / "history"),
        ):
            p = patch.object(review, name, value)
            p.start()
            self.addCleanup(p.stop)
        self.decisions = [
            {"subject_id": "wikt:hit", "event": "documentary_sampling", "claim_id": "a"},
            {"subject_id": "wikt:hit", "event": "first_attestation", "claim_id": "b"},
        ]
        folder = self.root / "_decisions"
        folder.mkdir()
        (folder / "decisions.json").write_text(json.dumps(self.decisions))

    def test_withdrawal_removes_one_decision_and_keeps_it_with_reason(self):
        result = review.withdraw("wikt:hit", "documentary_sampling", "Reviewer", "better evidence")
        self.assertEqual(result["withdrawn_claim"], "a")
        left = json.loads((self.root / "_decisions/decisions.json").read_text())
        self.assertEqual(left, self.decisions[1:])
        [record] = json.loads((self.root / "_decisions/withdrawals.json").read_text())
        self.assertEqual(
            (record["decision"], record["rationale"]), (self.decisions[0], "better evidence")
        )
        self.assertEqual(len(list((self.root / "history/decisions").iterdir())), 1)
        with self.assertRaisesRegex(ValueError, "no decision"):
            review.withdraw("wikt:hit", "documentary_sampling", "Reviewer", "again")
        with self.assertRaisesRegex(ValueError, "rationale"):
            review.withdraw("wikt:hit", "first_attestation", "Reviewer", " ")


class ReadoptTests(unittest.TestCase):
    def test_stale_decisions_are_recorded_again_with_a_note(self):
        stale = {
            "subject_id": "wikt:ang",
            "event": "conventional_period",
            "claim_id": "c1",
            "evidence_digest": "old",
            "reviewer": "R0",
            "rationale": "Chosen.",
            "dependencies": [],
            "kind": "editorial_default",
        }
        gone = {**stale, "subject_id": "wikt:hit", "claim_id": "c2"}
        other = {**stale, "subject_id": "wikt:la"}
        bundle = {
            "rejected_decisions": [
                {"decision": stale, "reason": "stale decision: the subject's evidence changed"},
                {"decision": gone, "reason": "stale decision: the subject's evidence changed"},
                {"decision": other, "reason": "stale decision: the subject's evidence changed"},
            ]
        }
        recorded = []

        def decide(raw, _bundle):
            if raw["claim_id"] == "c2":
                raise ValueError("claim not eligible for this subject and event")
            recorded.append(raw)

        with (
            patch.object(review, "build_bundle", return_value=bundle),
            patch.object(review, "bundle_decision_digest", return_value="new"),
            patch.object(review, "decide_record", side_effect=decide),
        ):
            result = review.readopt("R1", "Still holds.", {"wikt:ang", "wikt:hit"})
        self.assertEqual(result["readopted"], ["wikt:ang/conventional_period"])
        self.assertEqual(result["needs_decision"][0]["subject_id"], "wikt:hit")
        [raw] = recorded
        self.assertEqual(
            (raw["evidence_digest"], raw["reviewer"], raw["rationale"]),
            ("new", "R1", "Chosen. Still holds."),
        )
        self.assertNotIn("kind", raw)


if __name__ == "__main__":
    unittest.main()
