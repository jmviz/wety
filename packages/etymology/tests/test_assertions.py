"""Offline checks of the assertion schema, its cross-record rules and the converters."""

import copy
import unittest

from etymology import assertions as A
from etymology.convert import etymdict, wiktionary

SOURCE = {
    "type": "source",
    "id": "demo@1",
    "title": "Demo",
    "licence": "CC0-1.0",
    "may_display": "everything",
    "may_export": "everything",
    "profile": "demo.json",
}
EXTRACTION = {
    "type": "extraction",
    "method": "converter",
    "tool": "test",
    "revision": "0" * 40,
    "dirty": False,
    "inputs": [{"path": "demo.csv", "sha256": "0" * 64}],
}
EXTRACTION["id"] = "extraction:" + A.digest({k: v for k, v in EXTRACTION.items() if k != "type"})


def record(key, form, lang="la"):
    return {
        "type": "record",
        "id": f"demo:{key}",
        "source": "demo@1",
        "language": {"label": lang, "code": lang, "relation": "exact"},
        "form": form,
        "reconstructed": bool(form) and form.startswith("*"),
    }


def objects():
    """lupus < *lukʷos < *wĺ̥kʷos, endorsed and hedged; and an alternative set."""
    lupus, lukos, wlkos = (
        record("lupus", "lupus"),
        record("lukos", "*lukʷos"),
        record("wlkos", "*wĺ̥kʷos"),
    )
    marten = record("lup", "*lup-")
    h = A.hypothesis(
        lupus["id"],
        [
            {
                "child": lupus["id"],
                "parents": [lukos["id"]],
                "relation": "derivation",
                "mode": "inherited",
            },
            {
                "child": lukos["id"],
                "parents": [wlkos["id"]],
                "relation": "derivation",
                "mode": "inherited",
            },
        ],
    )
    h2 = A.hypothesis(
        lupus["id"],
        [
            {
                "child": lupus["id"],
                "parents": [marten["id"]],
                "relation": "derivation",
                "mode": "derived",
            }
        ],
    )
    alt = A.alternative_set([h["id"], h2["id"]], True)
    author = {"type": "agent", "id": "agent:demo", "kind": "person", "name": "Demo"}
    common = {"asserter": author["id"], "holder": author["id"], "extraction": EXTRACTION["id"]}
    a1 = A.assertion(
        "demo@1",
        "p. 353",
        h["id"],
        "endorses",
        hedges=[{"level": "possible", "words": "conceivable", "link": 1}],
        in_sets=[alt["id"]],
        **common,
    )
    a2 = A.assertion("demo@1", "p. 353", h2["id"], "rejects", in_sets=[alt["id"]], **common)
    a3 = A.assertion("demo@1", "p. 353", alt["id"], "endorses", **common)
    return [SOURCE, author, EXTRACTION, lupus, lukos, wlkos, marten, h, h2, alt, a1, a2, a3]


