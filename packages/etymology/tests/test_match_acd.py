"""Offline checks of the ACD matcher and relay rule (acd-3) on a small invented source."""

import unittest

from etymology import assertions as A
from etymology.match_acd import (
    RELAY_RULE,
    RULE,
    Acd,
    Keys,
    Matcher,
    Relayer,
    Wiktionary,
    closure,
    gloss_similarity,
    loose,
    near,
)

KEYS = Keys()
ACD_SRC = "acd@v2.0"
WK_SRC = "wiktionary@2026-09-01"
# ACD language ids: 1 PMP (a level), 2 Malay, 3 Tagalog, 4 Ilokano
LANGS = {
    "1": ("Proto-Malayo-Polynesian", "poz-pro"),
    "2": ("Malay", "ms"),
    "3": ("Tagalog", "tl"),
    "4": ("Ilokano", "ilo"),
}


class Tree:
    def __init__(self):
        self.languages = {"1": {"Name": "Proto-Malayo-Polynesian"}}

    def above(self, lang):
        return [] if lang == "1" else ["1"]


def acd_record(lang, key, form, gloss):
    label, code = LANGS[lang]
    return {
        "type": "record",
        "id": f"acd:{lang}-{key}-1",
        "source": ACD_SRC,
        "language": {"label": label, "code": code, "relation": "exact"},
        "form": form,
        "reconstructed": form.startswith("*"),
        "gloss": gloss,
    }


def wk_record(code, term, n=1, imputed=False):
    return {
        "type": "record",
        "id": f"wiktionary:{code}:{term}#{n}" + ("?" if imputed else ""),
        "source": WK_SRC,
        "language": {"label": code, "code": code, "relation": "exact"},
        "form": term,
        "reconstructed": code.endswith("-pro"),
    }


def inherits(child, parent, source, asserter, **fields):
    h = A.hypothesis(
        child,
        [{"child": child, "parents": [parent], "relation": "derivation", "mode": "inherited"}],
    )
    a = A.assertion(
        source, f"{child}/Etymology", h["id"], "endorses", asserter=asserter, holder=asserter,
        extraction="extraction:0000000000000000", **fields,
    )  # fmt: skip
    return [h, a]


def acd_objects():
    r = {
        "louse": acd_record("1", "louse", "*kutu₁", "louse"),
        "break": acd_record("1", "break", "*kutu₂", "a louse-like insect"),
        "ms": acd_record("2", "louse", "kutu", "louse"),
        "tl": acd_record("3", "louse", "kúto", "louse"),
        "ilo": acd_record("4", "break", "kutú", "break, snap"),
        "hand": acd_record("3", "hand", "kamáy", "hand"),
        "duck": acd_record("3", "duck", "papan", "Philippine duck"),
        "kin": acd_record("3", "kin", "bata", "kinsman"),
        "nit": acd_record("3", "nit", "lisa", "nit"),
        "infix": acd_record("3", "infix", "-um-", "actor focus"),
        "root": acd_record("1", "root", "*wakaR", "root"),
        "root2": acd_record("1", "root2", "*akaR", "root"),
        "ms-root": acd_record("2", "root", "akar", "root"),
        "ms-root2": acd_record("2", "root2", "akar", "root"),
        "ilo-root": acd_record("4", "root", "ramut", "root"),
        "ilo-root2": acd_record("4", "root2", "ramut", "root"),
    }
    sets = [
        {"type": "cognate_set", "id": "acd:1_1", "source": ACD_SRC, "category": "cognate set",
         "reconstruction": r["louse"]["id"], "members": [r["ms"]["id"], r["tl"]["id"]]},
        {"type": "cognate_set", "id": "acd:2_1", "source": ACD_SRC, "category": "cognate set",
         "reconstruction": r["break"]["id"], "members": [r["ilo"]["id"]]},
        {"type": "cognate_set", "id": "acd:3_1", "source": ACD_SRC, "category": "cognate set",
         "reconstruction": r["root"]["id"], "members": [r["ms-root"]["id"], r["ilo-root"]["id"]]},
        {"type": "cognate_set", "id": "acd:4_1", "source": ACD_SRC, "category": "cognate set",
         "reconstruction": r["root2"]["id"], "members": [r["ms-root2"]["id"], r["ilo-root2"]["id"]]},
    ]  # fmt: skip
    sets[0]["members"] += [r["kin"]["id"], r["nit"]["id"]]
    rule = "agent:wety-acd-tree@1"
    links = []
    for child, parent in (
        ("ms", "louse"), ("tl", "louse"), ("ilo", "break"), ("kin", "louse"), ("nit", "louse"),
        ("ms-root", "root"), ("ms-root2", "root2"), ("ilo-root", "root"),
        ("ilo-root2", "root2"),
    ):  # fmt: skip
        links += inherits(r[child]["id"], r[parent]["id"], ACD_SRC, rule)
    return r, [*r.values(), *sets, *links]


