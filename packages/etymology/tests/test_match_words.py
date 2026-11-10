"""Offline checks of the word matcher (match_words) on a small invented source."""

import unittest

from etymology import assertions as A
from etymology.match_words import SPECS, Keyer, Matcher, Source, Wiktionary, keys

SRC = "wold@v4.2"
WK = "wiktionary@2026-11-01"


def record(rid, code, form, gloss=None, source=SRC):
    r = {
        "type": "record",
        "id": rid,
        "source": source,
        "language": {"label": code, "code": code, "relation": "exact"},
        "form": form,
        "reconstructed": False,
    }
    if gloss:
        r["gloss"] = gloss
    return r


def origin(word, parent, source, holder):
    h = A.hypothesis(
        word, [{"child": word, "parents": [parent], "relation": "derivation", "mode": "borrowed"}]
    )
    a = A.assertion(source, word, h["id"], "endorses", holder=holder, asserter=holder)
    return [h, a]


class Keys(unittest.TestCase):
    def test_keys(self):
        self.assertEqual(keys("mōns"), {"mons"})
        self.assertEqual(keys("roque, rocque, roche"), {"roque", "rocque", "roche"})
        self.assertEqual(keys("lâa (1)"), {"laa"})
        self.assertEqual(keys("kupiga_rangi"), {"kupigarangi"})


class Homographs(unittest.TestCase):
    """English bay: the etymology that names WOLD's French source is the match."""

    def setUp(self):
        word = record("wold:English-1", "en", "bay", "bay, gulf")
        donor = record("wold:loan-1", "fr", "baie", "bay")
        src = Source(
            [word, donor, *origin(word["id"], donor["id"], SRC, "agent:wold-vocabulary-13")],
            SPECS["wold"],
        )
        objs = [
            record("wiktionary:en:bay#1", "en", "bay", source=WK),
            record("wiktionary:en:bay#2", "en", "bay", source=WK),
            record("wiktionary:fr:baie#1", "fr", "baie", source=WK),
            record("wiktionary:la:badius#1", "la", "badius", source=WK),
            *origin("wiktionary:en:bay#1", "wiktionary:fr:baie#1", WK, "agent:wiktionary-editors"),
            *origin(
                "wiktionary:en:bay#2", "wiktionary:la:badius#1", WK, "agent:wiktionary-editors"
            ),
        ]
        glosses = [
            {
                "code": "en",
                "word": "bay",
                "canonical": "bay",
                "ety": "1",
                "glosses": ["a body of water"],
            },
            {
                "code": "en",
                "word": "bay",
                "canonical": "bay",
                "ety": "2",
                "glosses": ["reddish-brown"],
            },
            {"code": "fr", "word": "baie", "canonical": "baie", "ety": "", "glosses": ["bay"]},
        ]
        keyer = Keyer(SPECS["wold"])
        self.matcher = Matcher(src, Wiktionary(objs, glosses, keyer), keyer)

    def test_the_etymology_naming_the_source(self):
        d = self.matcher.run()
        self.assertEqual(d["wold:English-1"][:2], ("match", "wiktionary:en:bay#1"))
        self.assertEqual(d["wold:English-1"][2]["lineage"], "source named")
        self.assertEqual(d["wold:loan-1"][:2], ("match", "wiktionary:fr:baie#1"))


if __name__ == "__main__":
    unittest.main()
