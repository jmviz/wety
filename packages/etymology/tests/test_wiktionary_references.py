"""Offline checks of page wikitext reading and the References converter (core-5, x-5)."""

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


def section(*calls):
    """An entries section whose one block holds these calls."""
    return {
        "digest": "x",
        "blocks": [
            {
                "kind": "item",
                "depth": 0,
                "text": "",
                "spans": [{"kind": "template", "start": 0, "end": 0, "calls": list(calls)}],
            }
        ],
    }


def call(name, **args):
    return {"name": name, "args": args}


# Latin lupus with two etymologies and English lupus, as entries: Etymology 1's own
# References and a footnote, a References section beside the etymologies (on each of
# Latin's entries), Further reading, and English's unnumbered References.
EDL1 = call("R:la:EDL", **{"1": "lupus", "2": "353"})
WIDE = section(
    call("R:la:Gaffiot", **{"1": "lupus"}),
    call("cite-book", author="Vasmer, Max", title="Wörterbuch", year="1953"),
)
ENTRIES = [
    {
        "page": "lupus",
        "lang_heading": "Latin",
        "ety_label": "1",
        "etymology": {
            "digest": "y",
            "blocks": [],
            "notes": [
                {"id": "cite_note-1", "text": "", "calls": [call("r:itc:EDL", **{"1": "lupus", "2": "353"})]},
                {"id": "cite_note-2", "text": "A. Author (2000), A Book", "calls": []},
            ],
        },
        "references": section(EDL1),
        "language_references": WIDE,
        "further_reading": section(call("R:la:L&S", **{"1": "lupus"})),
    },
    {"page": "lupus", "lang_heading": "Latin", "ety_label": "2", "language_references": WIDE},
    {
        "page": "lupus",
        "lang_heading": "English",
        "ety_label": None,
        "references": section(call("R:itc:EDL", page="10")),
    },
]


class Wikitext(unittest.TestCase):
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
        self.assertEqual(R.locator(roles, {"1": "lupus", "2": "353"}), "353")
        self.assertEqual(R.locator(roles, {"1": "lupus"}), "s.v. lupus")

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
    def test_entry_and_note_credits_by_section_place(self):
        with tempfile.TemporaryDirectory(dir=A.REPO) as tmp:
            tmp = Path(tmp)
            templates = [
                {"template": "R:itc:EDL", "text": EDL},
                {
                    "template": "R:la:EDL",
                    "text": "#REDIRECT [[Template:R:itc:EDL]]",
                    "redirect": "R:itc:EDL",
                },
            ]
            with gzip.open(tmp / "templates.jsonl.gz", "wt") as f:
                f.writelines(json.dumps(r) + "\n" for r in templates)
            with gzip.open(tmp / "entries.jsonl.gz", "wt") as f:
                f.writelines(json.dumps(e, separators=(",", ":")) + "\n" for e in ENTRIES)
            result = R.convert(
                tmp / "templates.jsonl.gz", tmp / "entries.jsonl.gz", tmp / "templates.jsonl.gz", tmp
            )
            counts = result["counts"]
            self.assertEqual(result["errors"], [])
            with gzip.open(tmp / "section_credits.jsonl.gz", "rt") as f:
                got = {(d["page"], d["lang"], d["ety"], d.get("note")): d for d in map(json.loads, f)}
        edl = "work:wiktionary:R:itc:EDL"
        one = {(c["work"], c.get("locator")) for c in got["lupus", "Latin", "1", None]["credits"]}
        # The redirect R:la:EDL is the same work.
        self.assertIn((edl, "353"), one)
        # Etymology 2 has no References of its own: the language-wide section covers it,
        # which the processor applies; it is read once. With no template page, no
        # argument is a page.
        self.assertNotIn(("lupus", "Latin", "2", None), got)
        wide = {(c["work"], c.get("locator")) for c in got["lupus", "Latin", None, None]["credits"]}
        self.assertIn(("work:wiktionary:R:la:Gaffiot", None), wide)
        self.assertNotIn((edl, "353"), wide)
        self.assertTrue(any(w.startswith("work:wiktionary:cite:") for w, _ in wide))
        self.assertEqual(counts["references sections"], 3)
        self.assertEqual(
            got["lupus", "English", None, None]["credits"],
            [{"work": edl, "role": "follows", "scope": "entry", "locator": "10"}],
        )
        # A footnote citing a template, whatever the case of its name, credits that
        # work; one citing none is left to the processor.
        self.assertEqual(
            got["lupus", "Latin", "1", "cite_note-1"]["credits"],
            [{"work": edl, "role": "follows", "scope": "note", "locator": "353"}],
        )
        self.assertNotIn(("lupus", "Latin", "1", "cite_note-2"), got)
        self.assertEqual(counts["further sections citing a work"], 1)


if __name__ == "__main__":
    unittest.main()
