import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from harness.evidence import (
    Temporal,
    Subject,
    EvidenceClaim,
    Assessment,
    normalize_time,
    convert_number,
    evidence_digest,
    Decision,
)
from harness.schema import TimeValue, Claim
from harness.llm import verify_quote, ExtractedTime, _time_value
from harness.bootstrap.dplace import parse_newick, parse_nexus_trees, map_tree
from harness.tree_ingest import summarize, quantile
from harness.registry import build_bundle, validate_bundle, study_identity
from harness.time_utils import parse_era, from_bp
from harness import store
from harness.config import DATING_DIR, DATA_DIR


class ModuleTests(unittest.TestCase):
    def test_all_modules_import(self):
        import importlib, pkgutil, harness

        for info in pkgutil.walk_packages(harness.__path__, harness.__name__ + "."):
            importlib.import_module(info.name)


class CalendarTests(unittest.TestCase):
    def test_bce_zero(self):
        self.assertEqual(convert_number(1, "BCE"), 0)

    def test_bce(self):
        self.assertEqual(convert_number(4000, "BCE"), -3999)

    def test_zero_rejected(self):
        with self.assertRaises(ValueError):
            convert_number(0, "BCE")

    def test_explicit_epoch(self):
        self.assertEqual(convert_number(5300, "BP", 2000), -3300)

    def test_no_default_epoch(self):
        with self.assertRaises(ValueError):
            convert_number(5300, "BP")
        with self.assertRaises(ValueError):
            from_bp(5300)

    def test_centuries(self):
        self.assertEqual(convert_number(53, "century_BP", 1950), -3350)

    def test_reverse_interval(self):
        t = normalize_time({"form": "event_range", "lower": 500, "upper": 700, "unit": "BCE"})
        self.assertEqual((t.lower, t.upper), (-699, -499))

    def test_mixed_legacy(self):
        with self.assertRaises(ValueError):
            TimeValue(value=1, earliest=1, latest=2)

    def test_hpd_needs_probability(self):
        with self.assertRaises(ValueError):
            Temporal(form="distribution", lower=0, upper=2, interval_type="hpd")

    def test_point_not_duration(self):
        with self.assertRaises(ValueError):
            Temporal(form="point", value=1, ongoing=True)

    def test_unknown_no_dates(self):
        with self.assertRaises(ValueError):
            Temporal(form="unknown", value=1)

    def test_bound_relation(self):
        with self.assertRaises(ValueError):
            Temporal(form="bound", value=1)

    def test_no_multistage_truncation(self):
        self.assertIsNone(parse_era("1500–1100 BCE; Classical Sanskrit continues today"))

    def test_qualified_not_exact(self):
        self.assertIsNone(parse_era("possibly 500 BCE"))

    def test_no_guessed_bp_parser(self):
        self.assertIsNone(parse_era("5300 BP"))

    def test_model_calendar_ignored(self):
        t = _time_value(
            ExtractedTime(
                expression="4000 BCE",
                value=-4000,
                conversion="model guess",
                source_values={"form": "point", "value": 4000, "unit": "BCE"},
            )
        )
        self.assertEqual(t.value, -3999)

    def test_missing_epoch_preserved(self):
        t = _time_value(
            ExtractedTime(
                expression="5300 BP",
                conversion="",
                source_values={"form": "point", "value": 5300, "unit": "BP"},
            )
        )
        self.assertTrue(t.unknown)


class QuoteTests(unittest.TestCase):
    def test_exact(self):
        self.assertTrue(
            verify_quote(
                "This source reports a date of 5300 BP.", "This source reports a date of 5300 BP."
            )
        )

    def test_invented_suffix(self):
        self.assertFalse(
            verify_quote(
                "This source reports a date of 5300 BP. 4000 BCE",
                "This source reports a date of 5300 BP.",
            )
        )

    def test_fragment_not_enough(self):
        self.assertFalse(
            verify_quote(
                "This source reports a date of 5300 BP. ... 99 BCE",
                "This source reports a date of 5300 BP.",
            )
        )


