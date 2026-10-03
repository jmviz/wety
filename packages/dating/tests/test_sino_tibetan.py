import json
from pathlib import Path
import tempfile
import unittest

from harness.config import DATA_DIR
from harness.packets import validate_packet
from harness.sino_tibetan_checks import check_deposits, shortest_sample_interval


PACKET = Path(DATA_DIR) / "_research/packets/sino-tibetan-2026-09-20/packet.json"


class SinoTibetanReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.result = check_deposits()

    def test_reported_root_reproduces_only_after_declared_selection(self):
        r = self.result["zhang2020"]
        self.assertTrue(self.result["passed"])
        self.assertEqual((r["deposited_trees"], r["retained_trees"]), (1001, 900))
        self.assertEqual(round(r["mean_years_bp"]), 7983)
        self.assertEqual([round(v) for v in r["sample_hpd_95_years_bp"]], [4778, 11285])
        self.assertIsNone(r["calendar_epoch"])
        self.assertIn("unverified", r["convergence"])

    def test_sampled_tip_is_fixed_despite_reported_uniform(self):
        r = self.result["zhang2020"]["tip_ranges_years_bp"]
        for name, expected in [("Old_Chinese", 2550), ("Padam", 120), ("Shaiyang", 120)]:
            for value in r[name]:
                self.assertAlmostEqual(value, expected, places=6)

    def test_deposited_karen_sigma_is_not_table_value(self):
        prior = self.result["zhang2019_xml_priors"]["Karen.prior"]
        self.assertEqual(prior["normal"], {"mean": 1150.0, "sigma": 75.0})
        self.assertNotEqual(prior["normal"]["sigma"], 100.0)

    def test_sample_interval_keeps_mass_and_is_not_equal_tailed(self):
        self.assertEqual(shortest_sample_interval([0, 1, 2, 3, 100], 0.8), [0, 3])
        self.assertEqual(shortest_sample_interval([0, 1, 2, 3, 100], 1), [0, 100])
        with self.assertRaises(ValueError):
            shortest_sample_interval([])

    def test_packet_preserves_abstention_and_provenance(self):
        r = validate_packet(PACKET, {}, [])
        self.assertEqual(r["errors"], [])
        self.assertEqual((r["findings"], r["claims"], r["relations"]), (31, 0, 0))
        self.assertFalse(r["independent_review"])

    def test_changed_deposit_hash_is_rejected(self):
        packet = json.loads(PACKET.read_text())
        packet["source_reviews"][-1]["raw_sha256"] = "0" * 64
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "packet.json"
            path.write_text(json.dumps(packet))
            with self.assertRaisesRegex(ValueError, "source hash mismatch"):
                check_deposits(path)


if __name__ == "__main__":
    unittest.main()