class Validate(unittest.TestCase):
    def test_valid(self):
        self.assertEqual(A.validate(objects()), [])

    def test_schema_refuses_a_derivation_without_mode(self):
        objs = objects()
        del objs[7]["links"][0]["mode"]
        self.assertTrue(A.validate(objs))

    def test_schema_refuses_unknown_origin_with_parents(self):
        objs = objects()
        objs[8]["links"][0] = {
            "child": "demo:lupus",
            "parents": ["demo:lup"],
            "relation": "unknown",
        }
        self.assertTrue(A.validate(objs))

    def test_digest_must_match(self):
        objs = objects()
        objs[7]["links"][1]["mode"] = "borrowed"
        errors = A.validate(objs)
        self.assertTrue(any("digest of its word and links" in e for e in errors))

    def test_references_resolve(self):
        objs = [o for o in objects() if o.get("id") != "demo:wlkos"]
        errors = A.validate(objs)
        self.assertTrue(any("demo:wlkos is not a record" in e for e in errors))

    def test_links_chain_from_the_word(self):
        objs = objects()
        h = objs[7]
        h["links"][1]["child"] = "demo:lup"
        h["id"] = "hyp:" + A.digest([h["word"], h["links"]])
        errors = A.cross_errors(objs)
        self.assertTrue(any("not the word or an earlier parent" in e for e in errors))

    def test_hedge_on_a_missing_link(self):
        objs = objects()
        objs[10]["hedges"][0]["link"] = 5
        self.assertTrue(any("hedge on link 5" in e for e in A.validate(objs)))

    def test_in_sets_names_a_set_holding_the_target(self):
        objs = objects()
        other = A.alternative_set(["demo:x", "demo:y"], False)
        objs[10]["in_sets"] = [other["id"]]
        self.assertTrue(any(other["id"] in e for e in A.validate(objs)))

    def test_cognate_set_names_something(self):
        objs = objects()
        cs = {
            "type": "cognate_set",
            "id": "demo:set",
            "source": "demo@1",
            "reconstruction": "demo:wlkos",
            "members": [],
        }
        self.assertEqual(A.validate([*objs, cs]), [])
        cs["reconstruction"] = None
        self.assertTrue(any("neither" in e for e in A.validate([*objs, cs])))

    def test_compiled_only_by_a_rule(self):
        objs = objects()
        objs[10]["compiled"] = {"from": "tree", "words": "implied by the tree"}
        errors = A.validate(objs)
        self.assertTrue(any("compiled, but its asserter is not a rule" in e for e in errors))
        objs[1]["kind"] = "rule"
        self.assertEqual(A.validate(objs), [])
        objs[12]["compiled"] = {"from": "tree", "words": "implied by the tree"}
        errors = A.validate(objs)
        self.assertTrue(any("compiled, but its target is not a hypothesis" in e for e in errors))

    def test_extensions_are_named_by_a_source(self):
        objs = objects()
        objs[3]["x"] = {"other": {"note": 1}}
        self.assertTrue(any("names no source" in e for e in A.validate(objs)))

    def test_extensions_are_declared(self):
        objs = objects()
        objs[3]["x"] = {"demo": {"note": 1}}
        self.assertTrue(any("x.demo.note is not declared" in e for e in A.validate(objs)))

    def test_matches_and_relays(self):
        objs = objects()
        lupus, h, a1 = objs[3], objs[7], objs[10]
        wikt = record("lupus-wikt", "lupus")
        rule = {"type": "agent", "id": "agent:wety-match-demo@1", "kind": "rule", "name": "x"}
        work = {"type": "work", "id": "work:demo:edl", "citation": "De Vaan (2008)"}
        crediting = A.assertion(
            "demo@1",
            "elsewhere",
            h["id"],
            "endorses",
            asserter="agent:demo",
            holder="agent:demo",
            extraction=EXTRACTION["id"],
            credits=[{"work": work["id"], "role": "follows", "scope": "entry"}],
        )
        basis = {"key": "equal", "ancestors": {"shared": 1, "cited": 1}}
        m = A.match(lupus["id"], rule["id"], 0.9, basis, target=wikt["id"])
        unclear = A.match(
            lupus["id"], rule["id"], 0.5, {"key": "equal"}, candidates=[wikt["id"], lupus["id"]]
        )
        agrees = A.relay(crediting["id"], a1["id"], work["id"], rule["id"])
        differs = A.relay(a1["id"], crediting["id"], work["id"], rule["id"], ["hedge"])
        good = objs + [wikt, rule, work, crediting, m, unclear, agrees]
        self.assertEqual(A.validate(good), [])
        # a1 credits no work, so it relays nothing
        self.assertIn("does not credit", " ".join(A.validate(good + [differs])))
        both = dict(m, candidates=[wikt["id"], lupus["id"]])
        self.assertTrue(A.validate(objs + [wikt, rule, both]))
        silent = dict(agrees, agrees=False)
        self.assertTrue(A.validate(good[:-1] + [silent]))

    def test_a_source_names_its_uniform_fields(self):
        objs = objects()
        objs[0] = dict(SOURCE, uniform=["mode"])
        self.assertEqual(A.validate(objs), [])
        objs[0] = dict(SOURCE, uniform=["gloss"])
        self.assertTrue(A.validate(objs))

    def test_writer_refuses_conflicting_objects(self):
        w = A.Writer()
        r = record("lupus", "lupus")
        w.add(r)
        w.add(copy.deepcopy(r))
        r2 = dict(r, form="lupa")
        with self.assertRaises(ValueError):
            w.add(r2)


class Registry(unittest.TestCase):
    """The extension registry: a concept two sources need belongs in the core."""

    def test_no_concept_is_declared_by_two_sources(self):
        sources = {}
        for (source, _, _), row in A.extensions().items():
            sources.setdefault(row["concept"], set()).add(source)
        shared = {c: sorted(s) for c, s in sources.items() if len(s) > 1}
        self.assertEqual(shared, {}, "promote these concepts into the core schema")

    def test_rows_are_complete(self):
        for key, row in A.extensions().items():
            self.assertIn(row["kind"], ("design", "source-native"), key)
            self.assertTrue(row["means"], key)


