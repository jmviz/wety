import tempfile
from pathlib import Path
import unittest
from harness.ie_sensitivity_checks import check_deposit


class IESensitivityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.result = check_deposit()

    def test_combined_scenario_summaries(self):
        self.assertTrue(all(self.result["checks"].values()))
        self.assertEqual(
            [self.result["scenarios"][k]["retained_rows"] for k in ("SA7a", "SA8a", "SA8b")],
            [27003] * 3,
        )
        self.assertIsNone(self.result["epoch"])

    def test_configuration_identity_not_publication_votes(self):
        a, b = [self.result["scenarios"][k] for k in ("SA8a", "SA8b")]
        self.assertEqual(a["alignment_sha256"], b["alignment_sha256"])
        self.assertEqual((len(a["taxa"]), len(b["taxa"])), (94, 94))
        self.assertEqual((len(a["ancestry_constraints"]), len(b["ancestry_constraints"])), (0, 8))
        self.assertEqual(len(a["monophyly_labelled_distributions"]), 43)

    def test_changed_archive_fails_before_interpretation(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "changed.zip"
            p.write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "archive hash mismatch"):
                check_deposit(p)


if __name__ == "__main__":
    unittest.main()
