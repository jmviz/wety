import copy, json, tempfile, unittest
from pathlib import Path
from unittest.mock import patch
from harness.compact_extraction import (
    Assertion,
    CompactExtraction,
    inspect_compact,
    inspect_review,
    Judgment,
    extraction_system,
    review_payload,
)
from harness.section_packets import build_packets


def assertion():
    return {
        "subject": "Sample inscription",
        "subject_type": "documentary_sample",
        "event": "first_attestation",
        "role": "secondary_report",
        "method": "palaeography",
        "native_time": {"form": "bound", "unit": "century_BCE", "value": 3, "relation": "by"},
        "expression": "by the third century BCE",
        "quote": "The inscription is dated by the third century BCE.",
        "locator": "Dating",
        "scenario": None,
        "reported_from": "A named original study",
        "qualifications": [],
        "relative_constraints": [],
        "sampled_taxa": [],
    }


def response(rows):
    return json.dumps({"assertions": rows, "gaps": [], "conflicts": [], "overflow": False})


class CompactTests(unittest.TestCase):
    def test_native_calendar_prompt_and_numeric_abstention_flag(self):
        a = assertion()
        a["native_time"] = {"form": "point", "unit": "BCE", "value": -1199}
        with self.assertRaises(ValueError):
            Assertion.model_validate(a, strict=True)
        a["native_time"] = {
            "form": "event_range",
            "unit": "century_CE",
            "lower": 5,
            "upper": 5,
            "precision": "century",
        }
        Assertion.model_validate(a, strict=True)
        prompt = extraction_system("Dravidian")
        self.assertIn("1200 BCE is value 1200", prompt)
        self.assertNotIn("1200 BCE = -1199", prompt)
        a["native_time"] = {"form": "unknown"}
        a["expression"] = "254 BCE"
        r = inspect_compact(response([a]), build_packets(a["quote"], "s")["packets"][0])
        self.assertIn(
            "numeric expression with unknown native time requires review",
            r["claims"][0]["semantic_flags"],
        )

    def test_compact_rejects_derived_calendar_catalog_and_legacy_fields(self):
        a = assertion()
        for field, value in [
            ("earliest", -300),
            ("glottocode", "tami1289"),
            ("kind", "attestation_first"),
        ]:
            with self.assertRaises(ValueError):
                Assertion.model_validate({**a, field: value}, strict=True)
        with self.assertRaises(ValueError):
            Assertion.model_validate(
                {**a, "native_time": {**a["native_time"], "mean": 5}}, strict=True
            )
        self.assertNotIn("conversion", json.dumps(CompactExtraction.model_json_schema()))

    def test_partial_recovery_exact_spans_and_unknown_calendar(self):
        a = assertion()
        bad = copy.deepcopy(a)
        bad["native_time"]["value"] = "3"
        p = build_packets(a["quote"], "s")["packets"][0]
        r = inspect_compact(response([a, bad]), p)
        self.assertEqual(r["valid_claims"], 1)
        self.assertEqual(r["status"], "partial")
        self.assertEqual(r["claims"][0]["calendar_projection"]["value"], -200)
        a["native_time"] = {"form": "point", "unit": "years_ago", "value": 5000}
        a["quote"] = "This sampled lineage dates to 5000 years ago."
        r = inspect_compact(response([a]), build_packets(a["quote"], "s")["packets"][0])
        self.assertIn("calendar_abstention", r["claims"][0])
        self.assertTrue(r["claims"][0]["mechanically_ready"])

    def test_cap_and_unknown_fields_cannot_create_complete_document(self):
        a = assertion()
        p = build_packets(a["quote"], "s")["packets"][0]
        d = json.loads(response([a]))
        d["overflow"] = True
        self.assertEqual(inspect_compact(json.dumps(d), p)["status"], "partial")
        self.assertEqual(inspect_compact(response([a] * 13), p)["status"], "partial")
        d = json.loads(response([a]))
        d["gaps"] = [{"kind": "other", "description": "x", "next_action": "x", "invented": True}]
        self.assertFalse(inspect_compact(json.dumps(d), p)["gaps"][0]["schema_valid"])

    def test_review_cannot_approve_wrong_ids_missing_evidence_or_failed_dimensions(self):
        a = assertion()
        packet = build_packets(a["quote"], "s")["packets"][0]
        task = {
            "packet": packet,
            "targets": [
                {"assertion_id": "a", "mechanically_ready": True, "primary_evidence": True}
            ],
        }
        system, user, _ = review_payload(task)
        self.assertIn("cannot encode mixed-era endpoints", system)
        self.assertIn("PROPOSAL SCHEMA", user)
        j = {
            "assertion_id": "a",
            "disposition": "supported",
            "subject_event": "supported",
            "role_dependency": "supported",
            "native_time": "supported",
            "scenario_scope": "supported",
            "evidence_quote": a["quote"],
            "rationale": "In source.",
        }

        def review(rows):
            return inspect_review(
                json.dumps({"judgments": rows, "omissions": [], "context_gaps": []}), task
            )

        self.assertTrue(review([j])["judgments"][0]["eligible_for_editorial_consideration"])
        task["targets"][0]["primary_evidence"] = False
        self.assertFalse(review([j])["judgments"][0]["eligible_for_editorial_consideration"])
        task["targets"][0]["primary_evidence"] = True
        for rows in [[j, j], [{**j, "assertion_id": "other"}], []]:
            self.assertFalse(review(rows)["aligned"])
        task["targets"][0]["mechanically_ready"] = False
        self.assertFalse(review([j])["judgments"][0]["eligible_for_editorial_consideration"])
        with self.assertRaises(ValueError):
            Judgment.model_validate({**j, "native_time": "uncertain"})
        self.assertFalse(
            review([{**j, "evidence_quote": "This quotation does not occur in the source."}])[
                "judgments"
            ][0]["eligible_for_editorial_consideration"]
        )


if __name__ == "__main__":
    unittest.main()
