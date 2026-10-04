import unittest
from unittest.mock import patch

from harness import bibliography as b

MODS_RECORDS = """<?xml version="1.0"?>
<zs:searchRetrieveResponse xmlns:zs="http://www.loc.gov/zing/srw/"><zs:records>
<zs:record><zs:recordData><mods xmlns="http://www.loc.gov/mods/v3">
  <titleInfo><nonSort>A </nonSort><title>history of the English language</title></titleInfo>
  <name type="personal"><namePart type="family">Baugh</namePart><namePart type="given">Albert C.</namePart>
    <role><roleTerm type="code">oth</roleTerm></role></name>
  <originInfo><place><placeTerm type="code">xxu</placeTerm></place><place><placeTerm type="text">London [u.a.]</placeTerm></place>
    <publisher>Routledge</publisher><dateIssued>2013</dateIssued><edition>6. ed.</edition></originInfo>
  <physicalDescription><extent>1 Online-Ressource</extent></physicalDescription>
  <identifier type="isbn">9780203568255</identifier>
</mods></zs:recordData></zs:record>
<zs:record><zs:recordData><mods xmlns="http://www.loc.gov/mods/v3">
  <titleInfo><nonSort>A </nonSort><title>history of the English language</title></titleInfo>
  <name type="personal"><namePart type="family">Baugh</namePart><namePart type="given">Albert C.</namePart>
    <role><roleTerm type="code">aut</roleTerm></role></name>
  <name type="personal"><namePart type="family">Cable</namePart><namePart type="given">Thomas</namePart>
    <role><roleTerm type="code">aut</roleTerm></role></name>
  <originInfo><place><placeTerm type="text">London</placeTerm></place>
    <publisher>Routledge</publisher><dateIssued>2013</dateIssued><edition>6. ed.</edition></originInfo>
  <physicalDescription><extent>XVIII, 446 S. 24 cm</extent></physicalDescription>
  <identifier type="isbn">0415655951</identifier><identifier type="isbn">9780415655958</identifier>
</mods></zs:recordData></zs:record>
</zs:records></zs:searchRetrieveResponse>"""

CSL_CHAPTER = {
    "type": "book-chapter",
    "title": "Sanskrit",
    "author": [{"family": "Jamison", "given": "Stephanie W."}],
    "container-title": ["The Ancient Languages of Asia and the Americas"],
    "publisher": "Cambridge University Press",
    "issued": {"date-parts": [[2008, 4, 10]]},
    "page": "6-32",
    "ISBN": ["9780521684941"],
    "DOI": "10.1017/cbo9780511486838.005",
}


class BibliographyTests(unittest.TestCase):
    def test_mods_prefers_print_takes_article_and_fullest_creators(self):
        with patch.object(b, "get", return_value=MODS_RECORDS.encode()):
            x = b.k10plus_search(
                {
                    "title": "A History of the English Language",
                    "authors": ["Albert C. Baugh"],
                    "year": 2013,
                }
            )
        item_type, fields, creators = b.from_mods(x)
        self.assertEqual(fields["title"], "A history of the English language")
        self.assertEqual(
            (fields["ISBN"], fields["numPages"], fields["place"]),
            ("9780415655958", "XVIII, 446", "London"),
        )
        self.assertEqual([c["lastName"] for c in creators], ["Baugh", "Cable"])
        with patch.object(b, "get", return_value=MODS_RECORDS.encode()):
            self.assertIsNone(
                b.k10plus_search(
                    {"title": "A History of English", "authors": ["Baugh"], "year": 2013}
                )
            )
            self.assertIsNone(
                b.k10plus_search(
                    {
                        "title": "A History of the English Language",
                        "authors": ["Baugh"],
                        "year": 2012,
                    }
                )
            )

    def test_csl_chapter_gets_container_pages_and_editors_from_catalogue(self):
        editor = {
            "title": "Ancient languages of Asia and the Americas",
            "subtitle": None,
            "creators": [{"creatorType": "editor", "lastName": "Woodard", "firstName": "Roger D."}],
            "names": ["Woodard"],
            "place": "Cambridge [u.a.]",
            "publisher": "CUP",
            "date": "2008",
            "edition": None,
            "isbns": ["9780521684941"],
            "doi": None,
            "extent": "XIV, 284 S.",
            "online": False,
            "series": None,
            "language": "eng",
        }
        with (
            patch.object(b, "crossref_search", return_value=CSL_CHAPTER["DOI"]),
            patch.object(b, "csl", return_value=CSL_CHAPTER),
            patch.object(b, "k10plus_by_isbn", return_value=editor),
        ):
            res = b.resolve(
                {
                    "title": "Sanskrit",
                    "authors": ["Stephanie W. Jamison"],
                    "year": 2008,
                    "item_type": "chapter",
                }
            )
        self.assertEqual(res["item_type"], "bookSection")
        f = res["fields"]
        self.assertEqual(
            (f["bookTitle"], f["pages"], f["place"], f["DOI"]),
            (CSL_CHAPTER["container-title"][0], "6-32", "Cambridge", CSL_CHAPTER["DOI"]),
        )
        self.assertEqual(
            [(c["creatorType"], c["lastName"]) for c in res["creators"]],
            [("author", "Jamison"), ("editor", "Woodard")],
        )

    def test_crossref_search_requires_same_title_author_and_year(self):
        found = {
            "message": {
                "items": [
                    {
                        "type": "monograph",
                        "DOI": "10.1/old",
                        "title": ["Greek"],
                        "subtitle": ["A History of the Language and its Speakers"],
                        "author": [{"family": "Horrocks"}],
                        "issued": {"date-parts": [[1997]]},
                    },
                    {
                        "type": "journal-article",
                        "DOI": "10.1/review",
                        "title": ["Greek: A History of the Language and its Speakers"],
                        "author": [{"family": "Horrocks"}],
                        "issued": {"date-parts": [[2010]]},
                    },
                    {
                        "type": "monograph",
                        "DOI": "10.1/right",
                        "title": ["Greek"],
                        "subtitle": ["A History of the Language and its Speakers"],
                        "author": [{"family": "Horrocks"}],
                        "issued": {"date-parts": [[2010]]},
                    },
                ]
            }
        }
        r = {
            "title": "Greek: A History of the Language and its Speakers",
            "authors": ["Geoffrey Horrocks"],
            "year": 2010,
        }
        with patch.object(b, "get", return_value=found):
            self.assertEqual(b.crossref_search(r, "book"), "10.1/right")
            self.assertIsNone(b.crossref_search({**r, "authors": ["Someone Else"]}, "book"))

    def test_unresolved_is_reported(self):
        with (
            patch.object(b, "crossref_search", return_value=None),
            patch.object(b, "k10plus_search", return_value=None),
        ):
            self.assertIn(
                "unresolved", b.resolve({"title": "Nothing", "authors": ["Nobody"], "year": 2000})
            )


if __name__ == "__main__":
    unittest.main()
