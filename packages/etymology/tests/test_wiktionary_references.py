"""Offline checks of page wikitext reading and the References converter (core-5)."""

import gzip
import json
import tempfile
import unittest
from pathlib import Path

from etymology import assertions as A
from etymology import wikitext
from etymology.convert import wiktionary_references as R

EDL = """{{#invoke:quote|call_quote_template|en
|template=cite-book
|last=De Vaan
|first=Michiel
|authorlink=Michiel de Vaan
|entry={{#if:{{{head|{{{1|}}}}}}|la:{{{head|{{{1|}}}}}}}}
|title=Etymological Dictionary of Latin and the other Italic Languages
|year=2008
|location=Leiden, Boston
|publisher=Brill
|pageparam=2
}}<noinclude>{{documentation}}</noinclude>"""

PAGE = """==Latin==
===Etymology 1===
From {{inh|la|itc-pro|*lukʷos}}.<ref>{{R:itc:EDL|lupus|353}}</ref>
====Noun====
# wolf
====References====
<references/>
* {{R:la:EDL|lupus|353}}
===Etymology 2===
From something else.
===References===
* {{R:la:Gaffiot|lupus}}
* {{cite-book|author=Vasmer, Max|title=Wörterbuch|year=1953}}
===Further reading===
* {{R:la:L&S|lupus}}
==English==
===Etymology===
Borrowed.
===References===
* {{R:itc:EDL|page=10}}
"""


class Wikitext(unittest.TestCase):
    def test_sections_carry_their_language_and_etymology(self):
        got = [(lang, ety, name) for lang, ety, _, name, _ in wikitext.sections(PAGE)]
        self.assertIn(("Latin", "1", "References"), got)
        self.assertIn(("Latin", None, "References"), got)  # beside the etymologies
        self.assertIn(("English", None, "References"), got)  # unnumbered: no label

    def test_templates_split_at_top_level(self):
        got = wikitext.templates(
            "* {{R:x|{{lang|a|b}}|p=[[c|d]]}} {{R:y|{{{1|}}}|page={{{page|}}}}}", "R:"
        )
        self.assertEqual(
            got, [("R:x", ["{{lang|a|b}}", "p=[[c|d]]"]), ("R:y", ["{{{1|}}}", "page={{{page|}}}"])]
        )

    def test_pages_from_dump_lines(self):
        lines = [
            "<page>\n",
            "<title>a &amp; b</title>\n",
            "<ns>0</ns>\n",
            '<text bytes="9" xml:space="preserve">x &lt;ref&gt;\n',
            "y</text>\n",
            "</page>\n",
            "<page>\n",
            "<title>Talk:a</title>\n",
            "<ns>1</ns>\n",
            "<text>z</text>\n",
            "</page>\n",
        ]
        self.assertEqual(list(wikitext.pages(lines)), [("a & b", "0", "x <ref>\ny")])


class Bibliography(unittest.TestCase):
    def test_a_template_page_gives_authors_year_and_argument_roles(self):
        name, call = R.citation_call(EDL)
        self.assertEqual(name, "cite-book")
        d = R.describe(call)
        self.assertEqual((d["authors"], d["year"]), (["Michiel de Vaan"], 2008))
        self.assertTrue(d["citation"].startswith("Michiel de Vaan (2008). Etymological Dictionary"))
        roles = R.argument_roles(call)
        self.assertEqual(R.locator(roles, ["lupus", "353"]), "353")
        self.assertEqual(R.locator(roles, ["lupus"]), "s.v. lupus")

    def test_names(self):
        self.assertEqual(R.person_name("Vasmer, Max"), "Max Vasmer")
        self.assertEqual(R.person_name("w:gl:Antón Santamarina"), "Antón Santamarina")
        self.assertEqual(
            R.split_people("Lytkin, V. I., Gulyaev, E. S."), ["V. I. Lytkin", "E. S. Gulyaev"]
        )
        self.assertEqual(
            R.split_people("Liddell, Henry George; Scott, Robert et al."),
            ["Henry George Liddell", "Robert Scott"],
        )

    def test_expand_takes_defaults_and_else_branches(self):
        self.assertEqual(R.expand("{{#switch:{{{3|}}}|1=1964|#default=1964–1973}}"), "1964–1973")
        self.assertEqual(R.expand("[[w:X|Y]] {{w|A|B}} ''t''"), "Y B t")


class Convert(unittest.TestCase):
    def test_entry_credits_by_section_place(self):
        with tempfile.TemporaryDirectory(dir=A.REPO) as tmp:
            tmp = Path(tmp)
            rows = [
                {"template": "R:itc:EDL", "text": EDL},
                {
                    "template": "R:la:EDL",
                    "text": "#REDIRECT [[Template:R:itc:EDL]]",
                    "redirect": "R:itc:EDL",
                },
            ]
            for lang, ety, level, name, body in wikitext.sections(PAGE):
                kind = R.KEPT.get(name)
                if kind:
                    rows.append(
                        {
                            "page": "lupus",
                            "lang": lang,
                            "ety": ety,
                            "level": level,
                            "kind": kind,
                            "text": body,
                        }
                    )
            with gzip.open(tmp / "sections.jsonl.gz", "wt") as f:
                f.writelines(json.dumps(r) + "\n" for r in rows)
            result = R.convert(tmp / "sections.jsonl.gz", tmp / "sections.jsonl.gz", tmp)
            counts = result["counts"]
            self.assertEqual(result["errors"], [])
            with gzip.open(tmp / "section_credits.jsonl.gz", "rt") as f:
                got = {(d["page"], d["lang"], d["ety"]): d for d in map(json.loads, f)}
        edl = "work:wiktionary:R:itc:EDL"
        one = {(c["work"], c.get("locator")) for c in got["lupus", "Latin", "1"]["credits"]}
        # The redirect R:la:EDL is the same work.
        self.assertIn((edl, "353"), one)
        # Etymology 2 has no References of its own: the language-wide section covers it,
        # which the processor applies. With no template page, no argument is a page.
        self.assertNotIn(("lupus", "Latin", "2"), got)
        wide = {(c["work"], c.get("locator")) for c in got["lupus", "Latin", None]["credits"]}
        self.assertIn(("work:wiktionary:R:la:Gaffiot", None), wide)
        self.assertNotIn((edl, "353"), wide)
        self.assertEqual(
            got["lupus", "English", None]["credits"],
            [{"work": edl, "role": "follows", "scope": "entry", "locator": "10"}],
        )
        self.assertEqual(counts["further sections citing a work"], 1)
        self.assertTrue(any(w.startswith("work:wiktionary:cite:") for w, _ in wide))


if __name__ == "__main__":
    unittest.main()
