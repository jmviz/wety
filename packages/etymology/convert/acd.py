"""The ACD's CLDF release to assertions.

A cognate set is the ACD's (Blust's) claim that its reconstruction and members share an
origin; the ACD's editors endorse it. Its doublet sets are alternative sets that are not
exclusive (each variant is supported), its disjunct sets exclusive ones with no
preference. The `cf.csv` groups convert by the kind map (`etc/kinds.json`): a noise group
is a cognacy the ACD rejects, a near group one it holds possible, member by member.
Where Dempwolff reconstructed a noise group, the ACD reports his reconstruction, held by
him, and rejects it.

Links are not compiled here: compiling them from the ACD's tree is a wety rule (acd-2).
"""

import argparse
import collections
import csv
import json
import re
from functools import cached_property
from pathlib import Path

from etymology import assertions as A
from etymology.convert import SOURCES, Languages, agent, source, work

NAME = "acd"
CLDF = SOURCES / NAME / "raw/lexibank-acd-16085f2/cldf"
EDITORS = "agent:acd-editors"
DEMPWOLFF = "agent:dempwolff"
SUBSCRIPT = re.compile(r"[₀-₉]+$")
BIB_ENTRY = re.compile(r"@\w+\{([^,]+),(.*?)\n\}", re.DOTALL)
BIB_FIELD = re.compile(r"(\w+) = \{(.*)\},?$", re.MULTILINE)
SOURCE_REF = re.compile(r"^([^\[]+)(?:\[(.*)\])?$")


