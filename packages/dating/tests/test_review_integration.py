import json
from pathlib import Path
import tempfile
import unittest

from harness.config import DATA_DIR
from harness.packets import validate_packet


ROOT = Path(DATA_DIR) / "_research/packets"


class ReviewIntegrationTests(unittest.TestCase):
    def test_corrections_append_without_new_dates(self):
        path = ROOT / "uralic-review-integration-2026-09-20/packet.json"
        result = validate_packet(path, {}, [])
        self.assertEqual(result["errors"], [])
        self.assertEqual((result["findings"], result["claims"], result["relations"]), (4, 0, 0))
        self.assertEqual(
            result["review_independence"]["kind"], "separate_model_session_source_first"
        )
        self.assertFalse(result["review_independence"]["project_held_out"])
        findings = {f["id"]: f for f in json.loads(path.read_text())["findings"]}
        self.assertIn("2000", findings["COR01"]["spans"][0]["quote"])
        self.assertTrue(findings["COR02"]["phase_b_expansion"])

    def test_changed_review_receipt_fails(self):
        source = ROOT / "uralic-review-integration-2026-09-20/packet.json"
        packet = json.loads(source.read_text())
        packet["review_files"][0]["sha256"] = "0" * 64
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "packet.json"
            path.write_text(json.dumps(packet))
            (path.parent / "claims.json").write_bytes((source.parent / "claims.json").read_bytes())
            self.assertTrue(
                any("review file hash mismatch" in e for e in validate_packet(path)["errors"])
            )

    def test_ie_provenance_and_abstention(self):
        path = ROOT / "indo-european-2026-09-20/packet.json"
        result = validate_packet(path, {}, [])
        self.assertEqual(result["errors"], [])
        self.assertEqual((result["findings"], result["claims"], result["relations"]), (23, 0, 0))
        sources = {s["key"]: s for s in result["source_reviews"]}
        self.assertEqual(sources["kassian"]["raw_source_kind"], "web_tool_capture")
        self.assertIn("third-party", sources["heggarty-main"]["completeness"])
        self.assertFalse(result["independent_review"])


if __name__ == "__main__":
    unittest.main()
