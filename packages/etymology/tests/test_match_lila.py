"""Offline checks of the LiLa matcher and relays, on a few made-up records of each source."""

import json
import tempfile
import unittest
from pathlib import Path

from etymology import assertions as A
from etymology import match_lila as M
from etymology.orthography import Profile
from test_assertions import EXTRACTION

WIKT = "wiktionary@2026-09-01"


def record(source, key, form, code, label):
    return {
        "type": "record",
        "id": f"{source.split('@')[0]}:{key}",
        "source": source,
        "language": {"label": label, "code": code, "relation": "exact"},
        "form": form,
        "reconstructed": form.startswith("*") or code.endswith("-pro"),
    }


def lila_objects():
    """lupus < *lukʷo‑ < *ulkʷo‑, and serō (no etymology), as lila-2 writes them."""
    src = {"type": "source", "id": "lila-edl@v1.0", "title": "LiLa", "licence": "x"}
    src.update(may_display="x", may_export="x", profile="x")
    agent = {"type": "agent", "id": "agent:de-vaan", "kind": "person", "name": "de Vaan"}
    lupus = record("lila-edl@v1.0", "la1", "lupus", "la", "Latin")
    pit = record("lila-edl@v1.0", "etymon/pit1", "*lukʷo‑", "itc-pro", "Proto-Italic")
    pie = record("lila-edl@v1.0", "etymon/pie1", "*ulkʷo‑", "ine-pro", "Proto-Indo-European")
    stem = record("lila-edl@v1.0", "etymon/pie2", "*streu‑e/o‑", "ine-pro", "Proto-Indo-European")
    sero = record("lila-edl@v1.0", "la2", "serō", "la", "Latin")
    h = A.hypothesis(
        lupus["id"],
        [
            {
                "child": lupus["id"],
                "parents": [pit["id"]],
                "relation": "derivation",
                "mode": "inherited",
            },
            {
                "child": pit["id"],
                "parents": [pie["id"]],
                "relation": "derivation",
                "mode": "inherited",
            },
        ],
    )
    a = A.assertion(
        src["id"],
        "etymology/1",
        h["id"],
        "endorses",
        asserter=agent["id"],
        holder=agent["id"],
        extraction=EXTRACTION["id"],
    )
    return [src, agent, EXTRACTION, lupus, pit, pie, stem, sero, h, a]


def wiktionary_lines():
    """lupus < *lukʷos < *wĺ̥kʷos, crediting de Vaan with a page, beside a homograph lupus
    of no stated origin; two serō; the root
    *strew-; an Umbrian word crediting de Vaan; another lukʷos-like imputed record."""
    src = {"type": "source", "id": WIKT, "title": "Wiktionary", "licence": "x"}
    src.update(may_display="x", may_export="x", profile="x")
    editors = {"type": "agent", "id": "agent:wiktionary-editors", "kind": "editors", "name": "e"}
    edl = {"type": "work", "id": M.EDL, "citation": "de Vaan 2008"}
    other = {"type": "work", "id": "work:wiktionary:R:la:L&S", "citation": "L&S"}
    rec = {
        k: record(WIKT, k, form, code, code)
        for k, form, code in (
            ("la:lupus#1", "lupus", "la"),
            ("la:lupus#2", "lupus", "la"),
            ("itc-pro:lukʷos#1", "lukʷos", "itc-pro"),
            ("ine-pro:wĺ̥kʷos#1", "wĺ̥kʷos", "ine-pro"),
            ("la:serō#1", "serō", "la"),
            ("la:serō#2", "serō", "la"),
            ("ine-pro:strew-#1", "strew-", "ine-pro"),
            ("xum:tota#1", "tota", "xum"),
            ("itc-pro:touta#1", "toutā", "itc-pro"),
        )
    }
    links = [
        ("la:lupus#1", "itc-pro:lukʷos#1"),
        ("itc-pro:lukʷos#1", "ine-pro:wĺ̥kʷos#1"),
        ("xum:tota#1", "itc-pro:touta#1"),
    ]
    hyps = [
        A.hypothesis(
            rec[c]["id"],
            [
                {
                    "child": rec[c]["id"],
                    "parents": [rec[p]["id"]],
                    "relation": "derivation",
                    "mode": "inherited",
                }
            ],
        )
        for c, p in links
    ]
    asserts = [
        A.assertion(
            WIKT,
            f"{h['word']}/Etymology",
            h["id"],
            "endorses",
            asserter=editors["id"],
            holder=editors["id"],
            extraction=EXTRACTION["id"],
            credits=[
                {"work": other["id"], "role": "follows", "scope": "entry"},
                {"work": M.EDL, "role": "follows", "scope": "entry", "locator": "357"},
            ],
        )
        for h in hyps
        if h["word"] != rec["itc-pro:lukʷos#1"]["id"]
    ]
    objs = [src, editors, EXTRACTION, edl, other, *rec.values(), *hyps, *asserts]
    return [json.dumps(o, ensure_ascii=False, separators=(",", ":")) + "\n" for o in objs]


