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


if __name__ == "__main__":
    unittest.main()