def wiktionary_objects():
    r = {
        "kutu": wk_record("poz-pro", "kutu"),
        "ms": wk_record("ms", "kutu"),
        "tl": wk_record("tl", "kuto"),
        "kamay": wk_record("tl", "kamay"),
        "papan1": wk_record("tl", "papan", 1),
        "papan2": wk_record("tl", "papan", 2),
        "other": wk_record("poz-pro", "baqi"),
        "bata": wk_record("tl", "bata", imputed=True),
        "lisa1": wk_record("tl", "lisa", 1),
        "lisa2": wk_record("tl", "lisa", 2),
        "infix": wk_record("tl", "-um-"),
        "prefix": wk_record("tl", "um-"),
        "root": wk_record("poz-pro", "wakaʀ"),
        "akar": wk_record("ms", "akar"),
        "ramut": wk_record("ilo", "ramut"),
    }
    editors = "agent:wiktionary-editors"
    credit = [{"work": "work:wiktionary:R:map:ACD", "role": "follows", "scope": "entry"}]
    objs = [*r.values()]
    objs += inherits(r["ms"]["id"], r["kutu"]["id"], WK_SRC, editors)
    objs += inherits(r["tl"]["id"], r["kutu"]["id"], WK_SRC, editors, credits=credit)
    objs += inherits(r["bata"]["id"], r["other"]["id"], WK_SRC, editors)
    objs += inherits(r["lisa1"]["id"], r["kutu"]["id"], WK_SRC, editors)
    objs += inherits(r["akar"]["id"], r["root"]["id"], WK_SRC, editors)
    objs += inherits(r["ramut"]["id"], r["root"]["id"], WK_SRC, editors)
    objs.append({"type": "work", "id": "work:wiktionary:R:map:ACD", "citation": "{{R:map:ACD}}"})
    glosses = [
        {"code": "poz-pro", "word": "kutu", "canonical": None, "ety": "", "glosses": ["louse"]},
        {"code": "tl", "word": "kamay", "canonical": None, "ety": "", "glosses": ["hand"]},
    ]
    return r, objs, glosses


def setup():
    ar, aobjs = acd_objects()
    wr, wobjs, glosses = wiktionary_objects()
    acd = Acd(aobjs, tree=Tree())
    wikt = Wiktionary(wobjs, glosses)
    matcher = Matcher(acd, wikt, KEYS, language_map=None, exact={c for _, c in LANGS.values()})
    return ar, wr, acd, wikt, matcher


class Keys_(unittest.TestCase):
    def test_reconstructions(self):
        self.assertEqual(KEYS("acd", True, "*qaRem₂"), KEYS("wiktionary", True, "qaRəm"))
        self.assertEqual(KEYS("acd", True, "*a(m)bek"), {"ambek", "abek"})
        self.assertIn("aRi", KEYS("wiktionary", True, "(um-)aʀi"))

    def test_attested(self):
        self.assertEqual(KEYS("acd", False, "taŋgól"), KEYS("wiktionary", False, "tanggol"))
        self.assertEqual(KEYS("acd", False, "iʔa"), KEYS("wiktionary", False, "iʻa"))
        a, w = KEYS("acd", False, "bátaʔ"), KEYS("wiktionary", False, "bata")
        self.assertFalse(a & w)
        self.assertEqual(loose(a), w)
        self.assertEqual(KEYS("acd", False, "labáha ~ nabáhas"), {"labaha", "nabahas"})

    def test_near_and_gloss(self):
        self.assertTrue(near("katiR", "kati"))
        self.assertFalse(near("aku", "ako"))
        self.assertEqual(gloss_similarity("louse", "head louse"), 1.0)
        self.assertEqual(gloss_similarity("hand", "foot"), 0)
        self.assertIsNone(gloss_similarity("louse", ""))