class Keys(unittest.TestCase):
    def test_compare_keys(self):
        self.assertEqual(M.compare_keys(({"luku"}, {"luku"}), ({"luku"}, {"luku"})), "equal")
        self.assertEqual(
            M.compare_keys(({"lukuo"}, {"lukuo", "luku"}), ({"lukuos"}, {"lukuos", "lukuo"})),
            "near",
        )
        self.assertEqual(
            M.compare_keys(({"uitio"}, {"uitio", "uiti"}), ({"uitis"}, {"uitis", "uiti"})), "cut"
        )
        self.assertIsNone(M.compare_keys(({"kieie"}, {"kieie", "ki"}), ({"ki"}, {"ki"})))

    def test_length_and_palatals(self):
        lila = M.variants(Profile.read(M.LILA / "etc/orthography.tsv"))
        wikt = M.variants(Profile.read(M.WIKTIONARY_PROFILE))
        m = M.Matcher.__new__(M.Matcher)
        m.profiles = {"lila": lila, "wikt": wikt}

        def compare(a, b):
            return m.proto_keys(a, "lila").compare(m.proto_keys(b, "wikt"))

        self.assertEqual(compare("*wēlo‑", "welō"), "loose")
        self.assertEqual(compare("*frāter‑", "frātēr"), "loose")
        self.assertEqual(compare("*spondā‑", "sponda"), "equal")
        self.assertEqual(compare("*ǵlh₂‑i‑", "gl̥Hyós"), "loose")
        self.assertEqual(compare("*h₂eḱ‑os", "h₂éḱos"), "equal")
        self.assertEqual(compare("*tepē‑", "tepēō"), "near")
        self.assertEqual(compare("*plāng‑e/o‑", "plāngō"), "equal")

    def test_shapes(self):
        self.assertEqual(M.shape("*streu‑e/o‑", True), "stem")
        self.assertEqual(M.shape("*lukʷo‑", True), "stem")
        self.assertEqual(M.shape("*gustu‑", True), "bare")
        self.assertEqual(M.shape("*‑ne", True), "affix")
        self.assertEqual(M.shape("strew-", False), "root")
        self.assertEqual(M.shape("porso-", False), "word")
        self.assertEqual(M.shape("-kelos", False), "affix")

    def test_latin_readings(self):
        self.assertEqual(M.latin_keys("ve/ortō", M.LATIN_NEAR), {"verto", "vorto"})
        self.assertIn("laevir", M.latin_keys("lēvir/laevir", M.LATIN_NEAR))
        self.assertNotIn("m", M.latin_keys("vēnus/m", M.LATIN_NEAR))


class Match(unittest.TestCase):
    def setUp(self):
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "export.jsonl"
            path.write_text("".join(wiktionary_lines()))
            self.wikt = M.Wiktionary(path)
        self.lila = M.Lila(lila_objects())
        profiles = (M.LILA / "etc/orthography.tsv", M.WIKTIONARY_PROFILE)
        self.matcher = M.Matcher(self.lila, self.wikt, *map(Profile.read, profiles))
        self.results = self.matcher.run()

    def test_scan_keeps_what_credits_de_vaan(self):
        self.assertEqual(len(self.wikt.crediting), 2)
        self.assertIn("wiktionary:xum:tota#1", self.wikt.records)

    def test_latin_and_etymons(self):
        r = self.results
        self.assertEqual(r["lila-edl:la1"]["found"], ["wiktionary:la:lupus#1"])
        self.assertEqual(r["lila-edl:etymon/pit1"]["found"], ["wiktionary:itc-pro:lukʷos#1"])
        self.assertEqual(
            r["lila-edl:etymon/pit1"]["basis"],
            {
                "key": "near",
                "reflexes": {"shared": 1, "cited": 1},
                "ancestors": {"shared": 1, "cited": 1},
            },
        )
        self.assertEqual(r["lila-edl:etymon/pie1"]["found"], ["wiktionary:ine-pro:wĺ̥kʷos#1"])

    def test_ancestry_decides_a_homograph(self):
        r = self.results["lila-edl:la1"]
        self.assertEqual(r["basis"], {"key": "equal", "ancestors": {"shared": 2, "cited": 2}})

    def test_homographs_stay_candidates(self):
        m = next(o for o in self.matcher.objects(EXTRACTION["id"]) if o["record"] == "lila-edl:la2")
        self.assertNotIn("target", m)
        self.assertEqual(m["candidates"], ["wiktionary:la:serō#1", "wiktionary:la:serō#2"])

    def test_a_stem_does_not_match_a_root(self):
        self.assertIsNone(self.results["lila-edl:etymon/pie2"])

    def test_relays_and_output_validate(self):
        relayer = M.Relayer(self.matcher)
        relays = relayer.run()
        self.assertEqual(len(relays), 1)
        self.assertEqual(relays[0]["differences"], ["locator"])
        outcomes = {r["code"]: r["outcome"] for r in relayer.rows}
        self.assertTrue(outcomes["xum"].startswith("no LiLa counterpart"))
        rule = {"type": "agent", "id": M.RULE, "kind": "rule", "name": "matcher"}
        made = [rule, *self.matcher.objects(EXTRACTION["id"]), *relays]
        pool = {o["id"]: o for o in [*self.lila.objs.values(), *self.wikt.objects(), *made]}
        objs = list(M.closure(made, pool).values())
        self.assertEqual(A.validate(objs), [])


if __name__ == "__main__":
    unittest.main()
