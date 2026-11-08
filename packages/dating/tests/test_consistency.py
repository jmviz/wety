import unittest

from harness.consistency import frame_errors, ordering_errors
from harness.periods import export_periods
from test_periods import bundle, claim

LANGS = {
    "ine-pro": {"kind": "reconstructed"},
    "gem-pro": {"kind": "reconstructed", "ancestors": ["ine-pro"]},
    "cel-pro": {"kind": "reconstructed", "ancestors": ["ine-pro"]},
    "got": {"kind": "regular", "ancestors": ["gem-pro"]},
}


def crown(cid, code, value, source="src_bc", study="study:a"):
    c = claim(
        cid,
        code,
        "crown_divergence",
        "hypothesis",
        {"form": "point", "unit": "BCE", "value": value, "precision": "year"},
        source,
    )
    return {**c, "study_id": study}


def period(cid, code, lo, hi, study):
    raw = {"form": "duration", "unit": "CE", "lower": lo, "upper": hi, "precision": "year"}
    c = claim(cid, code, "conventional_period", "hypothesis", raw)
    return {**c, "study_id": study}


def framed(claims, adopted, frame=None):
    b = bundle(claims, adopted, {c: "reconstructed" for c in ("ine-pro", "gem-pro", "cel-pro")})
    for code, v in b["view"].items():
        v["name"] = "Proto-" + code  # only a proto-language may end at a crown age
    b["frames"] = {"ine-pro": frame} if frame else {}
    return b


FRAME = {"root": "ine-pro", "name": "Study A", "study_ids": ["study:a"]}


class ConsistencyTests(unittest.TestCase):
    def test_a_proto_language_may_not_end_after_its_descendant(self):
        claims = [crown("p", "ine-pro", 500), crown("g", "gem-pro", 2000)]
        langs = export_periods(framed(claims, ["p", "g"]), "gen")["languages"]
        (error,) = ordering_errors(langs, LANGS)
        self.assertIn("ine-pro", error)
        self.assertIn("gem-pro ends", error)

    def test_the_frame_estimate_is_shown_and_the_other_stands_beside_it(self):
        claims = [
            crown("p", "ine-pro", 4000),
            crown("g", "gem-pro", 500),
            period("k", "gem-pro", 100, 300, "study:b"),
        ]
        b = framed(claims, ["p", "g", "k"], FRAME)
        out = {r["code"]: r for r in export_periods(b, "gen", langs=LANGS)["languages"]}
        gem = out["gem-pro"]
        self.assertEqual([e["claim_id"] for e in gem["evidence"]], ["g"])
        self.assertIn("k", [a["claim_id"] for a in gem["alternatives"]])
        self.assertEqual(gem["frame"]["in_frame"], True)

    def test_a_framed_code_dated_elsewhere_is_an_error_unless_excepted(self):
        claims = [crown("p", "ine-pro", 4000), period("k", "cel-pro", 100, 300, "study:b")]
        alt = crown("c", "cel-pro", 1200)
        b = framed(claims + [alt], ["p", "k"], FRAME)
        # The frame's claim is offered for the code (here as a proto alternative).
        for d in b["decisions"]:
            if d["claim_id"] == "k":
                d["proto_alternatives"] = [{"claim_id": "c", "proto_mapping": "m"}]
        langs = export_periods(b, "gen", langs=LANGS)["languages"]
        self.assertEqual(len(frame_errors(b, langs, LANGS)), 1)
        cel = {r["code"]: r for r in langs}["cel-pro"]
        self.assertFalse(cel["frame"]["in_frame"])
        b["frames"]["ine-pro"] = {**FRAME, "exceptions": {"cel-pro": "unsampled branches"}}
        langs = export_periods(b, "gen", langs=LANGS)["languages"]
        self.assertEqual(frame_errors(b, langs, LANGS), [])
        cel = {r["code"]: r for r in langs}["cel-pro"]
        self.assertEqual(cel["frame"]["note"], "unsampled branches")


if __name__ == "__main__":
    unittest.main()
