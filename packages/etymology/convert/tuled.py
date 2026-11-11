"""TuLeD's CLDF release to assertions (Gerardi, Reichert, Aragon, List and Wientzek, Tupían
Lexical Database).

TuLeD codes the words for 447 meanings in 90 Tupían varieties into cognate sets, by
expert judgement and partially: a form's morphemes (`Segment_Slice`) may belong to
different sets. It reconstructs nothing, so each set of two or more forms is a cognate
set with no reconstruction, endorsed by TuLeD's authors, and no link is compiled; a set
of one form says nothing and is counted.
"""

import collections

from etymology import assertions as A
from etymology.convert import SOURCES, Languages, agent, source
from etymology.convert.etymdict import main_for, read_table

NAME = "tuled"
CLDF = SOURCES / NAME / "raw/lexibank-tuled-a67d65a/cldf"
AUTHORS = "agent:tuled-authors"
INPUTS = ("languages.csv", "forms.csv", "parameters.csv", "cognates.csv")


class TuledRelease:
    def __init__(self):
        self.cldf = CLDF
        self.languages = {r["ID"]: r for r in read_table(CLDF / "languages.csv")}
        self.forms = {r["ID"]: r for r in read_table(CLDF / "forms.csv")}
        self.meanings = {r["ID"]: r["Name"] for r in read_table(CLDF / "parameters.csv")}
        self.sets = collections.defaultdict(list)
        for r in read_table(CLDF / "cognates.csv"):
            self.sets[r["Cognateset_ID"]].append(r)


class TuledConverter:
    NAME = NAME
    LABEL = "TuLeD"

    def __init__(self, release, extraction):
        self.r = release
        self.lang = Languages(NAME)
        self.src = source(NAME)
        self.out = A.Writer()
        self.extraction = extraction["id"]
        self.counts = collections.Counter()
        names = "Gerardi, Reichert, Aragon, List and Wientzek (TuLeD's authors)"
        for o in (self.src, extraction, agent(AUTHORS, "editors", names)):
            self.out.add(o)

    def record(self, form_id):
        f = self.r.forms[form_id]
        rec = {
            "type": "record",
            "id": f"{NAME}:{form_id}",
            "source": self.src["id"],
            "language": self.lang(f["Language_ID"], self.r.languages[f["Language_ID"]]["Name"]),
            "form": f["Value"],
            "reconstructed": False,
            "gloss": self.r.meanings[f["Parameter_ID"]],
            "locator": f"forms.csv#{form_id}",
        }
        return self.out.add(rec)

    def convert(self):
        for set_id, rows in self.r.sets.items():
            members = list(dict.fromkeys(self.record(r["Form_ID"]) for r in rows))
            if len(members) < 2:
                self.counts["sets of one form: not converted"] += 1
                continue
            cs = {
                "type": "cognate_set",
                "id": f"{NAME}:{set_id}",
                "source": self.src["id"],
                "reconstruction": None,
                "members": members,
                "category": "cognate set (partial cognacy)",
            }
            self.out.add(cs)
            a = A.assertion(
                self.src["id"],
                f"cognates.csv#Cognateset_ID={set_id}",
                cs["id"],
                "endorses",
                asserter=AUTHORS,
                holder=AUTHORS,
                extraction=self.extraction,
            )
            self.out.add(a)
            self.counts["cognate sets"] += 1


def main(argv=None):
    main_for(TuledConverter, TuledRelease(), argv, tables=INPUTS)


if __name__ == "__main__":
    main()
