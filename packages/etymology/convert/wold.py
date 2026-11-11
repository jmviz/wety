"""WOLD's CLDF release to assertions (Haspelmath and Tadmor 2009, World Loanword Database).

Each of WOLD's 41 vocabularies is its authors' account of the loanwords of one language:
for each word, how likely it is to be borrowed (`Borrowed`: clearly, probably, perhaps,
very little evidence, no evidence) and, in `borrowings.csv`, the source words it goes
back to. The vocabulary's authors hold every assertion on its words.

A word rated clearly, probably or perhaps borrowed converts, row by row:

- an `immediate` row: the word is borrowed (mode borrowed) from the source word. Several
  immediate rows for one word are WOLD's alternatives (Selice Romani praho from
  Bulgarian, Croatian, Serbian or Slovenian): an exclusive alternative set, no
  preference;
- an `earlier` row: the word goes back to the source word by unstated steps (ultimate),
  through its immediate source (English mountain, from French, from Latin);
- the rating hedges the link: "probably" at probable, "perhaps" at possible; a source
  word WOLD marks uncertain (`Source_certain` no) at possible;
- a row that names only a language (no source word; "Unidentified" is no language) has
  a source record with no form.

Rows on words rated "very little" or "no evidence" for borrowing name the source of a
part of a compound or of an earlier stage (Zinacantán Tzotzil tzahal 7ora, with Spanish
hora), and rows on a compound or phrase name the source of the part that is borrowed
(English earlobe, from Latin lobus): they are counted, not converted. The source word's language is read through the
donor map (`etc/donors.json`, WOLD's languoid names to Wiktionary codes), the word's own
through the language map. An assertion's locator is the word's page in WOLD
(`word/<Word_ID>`); each source word's record keeps its `borrowings.csv` row.
"""

import collections
import json
import re

from etymology import assertions as A
from etymology.convert import SOURCES, Languages, agent, source
from etymology.convert.etymdict import main_for, read_table

NAME = "wold"
CLDF = SOURCES / NAME / "raw/lexibank-wold-1df62b9/cldf"
HOMOGRAPH = re.compile(r"\s*\((\d+)\)$")
HEDGE = {
    "1. clearly borrowed": None,
    "2. probably borrowed": "probable",
    "3. perhaps borrowed": "possible",
}
# WOLD rates a compound or phrase borrowed when a part is (English earlobe: "second part
# only is borrowed"); its source word is then the part's source, not the word's
PART_BORROWED = {"analyzable compound", "analyzable phrasal"}
INPUTS = (
    "languages.csv",
    "forms.csv",
    "parameters.csv",
    "borrowings.csv",
    "contributions.csv",
    "contributors.csv",
)


class Donors(Languages):
    """WOLD's donor languoids, by name, to Wiktionary codes (`etc/donors.json`)."""

    def __init__(self):
        data = json.loads((SOURCES / NAME / "etc/donors.json").read_text())
        self.rows = {r["id"]: r for r in data["rows"]}


class WoldRelease:
    def __init__(self):
        self.cldf = CLDF
        self.languages = {r["ID"]: r for r in read_table(CLDF / "languages.csv")}
        self.forms = {r["ID"]: r for r in read_table(CLDF / "forms.csv")}
        self.meanings = {r["ID"]: r["Name"] for r in read_table(CLDF / "parameters.csv")}
        self.borrowings = collections.defaultdict(list)
        for r in read_table(CLDF / "borrowings.csv"):
            self.borrowings[r["Target_Form_ID"]].append(r)
        names = {r["ID"]: r["Name"] for r in read_table(CLDF / "contributors.csv")}
        self.vocabularies = {}
        for c in read_table(CLDF / "contributions.csv"):
            authors = [names[i] for i in c["primary_contributors"].split() if i in names]
            self.vocabularies[c["Language_ID"]] = (c["ID"], authors or [c["Contributor"]])


