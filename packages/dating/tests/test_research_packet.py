import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from harness.research_packet import adopt, build, registry_raw, write
from workflow_tools.paths import REPO

PILOT = REPO / "data/workspaces/dating/_research/period-pilot-2026-10-03"
PILOT_PACKET = REPO / "data/workspaces/dating/_research/packets/period-pilot-2026-10-03"
TEXT = "Old English (ca. 450-1100) and Hittite 1650-1180 BC. Hittite 1650-1180 BC."
LANGS = {
    "ang": {"kind": "regular", "canonicalName": "Old English"},
    "hit": {"kind": "regular", "canonicalName": "Hittite"},
}


def row(aid, code, quote, native, **extra):
    return {
        "id": aid,
        "code": code,
        "source_id": "s1",
        "quote": quote,
        "locator": {"start": 0},
        "subject": {"code": "wikt:" + code, "mapping": "exact", "source_label": code},
        "event": "conventional_period",
        "role": "convention",
        "method": "standards_reference_name",
        "native_time": native,
        "qualifications": ["q"],
        **extra,
    }


class SyntheticRun(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.base = Path(tmp.name)
        self.run = self.base / "run"
        (self.run / "evidence").mkdir(parents=True)
        text = self.run / "evidence" / "s1.txt"
        text.write_text(TEXT)
        import hashlib

        tsha = hashlib.sha256(text.read_bytes()).hexdigest()
        (self.run / "tools-state.json").write_text(
            json.dumps(
                {
                    "sources": {
                        "s1": {
                            "text_file": "s1.txt",
                            "text_sha256": tsha,
                            "url": "https://example.org/t",
                            "raw_path": "raw/s1",
                            "raw_sha256": "r" * 64,
                        }
                    }
                }
            )
        )
        rows = [
            row(
                "a",
                "ang",
                "Old English (ca. 450-1100)",
                {
                    "form": "duration",
                    "unit": "CE",
                    "lower": 450,
                    "upper": 1100,
                    "approximate": True,
                },
            ),
            row(
                "h",
                "hit",
                "Hittite 1650-1180 BC",
                {"form": "duration", "unit": "BCE", "lower": 1650, "upper": 1180},
            ),
        ]
        (self.run / "assertions.jsonl").write_text("\n".join(json.dumps(r) for r in rows))
        self.review = {
            "packet_id": "test-packet",
            "claim_prefix": "ev_test_",
            "reviewer": "Reviewer A",
            "scope": "test",
            "review_independence": "one session",
            "retrieved": "2026-10-03T00:00:00+00:00",
            "sources": {"s1": {"id": "src_test", "title": "Test table"}},
            "claims": [
                {"assertion": "a", "status": "supported", "rationale": "checked"},
                {"assertion": "h", "status": "candidate", "start": TEXT.index("Hittite")},
            ],
        }

    def build(self):
        return build(self.run, self.review, LANGS, self.base / "texts")

    def test_claims_spans_and_times(self):
        result = self.build()
        ang, hit = result["claims"]
        self.assertTrue(ang["id"].startswith("ev_test_"))
        self.assertEqual((ang["subject"]["id"], ang["status"]), ("wikt:ang", "supported"))
        self.assertEqual(ang["assessment"]["reviewer"], "Reviewer A")
        span = ang["spans"][0]
        self.assertEqual(TEXT[span["start"] : span["end"]], "Old English (ca. 450-1100)")
        # BCE years are reordered for the registry's convention, not changed.
        self.assertEqual((hit["time"]["lower"], hit["time"]["upper"]), (-1649, -1179))
        self.assertEqual(hit["spans"][0]["start"], TEXT.index("Hittite"))
        self.assertIsNone(hit["assessment"]["reviewer"])
        self.assertEqual(result["packet"]["source_reviews"][0]["claim_ids"], [ang["id"], hit["id"]])

    def test_review_must_cover_every_assertion_and_locate_quotes(self):
        self.review["claims"].pop()
        with self.assertRaisesRegex(ValueError, "every run assertion"):
            self.build()
        self.review["claims"].append({"assertion": "h", "status": "candidate"})
        with self.assertRaisesRegex(ValueError, "not unique"):
            self.build()
        self.review["claims"][1] = {"assertion": "h", "status": "candidate", "start": 0}
        with self.assertRaisesRegex(ValueError, "does not match"):
            self.build()

    def test_write_registers_sources_and_texts_once(self):
        sources = self.base / "sources.json"
        with patch("harness.store.SOURCES_PATH", str(sources)):
            result = self.build()
            summary = write(result, self.base / "packet", self.base / "texts")
            self.assertEqual((summary["claims"], summary["supported"]), (2, 1))
            self.assertIn("src_test", json.loads(sources.read_text()))
            self.assertEqual(len(list((self.base / "texts").glob("*.txt"))), 1)
            packet = json.loads((self.base / "packet/packet.json").read_text())
            self.assertEqual(packet["claims_sha256"], result["packet"]["claims_sha256"])
            # A registered source id cannot be rebound to different text.
            result["registrations"]["src_test"]["sha256"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "different text"):
                write(result, self.base / "packet", self.base / "texts")

    def test_registry_raw_keeps_numbers(self):
        raw = registry_raw(
            {
                "form": "duration",
                "unit": "BCE",
                "lower": 900,
                "upper": 1200,
                "endpoint": "x",
                "value": None,
            }
        )
        self.assertEqual(raw, {"form": "duration", "unit": "BCE", "lower": 900, "upper": 1200})


@unittest.skipUnless((PILOT / "tools-state.json").exists(), "pilot run is local data")
class PilotReproduction(unittest.TestCase):
    def test_pilot_packet_and_decisions_reproduce(self):
        review = json.loads((PILOT / "coordinator-review/review.json").read_text())
        result = build(PILOT, review)
        self.assertEqual(result["claims_bytes"], (PILOT_PACKET / "claims.json").read_bytes())
        self.assertEqual(result["packet"], json.loads((PILOT_PACKET / "packet.json").read_text()))
        # The pilot's adoptions are recorded unchanged unless a later review superseded
        # them. This test must never write: it runs against the live registry.
        from harness.registry import build_bundle

        bundle = build_bundle()
        current = {d["subject_id"][5:]: d for d in bundle["decisions"]}
        claims = {c["id"]: c for c in bundle["claims"]}
        superseded = {
            code
            for code, choice in review["adopt"].items()
            if code not in current
            or claims[current[code]["claim_id"]]["provenance"].get("assertion_id")
            != choice["assertion"]
            or current[code]["rationale"] != choice["rationale"]
        }
        written = []
        with (
            patch("harness.review.decide_record", lambda d, b=None: written.append(d)),
            patch("harness.review.withdraw", side_effect=AssertionError("no writes")),
        ):
            results = adopt(review)
        self.assertEqual({k for k, v in results.items() if v != "unchanged"}, superseded)
        self.assertEqual({d["subject_id"][5:] for d in written}, superseded)


if __name__ == "__main__":
    unittest.main()
