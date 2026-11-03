"""Offline checks of what the matchers share: judgements, weighing, assignment, and the
notation miner's distance."""

import json
import random
import tempfile
import unittest
from pathlib import Path

from etymology import matching as M
from etymology import notation as N
from etymology.orthography import Profile


def judged(record, candidate, same):
    return {"record": record, "candidate": candidate, "same": same, "cause": None}


class Judgements(unittest.TestCase):
    def setUp(self):
        self.j = M.Judgements(
            [
                judged("r1", "w1", True),
                judged("r2", "w2", False),
                {"record": "r3", "among": ["w4", "w3"], "same": True},
                judged("r4", "w5", False),
                judged("r4", "w6", False),
            ]
        )

    def test_verdicts(self):
        v = self.j.verdict
        self.assertEqual(v("r1", "w1"), "right")
        self.assertEqual(v("r1", "w9"), "wrong")  # another record was judged the word
        self.assertEqual(v("r1", None), "missed")
        self.assertEqual(v("r2", "w2"), "wrong")
        self.assertIsNone(v("r2", "w3"))
        self.assertEqual(v("r3", ("w3", "w4", "w5")), "right")
        self.assertEqual(v("r3", None), "missed")
        self.assertEqual(v("r4", ("w5", "w6")), "wrong")
        self.assertIsNone(v("r9", "w1"))

    def test_score_lists_regressions(self):
        s = self.j.score({"r1": "w1", "r2": "w2", "r3": None})
        self.assertEqual(s["target precision"], 0.5)
        self.assertEqual({r["record"] for r in s["regressions"]}, {"r2", "r3"})

    def test_a_later_row_replaces_an_earlier(self):
        self.assertTrue(self.j.add(judged("r2", "w2", True)))
        self.assertFalse(self.j.add(judged("r2", "w2", True)))
        self.assertEqual(self.j.verdict("r2", "w2"), "right")

    def test_harvest_reads_samples_and_changes(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "changes.json"
            rows = [
                {"record": "a", "found": "w1", "judgement": "right"},
                {"record": "b", "found": None, "before": "w2", "judgement": "right"},
                {"record": "c", "found": None, "before": "w3", "judgement": "wrong"},
                {"record": "d", "found": ["w4", "w5"], "judgement": "right"},
                {"record": "e", "found": None, "judgement": None, "answer": "w6"},
            ]
            path.write_text(json.dumps({"reviewer": "x", "rows": rows}))
            j = M.Judgements(M.harvest(path))
        self.assertEqual(j.verdict("a", "w1"), "right")
        self.assertEqual(j.verdict("b", "w2"), "wrong")
        self.assertEqual(j.verdict("c", "w3"), "right")
        self.assertEqual(j.verdict("d", ("w4", "w5")), "right")
        self.assertEqual(j.verdict("e", "w6"), "right")


class Weighing(unittest.TestCase):
    FIELDS = {"key": ("equal", "near", "far"), "lineage": ("shared", "none shared", "none cited")}

    def test_unfitted_weights_fall_and_missing_weighs_nothing(self):
        m = M.Linkage(self.FIELDS, {"lineage": "none cited"})
        t = m.table()["weights"]
        self.assertGreater(t["key"]["equal"], t["key"]["near"])
        self.assertGreater(t["key"]["near"], t["key"]["far"])
        self.assertEqual(t["lineage"]["none cited"], 0)

    def test_em_learns_which_levels_mark_matches(self):
        rng = random.Random(1)
        pairs = []
        for _ in range(400):
            if rng.random() < 0.6:  # one word: mostly equal keys and shared lineage
                key = rng.choices(["equal", "near", "far"], [8, 2, 0.2])[0]
                lineage = rng.choices(["shared", "none shared"], [8, 2])[0]
            else:
                key = rng.choices(["equal", "near", "far"], [1, 3, 6])[0]
                lineage = rng.choices(["shared", "none shared"], [1, 9])[0]
            pairs.append({"key": key, "lineage": lineage})
        m = M.Linkage(self.FIELDS).fit(pairs)
        self.assertGreater(m.probability({"key": "equal", "lineage": "shared"}), 0.9)
        self.assertLess(m.probability({"key": "far", "lineage": "none shared"}), 0.1)
        self.assertAlmostEqual(m.prior, 0.6, delta=0.1)

    def test_decide_and_assign(self):
        self.assertEqual(M.decide([("a", 0.9), ("b", 0.3)], 0.5, 1), ["a"])
        self.assertEqual(M.decide([("a", 0.8), ("b", 0.75)], 0.5, 1), ["a", "b"])
        self.assertEqual(M.decide([("a", 0.4)], 0.5, 1), [])
        # serō 'sow' takes serō 2 on its own; serō 'join', tied, keeps serō 1
        scored = {"sow": [("w1", 0.6), ("w2", 0.95)], "join": [("w1", 0.8), ("w2", 0.8)]}
        found = M.assign([["join", "sow"]], scored, 0.5, 1)
        self.assertEqual(found, {"sow": ["w2"], "join": ["w1"]})


class Notation(unittest.TestCase):
    def test_align_and_differences(self):
        d, pairs = N.align("leKs", "leGs")
        self.assertEqual(d, 1)
        self.assertEqual(N.differences("leKs", "leGs"), [("K", "G")])
        self.assertEqual(N.differences("Hster", "ster"), [("H", N.GAP)])

    def test_mined_costs_bring_keys_closer(self):
        pairs = [({f"Ha{i}x"}, {f"a{i}x"}, f"pair {i}") for i in "bcdfg"]
        rows = N.mine(pairs)
        self.assertEqual(rows[0]["From"], "H")
        self.assertEqual(rows[0]["Cost"], N.LOW)
        plain, learnt = N.Distance(), N.Distance({("H", N.GAP): N.LOW})
        self.assertLess(learnt("Hter", "ter"), plain("Hter", "ter"))


class Conventions(unittest.TestCase):
    def test_a_profile_includes_a_convention_and_may_replace_its_rows(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "profile.tsv"
            p.write_text("Kind\tGrapheme\tKey\tNote\ninclude\tine\t\t\ngrapheme\tz\tz\t\n")
            profile = Profile.read(p)
        self.assertEqual(profile.graphemes["h₂"], "H")
        self.assertEqual(profile.graphemes["z"], "z")


if __name__ == "__main__":
    unittest.main()
