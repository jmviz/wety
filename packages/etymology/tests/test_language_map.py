"""Offline checks of a source's language map against a small catalog bundle."""

import unittest

from etymology.language_map import Catalog, build, summary


def link(source, relation="catalog_correspondence", status="catalog_link"):
    return {"link_id": "x", "source": source, "relation": relation, "status": status}


BUNDLE = {
    "index_by_target": {
        "glottolog:aaaa1234": [link("wikt:lang:aaa")],
        "glottolog:bbbb1234": [link("wikt:lang:bbb"), link("wikt:lang:bbx")],
        "glottolog:cccc1234": [
            link("wikt:lang:ccc", "source_narrower", "reviewed"),
            link("wikt:lang:ccx", "catalog_correspondence", "rejected"),
        ],
        "iso639-3:ddd": [link("wikt:lang:ddd")],
        "glottolog:pppp1234": [link("wikt:family:ppp", "family_context", "context")],
    },
    "languages": {c: {} for c in ("aaa", "bbb", "bbx", "ccc", "ddd", "ppp-pro", "qqq")},
    "glottolog": [{"glottocode": "aaad1234", "parent_language": "aaaa1234"}],
    "families": {
        "ppp": {"glottocode": "pppp1234", "proto_language": "ppp-pro"},
        "rrr": {"glottocode": "rrrr1234", "proto_language": "rrr-pro"},
    },
}


def language(id, glottocode="", iso="", proto=False, dialect_of=""):
    return {
        "ID": id,
        "Name": "L" + id,
        "Glottocode": glottocode,
        "ISO639P3code": iso,
        "Is_Proto": "true" if proto else "false",
        "Dialect_Of": dialect_of,
    }


def mapped(languages, reviewed=()):
    return {r["id"]: r for r in build(languages, Catalog(BUNDLE), reviewed)}


class LanguageMap(unittest.TestCase):
    def test_one_lookup_code_is_exact(self):
        r = mapped([language("1", "aaaa1234")])["1"]
        self.assertEqual((r["relation"], r["code"], r["basis"]), ("exact", "aaa", "glottocode"))

    def test_two_codes_stay_unresolved(self):
        r = mapped([language("1", "bbbb1234")])["1"]
        self.assertEqual((r["relation"], r["code"]), ("unresolved", None))
        self.assertEqual(r["candidates"], ["bbb", "bbx"])

    def test_reviewed_scope_and_rejected_links(self):
        # Wiktionary's ccc is narrower than the glottocode: the source's variety is broader.
        r = mapped([language("1", "cccc1234")])["1"]
        self.assertEqual((r["relation"], r["code"]), ("superset", "ccc"))

    def test_iso_after_glottocode(self):
        r = mapped([language("1", "zzzz1234", "ddd")])["1"]
        self.assertEqual((r["relation"], r["code"], r["basis"]), ("exact", "ddd", "ISO 639-3 code"))

    def test_dialect_is_a_subset(self):
        rows = mapped([language("1", "aaad1234")])
        self.assertEqual((rows["1"]["relation"], rows["1"]["code"]), ("subset", "aaa"))
        rows = mapped([language("1", "aaaa1234"), language("2", "", dialect_of="1")])
        self.assertEqual((rows["2"]["relation"], rows["2"]["code"]), ("subset", "aaa"))

    def test_proto_language_through_its_family(self):
        rows = mapped(
            [language("1", "pppp1234", proto=True), language("2", "aaaa1234", proto=True)]
        )
        self.assertEqual((rows["1"]["relation"], rows["1"]["code"]), ("exact", "ppp-pro"))
        # A proto-language never takes a modern language's code.
        self.assertEqual(rows["2"]["relation"], "unresolved")

    def test_reviewed_rows_replace_and_keep_the_derived(self):
        decision = {
            "id": "1",
            "relation": "exact",
            "code": "qqq",
            "rationale": "r",
            "reviewer": "m",
        }
        r = mapped([language("1")], [decision])["1"]
        self.assertEqual((r["relation"], r["code"], r["reviewer"]), ("exact", "qqq", "m"))
        self.assertEqual(r["derived"]["relation"], "unresolved")

    def test_reviewed_rows_are_checked(self):
        bad = [
            {"id": "9", "relation": "exact", "code": "aaa", "rationale": "r", "reviewer": "m"},
            {"id": "1", "relation": "same", "code": "aaa", "rationale": "r", "reviewer": "m"},
            {"id": "1", "relation": "exact", "code": None, "rationale": "r", "reviewer": "m"},
            {"id": "1", "relation": "exact", "code": "nope", "rationale": "r", "reviewer": "m"},
        ]
        for decision in bad:
            with self.assertRaises(ValueError):
                mapped([language("1")], [decision])

    def test_summary_keeps_proto_languages_apart(self):
        rows = build(
            [language("1", "aaaa1234"), language("2"), language("3", "pppp1234", proto=True)],
            Catalog(BUNDLE),
        )
        counts = summary(rows)
        self.assertEqual(
            counts["varieties"], {"exact": 1, "unresolved": 1, "total": 2, "exact_share": 0.5}
        )
        self.assertEqual(counts["proto-languages"]["exact"], 1)


if __name__ == "__main__":
    unittest.main()