class TreeTests(unittest.TestCase):
    def test_annotations(self):
        n = parse_newick("('A one':1,B:1)[&height_median=1,height_95%_HPD={0.5,2},posterior=0.8];")
        self.assertEqual(n.annotations["height_95%_HPD"], [0.5, 2])
        self.assertEqual(n.tips()[0].name, "A one")

    def test_negative_branch(self):
        with self.assertRaises(ValueError):
            parse_newick("(A:-1,B:1);")

    def test_unmapped_cannot_inflate(self):
        class G:
            rows = {"F": {"level": "family"}}

            def mrca(self, t):
                return "F"

            def path(self, t):
                return ["F", t]

        rows, _ = map_tree(
            parse_newick("((A:100,B:100):900,U:1000);"), {"A": "a", "B": "b"}, G(), 1
        )
        self.assertEqual(rows[0]["age_bp"], 100)

    def test_quantile_interpolates(self):
        self.assertEqual(quantile([0, 100], 0.025), 2.5)

    def manifest(self):
        return json.loads((Path(DATA_DIR) / "_analyses/sagart2019_relaxed_fbd.json").read_text())

    def test_hash_required(self):
        m = self.manifest()
        m["sha256"] = "0" * 64
        with self.assertRaises(ValueError):
            summarize(m)

    def test_scope_required(self):
        m = self.manifest()
        m["taxa"] = m["taxa"][:-1]
        with self.assertRaises(ValueError):
            summarize(m)

    def test_no_posterior_by_extension(self):
        m = self.manifest()
        m["sample_type"] = "posterior"
        with self.assertRaises(ValueError):
            summarize(m)

    def test_missing_units(self):
        m = self.manifest()
        del m["unit"]
        with self.assertRaises(ValueError):
            summarize(m)

    def test_unannotated_clade_skipped(self):
        # TreeAnnotator leaves low-support clades without heights: no claim for them,
        # but a tree whose root has no heights is undated.
        def run(newick):
            with tempfile.TemporaryDirectory(dir=DATING_DIR) as d:
                path = Path(d) / "t.tre"
                path.write_text(newick)
                m = self.manifest()
                m.update(
                    path=str(path),
                    sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                    taxa=["A", "B", "C"],
                    unit="BP",
                    height_statistic="median",
                )
                return summarize(m)

        claims = run("((A:1,B:1)[&posterior=0.2]:1,C:2)[&height_median=200,posterior=1.0];")
        self.assertEqual([len(c["subject"]["taxa"]) for c in claims], [3])
        with self.assertRaises(ValueError):
            run("((A:1,B:1)[&height_median=100]:1,C:2)[&posterior=1.0];")

    def test_native_hpd_retained(self):
        m = self.manifest()
        root = max(summarize(m), key=lambda c: len(c["subject"]["taxa"]))
        self.assertEqual(root["time"]["original"]["interval_type"], "hpd")
        # With the labeled conventional epoch the HPD converts, and says so.
        self.assertEqual(root["time"]["form"], "distribution")
        self.assertTrue(any("AD 1950 by convention" in x for x in root["assessment"]["limitations"]))
        # Without it, an unstated epoch stays unconverted.
        m.pop("epoch_interpretation")
        m.pop("age_scale")
        root = max(summarize(m), key=lambda c: len(c["subject"]["taxa"]))
        self.assertEqual(root["time"]["form"], "unknown")
        self.assertEqual(len(root["subject"]["taxa"]), 50)


class IntegrityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.bundle = build_bundle()

    def test_whole_export_no_inherited_lifespans(self):
        self.assertGreater(len(self.bundle["view"]), 8000)
        self.assertTrue(all(v["spoken"] is None for v in self.bundle["view"].values()))

    def test_affected_codes_safe(self):
        # A chronology comes only from reviewed evidence about the code itself,
        # never from catalog or ancestor context.
        claims = {c["id"]: c for c in self.bundle["claims"]}
        for code in ["zh", "ar", "la", "sa", "urj-pro"]:
            view = self.bundle["view"].get(code)
            for entry in view["chronology"] if view else []:
                c = claims[entry["claim_id"]]
                self.assertEqual(c["subject"]["id"], "wikt:" + code)
                self.assertEqual((c["status"], c["subject"]["mapping"]), ("supported", "exact"))
                self.assertNotIn(entry["claim_id"], view["catalog_context"] + view["ancestral_context"])

    def test_bundle_valid(self):
        self.assertEqual(validate_bundle(self.bundle), [])

    def test_all_pending_in_queue(self):
        self.assertEqual(
            {c["id"] for c in self.bundle["claims"] if c["status"] != "supported"},
            {q["claim_id"] for q in self.bundle["review_queue"]},
        )

    def test_content_digest_not_only_ids(self):
        a = [{"id": "a", "time": 1}]
        b = [{"id": "a", "time": 2}]
        self.assertNotEqual(evidence_digest(a, {}), evidence_digest(b, {}))

    def test_context_digest(self):
        self.assertNotEqual(evidence_digest([], {"source": 1}), evidence_digest([], {"source": 2}))

    def test_duplicate_publication_not_independent(self):
        src = {"id": "s", "doi": "10.1/A"}
        child = {"id": "s_data_tree"}
        self.assertEqual(study_identity(src, {"s": src}), study_identity(child, {"s": src}))

    def test_no_raw_decision_override(self):
        with self.assertRaises(ValueError):
            Decision(
                subject_id="x",
                event="formation",
                claim_id="a",
                evidence_digest="b",
                reviewer="r",
                rationale="test",
                spoken_earliest=1,
            )

    def test_cycle_rejected(self):
        b = copy.deepcopy(self.bundle)
        a, c = b["claims"][:2]
        a["dependencies"] = [c["id"]]
        c["dependencies"] = [a["id"]]
        self.assertTrue(any("cycle" in e for e in validate_bundle(b)))

    def test_missing_source_rejected(self):
        b = copy.deepcopy(self.bundle)
        b["claims"][0]["source_id"] = "absent"
        self.assertTrue(any("missing source" in e for e in validate_bundle(b)))

    def test_unsupported_cannot_export(self):
        b = copy.deepcopy(self.bundle)
        b["view"]["la"]["chronology"] = [{"claim_id": b["claims"][0]["id"]}]
        self.assertTrue(any("unsafe" in e for e in validate_bundle(b)))

    def test_strict_relative_cycle(self):
        b = copy.deepcopy(self.bundle)
        a, c = [x["id"] for x in b["claims"][:2]]
        b["relations"] = [
            {
                "before_event": a,
                "after_event": c,
                "relation": "precedes",
                "basis_claim_ids": [a],
                "rationale": "test",
            },
            {
                "before_event": c,
                "after_event": a,
                "relation": "precedes",
                "basis_claim_ids": [c],
                "rationale": "test",
            },
        ]
        self.assertTrue(any("relative chronology cycle" in e for e in validate_bundle(b)))

    def test_deterministic(self):
        self.assertEqual(self.bundle["input_digest"], build_bundle()["input_digest"])


class ExtractionStoreTests(unittest.TestCase):
    def test_empty_run_supersedes_and_archives(self):
        with tempfile.TemporaryDirectory() as d:

            def family(f):
                p = Path(d) / f
                p.mkdir(exist_ok=True)
                return str(p)

            with (
                patch.object(store, "DATA_DIR", d),
                patch.object(store, "DATING_HISTORY", Path(d) / "_history"),
                patch.object(store, "family_dir", family),
            ):
                c = Claim(
                    id="clm_test1234_abc",
                    kind="expert_estimate",
                    time=TimeValue(value=1),
                    source="s",
                    extracted_by={"model": "m"},
                )
                store.replace_extraction("f", "s", [c], {"run": "first"})
                store.replace_extraction("f", "s", [], {"run": "second"})
                self.assertEqual(store.load_claims("f"), [])
                histories = list((Path(d) / "_history/extractions").glob("*.json"))
                self.assertEqual(len(histories), 2)
                self.assertTrue(any(json.loads(p.read_text())["previous"] for p in histories))


if __name__ == "__main__":
    unittest.main()