class WoldConverter:
    NAME = NAME
    LABEL = "WOLD"

    def __init__(self, release, extraction):
        self.r = release
        self.lang = Languages(NAME)
        self.donors = Donors()
        self.src = source(NAME)
        self.out = A.Writer()
        self.extraction = extraction["id"]
        self.counts = collections.Counter()
        self.out.add(self.src)
        self.out.add(extraction)

    def holder(self, language_id):
        """The agent of a vocabulary: its authors."""
        vocabulary, authors = self.r.vocabularies[language_id]
        language = self.r.languages[language_id]["Name"]
        names = " and ".join(authors)
        kind = "person" if len(authors) == 1 else "editors"
        a = agent(
            f"agent:wold-vocabulary-{vocabulary}", kind, f"{names} (WOLD's {language} vocabulary)"
        )
        return self.out.add(a)

    def word(self, form_id):
        f = self.r.forms[form_id]
        language = self.r.languages[f["Language_ID"]]
        rec = {
            "type": "record",
            "id": f"{NAME}:{form_id}",
            "source": self.src["id"],
            "language": self.lang(f["Language_ID"], language["Name"]),
            "form": f["Value"],
            "reconstructed": False,
            "gloss": f["gloss"] or self.r.meanings[f["Parameter_ID"]],
            "locator": f"forms.csv#{form_id}",
        }
        m = HOMOGRAPH.search(f["Value"])
        if m:
            rec["form"], rec["homograph"] = f["Value"][: m.start()], m.group(1)
        return self.out.add(rec)

    def source_word(self, b):
        word = b["Source_word"].strip()
        language = self.donors(b["Source_languoid"])
        known = b["Source_languoid"] != "Unidentified"
        rec = {
            "type": "record",
            "id": f"{NAME}:loan-{b['ID']}",
            "source": self.src["id"],
            "language": language
            if known
            else {**language, "label": "a source WOLD does not identify"},
            "form": word.lstrip("*") or None,
            "reconstructed": word.startswith("*")
            or self.donors.rows.get(b["Source_languoid"], {}).get("is_proto", False),
            "locator": f"borrowings.csv#{b['ID']}",
        }
        if b["Source_meaning"]:
            rec["gloss"] = b["Source_meaning"]
        return self.out.add(rec)

    def convert(self):
        for form_id, rows in self.r.borrowings.items():
            f = self.r.forms[form_id]
            if f["Borrowed"] not in HEDGE:
                self.counts[f"rows on words rated {f['Borrowed'][3:]!r}: not converted"] += len(
                    rows
                )
                continue
            if f["Analyzability"] in PART_BORROWED:
                why = "rows on compounds and phrases (a part borrowed): not converted"
                self.counts[why] += len(rows)
                continue
            word = self.word(form_id)
            holder = self.holder(f["Language_ID"])
            immediate = [b for b in rows if b["Source_relation"] == "immediate"]
            hyps = {}
            for b in rows:
                parent = self.source_word(b)
                if b["Source_relation"] == "immediate":
                    link = {
                        "child": word,
                        "parents": [parent],
                        "relation": "derivation",
                        "mode": "borrowed",
                    }
                else:
                    link = {"child": word, "parents": [parent], "relation": "ultimate"}
                h = A.hypothesis(word, [link])
                self.out.add(h)
                hyps[b["ID"]] = h["id"]
                self.counts[f"{b['Source_relation']} rows converted"] += 1
            in_sets = {}
            if len(immediate) > 1:
                alt = A.alternative_set(sorted(hyps[b["ID"]] for b in immediate), exclusive=True)
                self.out.add(alt)
                self.endorse(alt["id"], holder, f"word/{f['Word_ID']}")
                in_sets = {b["ID"]: [alt["id"]] for b in immediate}
                self.counts["alternative sets of immediate sources"] += 1
            for b in rows:
                hedges = []
                if HEDGE[f["Borrowed"]]:
                    hedges.append(
                        {"level": HEDGE[f["Borrowed"]], "words": f["Borrowed"], "link": 0}
                    )
                if b["Source_certain"] == "no":
                    hedges.append(
                        {"level": "possible", "words": "source word uncertain", "link": 0}
                    )
                self.endorse(
                    hyps[b["ID"]],
                    holder,
                    f"word/{f['Word_ID']}",
                    hedges=hedges,
                    quote=b["Comment"] or None,
                    in_sets=in_sets.get(b["ID"], []),
                )

    def endorse(self, target, holder, locator, **fields):
        a = A.assertion(
            self.src["id"],
            locator,
            target,
            "endorses",
            asserter=holder,
            holder=holder,
            extraction=self.extraction,
            **fields,
        )
        return self.out.add(a)


def main(argv=None):
    main_for(WoldConverter, WoldRelease(), argv, tables=INPUTS)


if __name__ == "__main__":
    main()
