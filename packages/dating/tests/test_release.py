import json
import unittest
from unittest.mock import patch

from harness import release

PERIODS = {
    "generation": "g",
    "languages": [
        {
            "code": "eya",
            "name": "Eyak",
            "label": "1805–2008 CE",
            "basis": "last_speaker",
            "kind": "attested",
            "spoken": {
                "start": {"earliest": 1805, "latest": 1805},
                "end": {"earliest": 2008, "latest": 2008},
            },
            "evidence": [
                {
                    "claim_id": "c1",
                    "event": "native_transmission_end",
                    "role": "hypothesis",
                    "method": "expert_estimate",
                    "quote": "In 2008 Eyak became the first Alaska Native language to disappear.",
                    "locator": "p. 107",
                    "source": "Holton 2012",
                }
            ],
            "alternatives": [
                {
                    "claim_id": "c2",
                    "event": "first_attestation",
                    "quote": "x" * 400,
                    "label": "1805",
                }
            ],
        }
    ],
}
CLAIMS = [
    {"id": "c1", "source_id": "s1", "provenance": {"packet_id": "secret-run"}},
    {"id": "c2", "source_id": "s2"},
]
SOURCES = {
    "s1": {
        "id": "s1",
        "citation": "Holton 2012.",
        "authors": ["Gary Holton"],
        "year": 2012,
        "raw_sha256": "f" * 64,
    },
    "s2": {"id": "s2", "citation": "Krauss.", "authors": ["Michael Krauss"]},
}
DECISION = {
    "subject_id": "wikt:eya",
    "event": "x",
    "rationale": "Why.",
    "reviewer": "Reviewer R",
    "evidence_digest": "d",
}


class ReleaseTests(unittest.TestCase):
    def graph(self, rights):
        with (
            patch("harness.registry.load_evidence", return_value=CLAIMS),
            patch("harness.store.load_sources", return_value=SOURCES),
            patch.object(release, "rights", return_value=rights),
            patch("harness.ledger.current", return_value=({"k": DECISION}, {})),
        ):
            return release.build(PERIODS)

    def test_quotes_follow_reuse_rights(self):
        self.assertEqual(release.quote_for("abc", "open"), ("abc", "full"))
        self.assertEqual(release.quote_for("abc", None), (None, "withheld"))
        short, how = release.quote_for("word " * 100, "short_quote")
        self.assertEqual(how, "shortened")
        self.assertTrue(len(short) <= release.QUOTE_LIMIT and short.endswith("…"))
        g = self.graph({"s1": {"reuse": "open"}, "s2": {"reuse": "short_quote"}})
        self.assertEqual(
            g["stats"]["quotes"], {"full": 1, "shortened": 1, "withheld": 0, "none": 0}
        )
        g = self.graph({})
        self.assertEqual(g["stats"]["quotes"]["withheld"], 2)

    def test_release_carries_no_process_records(self):
        g = self.graph({"s1": {"reuse": "open"}})
        text = release.turtle(g) + json.dumps(release.jsonld(g))
        for internal in ("secret-run", "f" * 64, "evidence_digest", "claim_id", '"d"'):
            self.assertNotIn(internal, text)
        period = g["nodes"][release.BASE + "period/eya"]
        self.assertEqual(period["wety:endLatest"][0], {"@value": 2008, "@type": "xsd:integer"})
        self.assertEqual(period["wety:reviewedBy"][0]["@value"], "Reviewer R")
        self.assertIn("wety:alternative", period)


if __name__ == "__main__":
    unittest.main()
