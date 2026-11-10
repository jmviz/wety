"""The Comparative Cariban Database's clld app deposit to assertions (Matter 2021).

Published as a clld app's database dump, not as CLDF. Its cognate sets (`cognateset`,
lexical and grammatical: pronouns, affixes) gather morphemes (`unit`) of the Cariban
languages, among them Proto-Cariban's, which the set's name gives too (*(ə)wɨ '1PRO').
It is read into pyetymdict's shape (`etymdict`): a set's reconstruction is its
Proto-Cariban morpheme, and links follow the nested rule on a tree with Proto-Cariban
above every language (the database's phylogenies are alternative classifications; none
is read). The set's description is its gloss ('eye-1'), and its morphemes' where the
database gives them none.
"""

import collections
import re
from functools import cached_property

from etymology.convert import SOURCES
from etymology.convert.clld_dump import tables
from etymology.convert.etymdict import Converter, Release, Tree, main_for

NAME = "cariban"
DUMP = SOURCES / NAME / "raw/cariban.sql.gz"
PROTO = "pc"
WANTED = {"language", "identifier", "languageidentifier", "cognateset", "cognate", "unit"}


class CaribanRelease(Release):
    """The dump's tables in pyetymdict's shape."""

    def __init__(self):
        self.name, self.cldf = NAME, DUMP.parent
        t = tables(DUMP, WANTED)
        by_pk = {r["pk"]: r for r in t["language"]}
        ids = {r["pk"]: r for r in t["identifier"]}
        codes = collections.defaultdict(dict)
        for li in t["languageidentifier"]:
            i = ids[li["identifier_pk"]]
            codes[li["language_pk"]][i["type"]] = i["name"]
        self.languages = {
            r["id"]: {
                "ID": r["id"],
                "Name": r["name"],
                "Glottocode": codes[pk].get("glottolog", ""),
                "ISO639P3code": codes[pk].get("iso639-3", ""),
                "Is_Proto": "true" if r["id"] == PROTO else "false",
            }
            for pk, r in by_pk.items()
        }
        units = {r["pk"]: r for r in t["unit"]}
        sets = {r["pk"]: r for r in t["cognateset"]}
        self.forms, self.sets, self.cognates = {}, {}, collections.defaultdict(list)
        for c in t["cognate"]:
            u = units[c["counterpart_pk"]]
            lang = by_pk[u["language_pk"]]["id"]
            form_id = f"{lang}-{u['pk']}"
            self.forms[form_id] = {
                "ID": form_id,
                "Language_ID": lang,
                "Value": u["name"],
                "Description": u["description"] or "",
                "Doubt": "false",
                "Loan": "",
                "Source": "",
            }
            s = sets[c["cognateset_pk"]]
            self.cognates[s["id"]].append({"ID": c["pk"], "Form_ID": form_id, "Doubt": "false"})
        for s in sets.values():
            meaning = re.sub(r"-\d+$", "", s["description"] or "")
            for r in self.cognates[s["id"]]:
                form = self.forms[r["Form_ID"]]
                form["Description"] = form["Description"] or meaning
            protos = [r["Form_ID"] for r in self.cognates[s["id"]] if self.is_proto(r["Form_ID"])]
            self.sets[s["id"]] = {
                "ID": s["id"],
                "Form_ID": protos[0] if protos else None,
                "Doubt": "false",
                "type": s["cogset_type"],
            }
        self.cf, self.cf_items, self.borrowings = {}, {}, []
        self.loan_members, self.doubted, self._bib = {}, set(), {}

    @property
    def bib(self):
        return self._bib

    @cached_property
    def tree(self):
        parents = {lang: PROTO for lang in self.languages if lang != PROTO}
        return Tree(parents, PROTO, self.levels(), {}, self.languages)


class CaribanConverter(Converter):
    NAME = NAME
    LABEL = "Comparative Cariban Database"
    EDITORS = "agent:florian-matter"
    EDITORS_NAME = "Florian Matter (the Comparative Cariban Database's author)"
    IMPLIED = "implied by the Comparative Cariban Database's sets"
    YEAR = 2021

    def set_category(self, set_id):
        return f"cognate set, {self.r.sets[set_id]['type']}"


def main(argv=None):
    main_for(CaribanConverter, CaribanRelease(), argv, tables=("cariban.sql.gz",))


if __name__ == "__main__":
    main()
