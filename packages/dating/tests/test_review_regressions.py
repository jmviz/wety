"""Regressions for the linguistic critique and second implementation audit."""

import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from harness.evidence import EvidenceClaim, normalize_time
from harness.registry import validate_bundle, default_scope_error
from harness.tree_ingest import summarize
from harness.research import validate_graph
from harness.coverage import source_state
from harness.batch import request_matches
from harness.io_utils import digest
from harness.checks import check_analysis_edges
from harness.config import DATING_DIR, DATA_DIR


def bundle(claims):
    return dict(claims=claims, sources={"s": {}}, records={}, view={}, relations=[])


def claim(**updates):
    raw = dict(
        id="x",
        subject=dict(id="source:x", type="documentary_sample", name="a corpus", mapping="exact"),
        event="documentary_sampling",
        role="observation",
        method="text dating",
        time=normalize_time(dict(form="point", unit="BCE", value=500)).model_dump(),
        source_id="s",
        study_id="study:s",
    )
    raw.update(updates)
    return EvidenceClaim.model_validate(raw).model_dump()


def supported(c):
    c["status"] = "supported"
    c["assessment"].update(
        textual="exact_span", semantic="checked", mapping="checked", reviewer="r", rationale="r"
    )
    c["spans"] = [dict(text_sha256="a" * 64, start=0, end=3, quote="abc")]
    return c