def table(name):
    with open(CLDF / name, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


class Release:
    def __init__(self):
        self.languages = {r["ID"]: r for r in table("languages.csv")}
        self.forms = {r["ID"]: r for r in table("forms.csv")}
        self.sets = {r["ID"]: r for r in table("cognatesets.csv")}
        self.cognates = collections.defaultdict(list)
        for r in table("cognates.csv"):
            self.cognates[r["Cognateset_ID"]].append(r)
        self.cf = {r["ID"]: r for r in table("cf.csv")}
        self.cf_items = collections.defaultdict(list)
        for r in table("cfitems.csv"):
            self.cf_items[r["Cfset_ID"]].append(r)
        self.doublets = collections.defaultdict(set)
        self.disjuncts = collections.defaultdict(set)
        for set_id, rows in self.cognates.items():
            for r in rows:
                if r["Doublet_Set"]:
                    self.doublets[r["Doublet_Set"]].add(set_id)
                if r["Disjunct_Set"]:
                    self.disjuncts[r["Disjunct_Set"]].add(set_id)

    @cached_property
    def bib(self):
        entries = {}
        for key, body in BIB_ENTRY.findall((CLDF / "sources.bib").read_text()):
            fields = dict(BIB_FIELD.findall(body))
            entries[key] = fields
        return entries

    def groups_of(self, set_id):
        """The doublet and disjunct groups a cognate set's reconstruction belongs to."""
        found = []
        for r in self.cognates[set_id]:
            if r["Form_ID"] == self.sets[set_id]["Form_ID"]:
                if r["Doublet_Set"]:
                    found.append(("doublet", r["Doublet_Set"]))
                if r["Disjunct_Set"]:
                    found.append(("disjunct", r["Disjunct_Set"]))
        return found


class Converter:
    def __init__(self, release, extraction):
        self.r = release
        self.lang = Languages(NAME)
        self.src = source(NAME)
        self.out = A.Writer()
        self.extraction = extraction["id"]
        for o in (
            self.src,
            extraction,
            agent(EDITORS, "editors", "Robert Blust and Stephen Trussel (the ACD's authors)"),
        ):
            self.out.add(o)

    def record(self, form_id):
        f = self.r.forms[form_id]
        language = self.r.languages[f["Language_ID"]]
        proto = language["Is_Proto"] == "true"
        rec = {
            "type": "record",
            "id": f"{NAME}:{form_id}",
            "source": self.src["id"],
            "language": self.lang(f["Language_ID"], language["Name"]),
            "form": f["Value"],
            "reconstructed": proto,
            "locator": f"forms.csv#{form_id}",
        }
        if proto and SUBSCRIPT.search(f["Value"]):
            rec["form"] = SUBSCRIPT.sub("", f["Value"])
            rec["homograph"] = SUBSCRIPT.search(f["Value"]).group()
        if f["Description"]:
            rec["gloss"] = f["Description"]
        if f["Doubt"] == "true":
            rec["certainty"] = {"level": "possible", "words": "Doubt (form)"}
        return self.out.add(rec)

    def work(self, key):
        fields = self.r.bib.get(key, {})
        citation = fields.get("key", key)
        if fields.get("title"):
            citation += f". {fields['title']}"
        w = work(f"work:{NAME}:{key}", citation)
        if fields.get("year", "").isdigit():
            w["year"] = int(fields["year"])
        return self.out.add(w)

    def data_credits(self, rows):
        """The dictionaries members' forms are taken from: credits for data, not etymology."""
        credits = {}
        for ref in sorted({s for r in rows for s in r["Source"].split(";") if s}):
            key, page = SOURCE_REF.match(ref.strip()).groups()
            credit = {"work": self.work(key.strip()), "role": "data", "scope": "inline"}
            if page:
                credit["locator"] = page
            credits[ref] = credit
        return list(credits.values())

    def cognate_set(self, set_id):
        s = self.r.sets[set_id]
        rows = [r for r in self.r.cognates[set_id] if r["Form_ID"] != s["Form_ID"]]
        members = [self.record(r["Form_ID"]) for r in rows]
        cs = {
            "type": "cognate_set",
            "id": f"{NAME}:{set_id}",
            "source": self.src["id"],
            "reconstruction": self.record(s["Form_ID"]),
            "members": members,
            "category": "cognate set" if s["Is_Main_Entry"] == "true" else "cognate set, sub-entry",
        }
        irregular = {
            kind: [i for i, r in enumerate(rows) if r[kind] == "true"]
            for kind in ("Metathesis", "Assimilation")
        }
        ext = {"etymon": s["Etymon_ID"]}
        ext.update({k.lower(): v for k, v in irregular.items() if v})
        cs["x"] = {NAME: ext}
        self.out.add(cs)
        hedges = [
            {"level": "possible", "words": "Doubt (membership)", "member": i}
            for i, r in enumerate(rows)
            if r["Doubt"] == "true"
        ]
        if s["Doubt"] == "true":
            hedges.append({"level": "possible", "words": "Doubt (reconstruction)"})
        forms = [self.r.forms[r["Form_ID"]] for r in rows]
        return self.add_assertion(
            cs["id"],
            "endorses",
            f"cognatesets.csv#{set_id}",
            hedges=hedges,
            credits=self.data_credits(forms),
        )

    def add_assertion(self, target, stance, locator, holder=EDITORS, **fields):
        a = A.assertion(
            self.src["id"],
            locator,
            target,
            stance,
            asserter=EDITORS,
            holder=holder,
            extraction=self.extraction,
            **fields,
        )
        return self.out.add(a)

    def group(self, kind, group_id):
        """A doublet or disjunct group as an alternative set over its cognate sets."""
        found = self.r.doublets if kind == "doublet" else self.r.disjuncts
        members = sorted(f"{NAME}:{m}" for m in found[group_id])
        for m in found[group_id]:
            if f"{NAME}:{m}" not in self.out.objects:
                self.cognate_set(m)
        if len(members) < 2:
            return None
        alt = A.alternative_set(members, exclusive=kind == "disjunct")
        self.out.add(alt)
        self.add_assertion(
            alt["id"], "endorses", f"cognates.csv#{kind.capitalize()}_Set={group_id}"
        )
        return alt["id"]

    def convert_set(self, set_id):
        self.cognate_set(set_id)
        for kind, group_id in self.r.groups_of(set_id):
            alt = self.group(kind, group_id)
            if alt is None:
                continue
            for m in self.out.objects[alt]["members"]:
                self.mark_in_set(m, alt)

    def mark_in_set(self, set_id, alt):
        for o in list(self.out.objects.values()):
            if o["type"] == "assertion" and o["target"] == set_id and o["stance"] == "endorses":
                o.setdefault("in_sets", [])
                if alt not in o["in_sets"]:
                    o["in_sets"].append(alt)
                    o["in_sets"].sort()

    def convert_cf(self, cf_id):
        """A noise or near group from cf.csv."""
        c = self.r.cf[cf_id]
        items = self.r.cf_items[cf_id]
        members = [self.record(i["Form_ID"]) for i in items]
        cs = {
            "type": "cognate_set",
            "id": f"{NAME}:{cf_id}",
            "source": self.src["id"],
            "reconstruction": None,
            "members": members,
            "category": c["Category"],
        }
        if c["Description"]:
            cs["x"] = {NAME: {"gloss": c["Description"]}}
        locator = f"cf.csv#{cf_id}"
        if c["Category"] == "noise":
            self.out.add(cs)
            self.add_assertion(cs["id"], "rejects", locator)
            if c["Dempwolff_Etymology"]:
                self.dempwolff(c, cs)
        elif c["Category"] == "near":
            self.out.add(cs)
            hedges = [
                {"level": "possible", "words": "near (the ACD's category)", "member": i}
                for i in range(len(members))
            ]
            self.add_assertion(cs["id"], "endorses", locator, hedges=hedges)
        else:
            raise ValueError(f"{cf_id}: category {c['Category']!r} is not converted yet")

    def dempwolff(self, c, noise):
        """Dempwolff's reconstruction of a noise group, reported by the ACD and rejected."""
        self.out.add(agent(DEMPWOLFF, "person", "Otto Dempwolff"))
        rec = {
            "type": "record",
            "id": f"{NAME}:{c['ID']}:dempwolff-form",
            "source": self.src["id"],
            "language": {
                "label": "Uraustronesisch (Dempwolff)",
                "code": None,
                "relation": "unresolved",
            },
            "form": c["Dempwolff_Etymology"],
            "reconstructed": True,
            "locator": f"cf.csv#{c['ID']}",
        }
        if c["Description"]:
            rec["gloss"] = c["Description"]
        self.out.add(rec)
        cs = {
            "type": "cognate_set",
            "id": f"{NAME}:{c['ID']}:dempwolff",
            "source": self.src["id"],
            "reconstruction": rec["id"],
            "members": noise["members"],
            "category": "Dempwolff_Etymology",
        }
        self.out.add(cs)
        credit = {"work": self.work("dempwolff193438"), "role": "against", "scope": "inline"}
        self.add_assertion(
            cs["id"],
            "rejects",
            f"cf.csv#{c['ID']}:Dempwolff_Etymology",
            holder=DEMPWOLFF,
            credits=[credit],
        )


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument(
        "sample", type=Path, help="JSON with `sets` (cognatesets.csv ids) and `cf` (cf.csv ids)"
    )
    p.add_argument("--out", type=Path, required=True)
    args = p.parse_args(argv)
    sample = json.loads(args.sample.read_text())
    extraction = A.extraction(
        "converter",
        "packages/etymology/convert/acd.py",
        [
            CLDF / t
            for t in ("cognatesets.csv", "cognates.csv", "forms.csv", "cf.csv", "cfitems.csv")
        ]
        + [SOURCES / NAME / "etc/languages.json", args.sample],
    )
    c = Converter(Release(), extraction)
    for s in sample.get("sets", []):
        c.convert_set(s)
    for cf_id in sample.get("cf", []):
        c.convert_cf(cf_id)
    n = c.out.write(args.out)
    print(f"{args.out}: {n} objects")


if __name__ == "__main__":
    main()