class Matching(unittest.TestCase):
    def test_reflexes_decide_between_homographs(self):
        ar, wr, _acd, _wikt, m = setup()
        d = m.run()
        what, target, basis = d[ar["louse"]["id"]]
        self.assertEqual((what, target), ("match", wr["kutu"]["id"]))
        self.assertEqual(basis["reflexes"], {"shared": 3, "cited": 4})
        self.assertEqual(d[ar["break"]["id"]][:2], ("unmatched", "another ACD record fits better"))

    def test_an_equal_key_beats_a_near_one(self):
        ar, wr, _acd, _wikt, m = setup()
        d = m.run()
        self.assertEqual(d[ar["root"]["id"]][:2], ("match", wr["root"]["id"]))
        self.assertEqual(d[ar["root2"]["id"]][:2], ("unmatched", "another ACD record fits better"))

    def test_affix_shape(self):
        ar, wr, _acd, _wikt, m = setup()
        self.assertEqual(m.run()[ar["infix"]["id"]][:2], ("match", wr["infix"]["id"]))

    def test_ancestors(self):
        ar, wr, _acd, _wikt, m = setup()
        d = m.run()
        # Wiktionary derives its bata from another PMP word than the ACD's *kutu₁
        self.assertEqual(
            d[ar["kin"]["id"]][:2], ("unmatched", "candidates whose ancestry contradicts the ACD")
        )
        what, target, basis = d[ar["nit"]["id"]]
        self.assertEqual((what, target), ("match", wr["lisa1"]["id"]))
        self.assertEqual(basis["ancestors"], {"shared": 1, "cited": 1})

    def test_attested(self):
        ar, wr, _acd, _wikt, m = setup()
        d = m.run()
        self.assertEqual(d[ar["hand"]["id"]][:2], ("match", wr["kamay"]["id"]))
        self.assertEqual(d[ar["tl"]["id"]][:2], ("match", wr["tl"]["id"]))
        what, cands, _ = d[ar["duck"]["id"]]
        self.assertEqual(what, "ambiguous")
        self.assertEqual(sorted(cands), sorted([wr["papan1"]["id"], wr["papan2"]["id"]]))

    def test_relay_agrees_and_validates(self):
        _ar, _wr, acd, wikt, m = setup()
        d = m.run()
        relays, _outcomes = Relayer(acd, wikt, m, d).run()
        self.assertEqual(len(relays), 1)
        self.assertTrue(relays[0]["agrees"])
        self.assertEqual(relays[0]["holder"], RELAY_RULE)
        matches = [
            A.match(rid, RULE, 1.0, b, target=t)
            for rid, (what, t, b) in d.items()
            if what == "match"
        ]
        agents = {
            i: {"type": "agent", "id": i, "kind": k, "name": i}
            for i, k in (
                (RULE, "rule"), (RELAY_RULE, "rule"), ("agent:wety-acd-tree@1", "rule"),
                ("agent:wiktionary-editors", "editors"),
            )
        }  # fmt: skip
        extraction = {
            "type": "extraction", "id": "extraction:0000000000000000", "method": "converter",
            "tool": "test", "revision": "0" * 40, "dirty": False,
            "inputs": [{"path": "x", "sha256": "0" * 64}],
        }  # fmt: skip
        sources = {
            s: {"type": "source", "id": s, "title": s, "licence": "x", "may_display": "x", "may_export": "x",
                "profile": "x"}
            for s in (ACD_SRC, WK_SRC)
        }  # fmt: skip
        objs = list(
            closure(matches + relays, agents, sources, {extraction["id"]: extraction},
                    acd.objects, wikt.objects).objects.values()
        )  # fmt: skip
        errors = [e for e in A.validate(objs) if "digest" not in e]
        self.assertEqual(errors, [])

    def test_relay_differs_in_parents(self):
        ar, wr, acd, wikt, m = setup()
        d = m.run()
        # Wiktionary derives Tagalog kuto from PMP *kutu; say the ACD's PMP *kutu₁ were
        # matched elsewhere: Wiktionary's parent is then another word
        d[ar["louse"]["id"]] = ("match", "wiktionary:poz-pro:kutuq#1", d[ar["louse"]["id"]][2])
        d["acd:1-other-1"] = ("match", wr["kutu"]["id"], {"key": "equal"})
        acd.records["acd:1-other-1"] = acd_record("1", "other", "*kutu", "other")
        relays, _ = Relayer(acd, wikt, m, d).run()
        self.assertEqual(relays[0]["differences"], ["parents"])


if __name__ == "__main__":
    unittest.main()