class IntegrityRegressions(unittest.TestCase):
    def test_normalization_cannot_be_self_certified(self):
        c = claim()
        c["time"]["value"] = 1200
        self.assertTrue(any("normalization mismatch" in e for e in validate_bundle(bundle([c]))))

    def test_program_verified_without_native_values_rejected(self):
        c = claim()
        c["time"]["original"] = {}
        self.assertTrue(any("unreproducible temporal" in e for e in validate_bundle(bundle([c]))))

    def test_unknown_relative_time_needs_no_fake_epoch(self):
        t = normalize_time({"form": "unknown"})
        self.assertEqual(t.form, "unknown")
        self.assertIsNone(t.value)

    def test_unknown_cannot_hide_numbers(self):
        with self.assertRaises(ValueError):
            normalize_time({"form": "unknown", "value": 4000})

    def test_reported_range_does_not_invent_probability(self):
        t = normalize_time(
            dict(
                form="distribution",
                mean=5900,
                lower=4200,
                upper=7800,
                unit="BP",
                present=1950,
                interval_type="reported_range",
            )
        )
        self.assertIsNone(t.probability)

    def test_single_native_bound_reverses_calendar_direction(self):
        t = normalize_time(dict(form="duration", upper=4000, unit="BCE", ongoing=True))
        self.assertEqual(t.lower, -3999)
        self.assertIsNone(t.upper)

    def test_corrupt_cached_text_not_used_for_extraction(self):
        from harness import retrieve

        with tempfile.TemporaryDirectory() as d, patch.object(retrieve, "TEXTS", d):
            (Path(d) / ("a" * 64 + ".txt")).write_text("wrong document")
            c = {"retrieved": {"sha256": "a" * 64}, "abstract": "abstract only"}
            self.assertIsNone(retrieve.cached_text(c))
            self.assertEqual(retrieve.text_for(c), "abstract only")

    def test_distinct_documentary_events(self):
        for event in [
            "composition",
            "manuscript_production",
            "last_documentation",
            "last_fluent_speaker_report",
            "regional_language_shift",
            "literary_use",
        ]:
            self.assertEqual(claim(event=event)["event"], event)

    def test_population_cannot_be_wiktionary_stage(self):
        c = supported(claim(event="population_event"))
        c["subject"].update(id="wikt:xx", type="population")
        with self.assertRaises(ValueError):
            EvidenceClaim.model_validate(c)

    def test_relative_support_requires_structured_relation(self):
        c = supported(claim(event="relative_constraint", time={"form": "unknown"}))
        self.assertTrue(any("lacks structured relation" in e for e in validate_bundle(bundle([c]))))

    def test_supported_span_must_belong_to_source(self):
        self.assertTrue(
            any(
                "not bound to registered source" in e
                for e in validate_bundle(bundle([supported(claim())]))
            )
        )

    def test_marginal_distributions_are_not_hard_order_bounds(self):
        a = claim(
            analysis_id="a",
            time=normalize_time(
                dict(
                    form="distribution",
                    unit="CE",
                    lower=1000,
                    upper=1500,
                    interval_type="hpd",
                    probability=0.95,
                )
            ).model_dump(),
        )
        b = claim(
            analysis_id="a",
            time=normalize_time(
                dict(
                    form="distribution",
                    unit="CE",
                    lower=1,
                    upper=500,
                    interval_type="hpd",
                    probability=0.95,
                )
            ).model_dump(),
        )
        self.assertEqual(check_analysis_edges([("a", "b")], {"a": a, "b": b}), [])

    def test_prior_and_demonstration_not_historical_defaults(self):
        self.assertIsNotNone(default_scope_error(claim(role="calibration_prior"), {}))
        graph = {"nodes": [{"id": "analysis:a", "scope": "methodological_demonstration"}]}
        self.assertIsNotNone(default_scope_error(claim(analysis_id="a"), graph))

    def test_research_dependency_cycle(self):
        g = {
            "nodes": [dict(id=x, kind="analysis", label=x) for x in ["a", "b"]],
            "relations": [
                dict(
                    source=a,
                    target=b,
                    relation="uses_prior_from",
                    basis="test",
                    references=["source"],
                )
                for a, b in [("a", "b"), ("b", "a")]
            ],
        }
        self.assertTrue(any("cycle" in e for e in validate_graph(g)))

    def test_critique_is_not_dependency_cycle(self):
        g = {
            "nodes": [dict(id=x, kind="study", label=x) for x in ["a", "b"]],
            "relations": [
                dict(
                    source="a",
                    target="b",
                    relation="critiques",
                    basis="test",
                    references=["source"],
                ),
                dict(
                    source="b",
                    target="a",
                    relation="replies_to",
                    basis="test",
                    references=["source"],
                ),
            ],
        }
        self.assertEqual(validate_graph(g), [])

    def test_wrong_dependency_target(self):
        g = {
            "nodes": [dict(id=x, kind="study", label=x) for x in ["a", "b"]],
            "relations": [
                dict(
                    source="a",
                    target="b",
                    relation="uses_dataset",
                    basis="test",
                    references=["source"],
                )
            ],
        }
        self.assertTrue(validate_graph(g))

    def test_batch_cannot_relabel_old_prompt(self):
        b = {"request_digests": {"x": digest({"system": "old", "user": "text"})}}
        self.assertTrue(request_matches(b, "x", "old", "text"))
        self.assertFalse(request_matches(b, "x", "new", "text"))
        self.assertFalse(request_matches({}, "x", "old", "text"))

    def test_same_run_reextracts_changed_prompt_or_text(self):
        from harness import llm

        c = {
            "screen": {"decision": "include"},
            "source_id": "s",
            "extracted": {"run": "r", "prompt_sha256": digest({"system": "old", "user": "text"})},
        }
        with (
            patch.object(llm, "text_for", return_value="text"),
            patch.object(llm, "extract_system", return_value="new"),
            patch.object(llm, "extract_user", return_value="text"),
        ):
            self.assertEqual(len(llm._extraction_todo([c], "r", "family")), 1)

    def test_relative_claim_and_relations_import_together(self):
        from harness import review, registry

        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            text = "Event A precedes event B."
            sha = hashlib.sha256(text.encode()).hexdigest()
            (root / "texts").mkdir()
            (root / "texts" / (sha + ".txt")).write_text(text)
            a = claim(id="a")
            b = claim(id="b")
            r = supported(
                claim(id="relative", event="relative_constraint", time={"form": "unknown"})
            )
            r["spans"] = [dict(text_sha256=sha, start=0, end=len(text), quote=text)]
            relation = dict(
                before_event="a",
                after_event="b",
                relation="precedes",
                basis_claim_ids=["relative"],
                rationale="fixture quotation",
            )
            p = root / "input.json"
            p.write_text(json.dumps({"claims": [a, b, r], "relations": [relation]}))
            initial = bundle([])
            initial["sources"]["s"]["sha256"] = sha
            with (
                patch.object(review, "DATA_DIR", d),
                patch.object(registry, "CACHE_DIR", d),
                patch.object(review, "build_bundle", return_value=initial),
            ):
                self.assertEqual(review.import_evidence(p)["imported"], 3)
                self.assertEqual(
                    json.loads((root / "_constraints/relations.json").read_text()), [relation]
                )

    def test_zero_abstract_claims_not_complete(self):
        s = source_state({"abstract": "A summary", "extracted": {"claims": 0}})
        self.assertEqual(s["retrieval"], "abstract_only")
        self.assertEqual(s["extraction"], "legacy_input_unverified")
        self.assertFalse(s["zero_claims_is_evidence_of_absence"])


