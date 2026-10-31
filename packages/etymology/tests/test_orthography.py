"""Offline checks of comparison keys from orthography profiles."""

import tempfile
import unittest
from pathlib import Path

from etymology.orthography import Profile, key, same

COMMON = """Kind\tGrapheme\tKey\tNote
grapheme\t*\t\t
grapheme\t-\t\t
grapheme\t‑\t\t
grapheme\th₁\tH\t
grapheme\th₂\tH\t
grapheme\tH\tH\t
grapheme\tʰ\th\t
grapheme\tʷ\tu\t
grapheme\tw\tu\t
grapheme\ty\ti\t
grapheme\tj\ti\t
grapheme\t́\t\t
grapheme\t̄\t\t
grapheme\t̥\t\t
grapheme\t̯\t\t
"""
DE_VAAN = COMMON + "grapheme\th₁/₃\tH\t\nending\to\t\t\nending\te\t\t\n"
WIKTIONARY = COMMON + "grapheme\tz\ts\t\nending\tos\t\t\nending\ts\t\t\nending\teti\t\t\n"


def profile(text):
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "profile.tsv"
        p.write_text("# a comment line\n" + text)
        return Profile.read(p)


DV, WK = profile(DE_VAAN), profile(WIKTIONARY)


class Keys(unittest.TestCase):
    def test_folds_laryngeals_resonants_and_labiovelars(self):
        self.assertTrue(same(key("*h₁ésh₂‑r", DV), key("*h₁ésh₂r̥", WK)))
        self.assertTrue(same(key("*ml‑u̯o‑", DV), key("*ml̥-wo-s", WK)))
        self.assertEqual(key("*sekʷ‑", DV).keys, {"seku"})

    def test_case_folds_but_not_the_laryngeal(self):
        self.assertTrue(same(key("*kerēs", DV), key("*Kerēs", WK)))
        self.assertEqual(key("*Hoi‑no‑", DV).keys, {"Hoino", "Hoin"})

    def test_stem_against_citation_form(self):
        self.assertTrue(same(key("*lukʷo‑", DV), key("*lukʷos", WK)))
        self.assertTrue(same(key("*kors‑e/o‑", DV), key("*korzō", WK)))
        self.assertTrue(same(key("*krs‑e/o‑", DV), key("*ḱr̥séti", WK)))

    def test_optional_and_alternative_segments(self):
        k = key("*(s)keh₂u‑l‑i‑", DV)
        self.assertEqual(k.keys, {"skeHuli", "keHuli"})
        self.assertEqual(k.uncertain, 1)
        k = key("*ḱ/ken‑", DV)
        self.assertEqual(k.keys, {"ken"})
        self.assertEqual(k.uncertain, 1)
        self.assertEqual(key("*ḱerh₁/₃‑", DV).keys, {"kerH"})

    def test_several_forms_hedges_and_unknown_graphemes(self):
        k = key("*h₁ésh₂‑r / *h₁sh₂‑én‑s?", DV)
        self.assertTrue(k.hedged)
        self.assertEqual(k.keys, {"HesHr", "HsHens"})
        self.assertEqual(key("*pend‑e/o‑ >", DV).unknown, {">"})

    def test_a_different_word_keeps_a_different_key(self):
        self.assertFalse(same(key("*li‑n‑ǵʰ‑", DV), key("*leyǵʰ-", WK)))

    def test_profile_rejects_duplicates_and_unknown_kinds(self):
        with self.assertRaises(ValueError):
            profile(COMMON + "grapheme\tw\tw\t\n")
        with self.assertRaises(ValueError):
            profile(COMMON + "suffix\to\t\t\n")


if __name__ == "__main__":
    unittest.main()