class AcdTree(unittest.TestCase):
    """Placing languages on the ACD's tree of reconstruction levels."""

    NEWICK = "(20004,((19083)19094,(19076,(20002,19082)20001)19074)19081)19072"
    ABBR = {  # noqa: RUF012
        "19072": "PAN",
        "20004": "Form.",
        "19081": "PMP",
        "19094": "PWMP",
        "19083": "PPH",
        "19074": "PCEMP",
        "19076": "PCMP",
        "20001": "PEMP",
        "20002": "PSHWNG",
        "19082": "POC",
    }

    def tree(self):
        languages = {n: {"Abbr": a, "Group": a} for n, a in self.ABBR.items()}
        languages.update(
            {
                "tagalog": {"Group": "PPH"},
                "paiwan": {"Group": "Form."},
                "proto-polynesian": {"Group": "POC"},
            }
        )
        levels = set(self.ABBR) - {"20004"}
        parents, root = etymdict.newick(self.NEWICK)
        node_of_group = {a: n for n, a in self.ABBR.items()}
        place = {k: node_of_group[v["Group"]] for k, v in languages.items() if k not in self.ABBR}
        return etymdict.Tree(parents, root, levels, place)

    def test_newick(self):
        parents, root = etymdict.newick(self.NEWICK)
        self.assertEqual(root, "19072")
        self.assertEqual(parents["19083"], "19094")
        self.assertEqual(parents["20004"], "19072")
        self.assertEqual(len(parents), 9)

    def test_levels_above(self):
        t = self.tree()
        self.assertEqual(t.above("tagalog"), ["19083", "19094", "19081", "19072"])
        self.assertEqual(t.above("19083"), ["19094", "19081", "19072"])
        self.assertEqual(t.above("paiwan"), ["19072"], "Formosan is a node, not a level")
        self.assertEqual(t.above("proto-polynesian")[0], "19082")

    def test_lowest_above(self):
        t = self.tree()
        self.assertEqual(t.lowest_above(["tagalog", "proto-polynesian"]), "19081")
        self.assertEqual(t.lowest_above(["tagalog", "tagalog"]), "19083")
        self.assertEqual(t.lowest_above(["tagalog", "19072"]), None)


class WiktionaryClaims(unittest.TestCase):
    """A claims file converts to assertions that validate."""

    def convert(self, entry, sample=None):
        claims = {
            "model": "test",
            "agents": {"de-vaan": {"kind": "person", "name": "Michiel de Vaan"}},
            "works": {},
            "entries": [entry],
        }
        c = wiktionary.Converter(claims, EXTRACTION)
        c.src = dict(SOURCE, id="wiktionary@1")
        c.out.objects = {o["id"]: o for o in (c.src, EXTRACTION)}
        c.out.add({"type": "agent", "id": wiktionary.EDITORS, "kind": "editors", "name": "editors"})
        c.entry(entry, sample or {"references_wikitext": []})
        return list(c.out.objects.values())

    def test_alternatives_with_entry_credits(self):
        entry = {
            "page": "dokotera",
            "entry": "mg:dokotera",
            "claims": [
                {"key": "a", "links": [{"parents": ["fr:docteur"], "mode": "borrowed"}]},
                {
                    "key": "b",
                    "stance": "reports",
                    "holder": "de-vaan",
                    "links": [{"parents": ["en:-"], "mode": "borrowed"}],
                },
            ],
            "sets": [{"members": ["a", "b"], "exclusive": True}],
        }
        refs = {"references_wikitext": ["===References===\n* {{R:mg:Demo|page=12}}"]}
        objs = self.convert(entry, refs)
        self.assertEqual(A.validate(objs), [])
        by_stance = {
            o["stance"]: o
            for o in objs
            if o["type"] == "assertion" and o["target"].startswith("hyp:")
        }
        self.assertEqual(
            by_stance["endorses"]["credits"],
            [
                {
                    "work": "work:wiktionary:R:mg:Demo",
                    "role": "follows",
                    "scope": "entry",
                    "locator": "12",
                }
            ],
        )
        self.assertNotIn("credits", by_stance["reports"])
        self.assertEqual(by_stance["reports"]["holder"], "agent:de-vaan")
        unnamed = [o for o in objs if o["type"] == "record" and o["form"] is None]
        self.assertEqual([r["id"] for r in unnamed], ["wiktionary:en:-@mg:dokotera"])

    def test_chain_follows_the_head_parent(self):
        entry = {
            "page": "cratis",
            "entry": "la:cratis",
            "homograph": "hurdle",
            "claims": [
                {
                    "key": "a",
                    "links": [
                        {"parents": ["ine-pro:*kréh₂-tis"], "mode": "derived"},
                        {
                            "parents": ["ine-pro:*-tis", "ine-pro:*kreh₂-<id:fence>"],
                            "mode": "affix",
                            "head": 1,
                        },
                        {"parents": ["ine-pro:*ker-"], "relation": "ultimate"},
                        {"child": "entry", "parents": ["grc:κύρτος"], "relation": "influence"},
                    ],
                }
            ],
        }
        objs = self.convert(entry)
        self.assertEqual(A.validate(objs), [])
        h = next(o for o in objs if o["type"] == "hypothesis")
        self.assertEqual(h["word"], "wiktionary:la:cratis<hurdle>")
        self.assertEqual(h["links"][2]["child"], "wiktionary:ine-pro:*kreh₂-<fence>")
        self.assertEqual(h["links"][3]["child"], h["word"])


if __name__ == "__main__":
    unittest.main()