class TreeRegressions(unittest.TestCase):
    def manifest(self):
        return json.loads((Path(DATA_DIR) / "_analyses/sagart2019_relaxed_fbd.json").read_text())

    def test_summary_topology_unknown(self):
        c = summarize(self.manifest())[0]
        self.assertIsNone(c["provenance"]["topology_uncertainty"])
        self.assertEqual(c["provenance"]["joint_samples"], [])

    def test_result_role_not_inferred_from_tree(self):
        m = self.manifest()
        del m["result_role"]
        with self.assertRaises(ValueError):
            summarize(m)

    def test_prior_chain_cannot_call_itself_posterior(self):
        m = self.manifest()
        m.update(sample_type="posterior", result_role="calibration_prior")
        with self.assertRaisesRegex(ValueError, "prior-only"):
            summarize(m)

    def test_changed_analysis_changes_evidence_identity(self):
        m = self.manifest()
        original = summarize(m)[0]["id"]
        m["present"] = 1950
        self.assertNotEqual(summarize(m)[0]["id"], original)

    def test_mechanical_numbers_must_reproduce(self):
        c = summarize(self.manifest())[0]
        c["time"]["original"]["mean"] = 99999
        b = bundle([c])
        b["sources"] = {c["source_id"]: {}}
        self.assertTrue(any("mechanical result differs" in e for e in validate_bundle(b)))

    def test_joint_samples_retain_tree_identity(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "trees.nex"
            trace = Path(d) / "trace.log"
            trace.write_text("fixture trace")
            p.write_text(
                "#NEXUS\nBegin trees;\n"
                + "\n".join(
                    f"tree STATE_{i} = " + ("((A:1,B:1):2,C:3);" if i % 2 else "((A:1,C:1):2,B:3);")
                    for i in range(200)
                )
                + "\nEnd;"
            )
            m = self.manifest()
            m.update(
                path=str(p),
                sha256=hashlib.sha256(p.read_bytes()).hexdigest(),
                taxa=["A", "B", "C"],
                sample_type="posterior",
                tip_ages={"A": 0, "B": 0, "C": 0},
                burnin_trees=0,
                burnin_evidence="synthetic fixture",
                diagnostics={
                    "converged": True,
                    "minimum_ess": 200,
                    "trace_path": str(trace),
                    "trace_sha256": hashlib.sha256(trace.read_bytes()).hexdigest(),
                },
            )
            cs = summarize(m)
            root = next(c for c in cs if len(c["subject"]["taxa"]) == 3)
            ab = next(c for c in cs if c["subject"]["taxa"] == ["A", "B"])
            self.assertEqual(len(root["provenance"]["joint_samples"]), 200)
            self.assertEqual(ab["provenance"]["node_support"], 0.5)
            self.assertTrue(ab["provenance"]["topology_uncertainty"])
            self.assertEqual(ab["provenance"]["joint_samples"][0]["sample_index"], 1)
            self.assertEqual(ab["time"]["original"]["interval_type"], "equal_tailed")


if __name__ == "__main__":
    unittest.main()
