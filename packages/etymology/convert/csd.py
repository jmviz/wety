"""The CSD's clld app deposit to assertions (Rankin, Carter, Jones, Koontz, Rood and
Hartmann 2015, Comparative Siouan Dictionary).

The CSD is published as a clld app's database dump, not as CLDF. Its entries are
cognate sets: an entry (a `parameter`) has forms (`value`) in the Siouan-Catawba
languages, reconstructions among them at each level that has one (Proto-Siouan,
Proto-Crow-Hidatsa *hú•ri- 'yesterday'). It is read into pyetymdict's shape (`etymdict`):
a set's reconstruction is its form at the highest level, and links follow the nested
rule on the CSD's own tree of languages (`languoid`), which places every language. An
entry with no reconstruction is a cognate set with none. The forms' references are
credits for data, with their pages.
"""

import collections
from functools import cached_property

from etymology.convert import SOURCES
from etymology.convert.clld_dump import tables
from etymology.convert.etymdict import Converter, Release, Tree, main_for

NAME = "csd"
DUMP = SOURCES / NAME / "raw/csd.sql.gz"
WANTED = {
    "language",
    "languoid",
    "identifier",
    "languageidentifier",
    "parameter",
    "valueset",
    "value",
    "valuereference",
    "source",
}


class CsdRelease(Release):
    """The dump's tables in pyetymdict's shape."""

    def __init__(self):
        self.name, self.cldf = NAME, DUMP.parent
        t = tables(DUMP, WANTED)
        by_pk = {r["pk"]: r for r in t["language"]}
        languoid = {r["pk"]: r for r in t["languoid"]}
        ids = {r["pk"]: r for r in t["identifier"]}
        codes = collections.defaultdict(dict)
        for li in t["languageidentifier"]:
            i = ids[li["identifier_pk"]]
            codes[li["language_pk"]][i["type"]] = i["name"]
        self.languages = {}
        self.tree_parents = {}
        for pk, r in by_pk.items():
            lo = languoid[pk]
            self.languages[r["id"]] = {
                "ID": r["id"],
                "Name": r["name"],
                "Glottocode": codes[pk].get("glottolog", ""),
                "ISO639P3code": codes[pk].get("iso639-3", ""),
                "Is_Proto": "true" if lo["proto"] == "t" else "false",
                "level": int(lo["level"]),
            }
            if lo["parent_pk"]:
                self.tree_parents[r["id"]] = by_pk[lo["parent_pk"]]["id"]
        sources = {r["pk"]: r for r in t["source"]}
        refs = collections.defaultdict(list)
        for r in t["valuereference"]:
            src = sources[r["source_pk"]]
            refs[r["value_pk"]].append(
                f"{src['id']}[{r['description']}]" if r["description"] else src["id"]
            )
        self._bib = {
            r["id"]: {"key": r["name"], "title": r["description"] or ""} for r in sources.values()
        }
        valuesets = {r["pk"]: r for r in t["valueset"]}
        self.forms, by_entry = {}, collections.defaultdict(list)
        for v in t["value"]:
            vs = valuesets[v["valueset_pk"]]
            lang = by_pk[vs["language_pk"]]["id"]
            form_id = f"{lang}-{v['pk']}"
            self.forms[form_id] = {
                "ID": form_id,
                "Language_ID": lang,
                "Value": v["name"],
                "Description": v["description"] or "",
                "Doubt": "false",
                "Loan": "",
                "Source": ";".join(refs[v["pk"]]),
            }
            by_entry[vs["parameter_pk"]].append(form_id)
        self.sets, self.cognates = {}, collections.defaultdict(list)
        for p in t["parameter"]:
            forms = by_entry.get(p["pk"], [])
            protos = [f for f in forms if self.is_proto(f)]
            top = min(
                protos, key=lambda f: self.languages[self.language_of(f)]["level"], default=None
            )
            self.sets[p["id"]] = {
                "ID": p["id"],
                "Form_ID": top,
                "Doubt": "false",
                "Name": p["name"],
            }
            self.cognates[p["id"]] = [
                {"ID": f"{p['id']}-{f}", "Form_ID": f, "Doubt": "false"} for f in forms
            ]
        self.cf, self.cf_items, self.borrowings = {}, {}, []
        self.loan_members, self.doubted = {}, set()

    @property
    def bib(self):
        return self._bib

    @cached_property
    def tree(self):
        nodes = set(self.tree_parents) | set(self.tree_parents.values())
        (root,) = nodes - set(self.tree_parents)
        return Tree(self.tree_parents, root, self.levels(), {}, self.languages)


class CsdConverter(Converter):
    NAME = NAME
    LABEL = "CSD"
    EDITORS = "agent:csd-editors"
    EDITORS_NAME = "Rankin, Carter, Jones, Koontz, Rood and Hartmann (the CSD's editors)"
    IMPLIED = "implied by the CSD's tree"
    YEAR = 2015


def main(argv=None):
    main_for(CsdConverter, CsdRelease(), argv, tables=("csd.sql.gz",))


if __name__ == "__main__":
    main()
