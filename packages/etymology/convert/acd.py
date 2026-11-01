"""The ACD's CLDF release to assertions: the whole release, and the links its tree implies.

A cognate set is the ACD's (Blust's) claim that its reconstruction and members share an
origin; the ACD's editors endorse it. Its doublet groups are alternative sets that are
not exclusive (each variant is supported), its disjunct groups exclusive ones with no
preference; a set belongs to a group when its reconstruction or one of its members
carries the group. The `cf.csv` groups convert by the kind map (`etc/kinds.json`): a
noise group is a cognacy the ACD rejects, a near group one it holds possible, member by
member; root, also and loan groups are endorsed under the ACD's own category. Each row
of `borrowings.csv` is a borrowing from a donor the ACD does not name. Where Dempwolff
reconstructed a cf group, the ACD reports his reconstruction, held by him, and rejects it.

Links are compiled by a wety rule (`RULE`), crediting the ACD, never stated:

- each member of a cognate set descends (inherited) from the set's reconstruction, where
  the ACD's tree places the member's language below the reconstruction's level; members
  flagged as loans, and members the tree does not place below, get no link;
- a sub-entry's reconstruction descends by unstated steps (ultimate) from its etymon's
  main entry, where the tree places it lower; at the same level the tree implies nothing;
- a near group whose members share no named ancestor gets an unnamed ancestor (form
  null) at the lowest level above all its members, and each member descends from it by
  unstated steps, at "possible".

The tree's levels are its nodes at which the ACD reconstructs something: Formosan is a
node but not a level, so Formosan languages sit directly below PAN.
"""

import argparse
import base64
import collections
import csv
import json
import re
from functools import cached_property
from pathlib import Path

from etymology import assertions as A
from etymology.convert import SOURCES, Languages, agent, profile, source, work

NAME = "acd"
CLDF = SOURCES / NAME / "raw/lexibank-acd-16085f2/cldf"
KINDS = SOURCES / NAME / "etc/kinds.json"
EDITORS = "agent:acd-editors"
DEMPWOLFF = "agent:dempwolff"
RULE = "agent:wety-acd-tree@1"
RELEASE = f"work:{NAME}:release"
TREE = "media.csv#tree"
IMPLIED = "implied by the ACD's tree"
NEAR = "near (the ACD's category)"
SUBSCRIPT = re.compile(r"[₀-₉]+$")
BIB_ENTRY = re.compile(r"@\w+\{([^,]+),(.*?)\n\}", re.DOTALL)
BIB_FIELD = re.compile(r"(\w+) = \{(.*)\},?$", re.MULTILINE)
SOURCE_REF = re.compile(r"^([^\[]+)(?:\[(.*)\])?$")
INPUTS = (
    "languages.csv",
    "media.csv",
    "forms.csv",
    "cognatesets.csv",
    "cognates.csv",
    "cf.csv",
    "cfitems.csv",
    "borrowings.csv",
    "sources.bib",
)


def table(name):
    with open(CLDF / name, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def newick(text):
    """The parent of each node of a Newick tree whose nodes all carry labels, and its root."""
    parents, stack, label, closed = {}, [[]], "", None
    for ch in text.strip().rstrip(";") + ",":
        if ch == "(":
            stack.append([])
        elif ch in ",)":
            node = label.strip()
            if not node:
                raise ValueError("a Newick node without a label")
            for child in closed or ():
                parents[child] = node
            stack[-1].append(node)
            label, closed = "", None
            if ch == ")":
                closed = stack.pop()
        else:
            label += ch
    (root,) = stack[0]
    return parents, root


class Tree:
    """The ACD's tree of reconstruction levels; a language hangs from the node of its Group."""

    def __init__(self, text, languages, levels):
        self.parents, self.root = newick(text)
        self.nodes = set(self.parents) | {self.root}
        self.languages = languages
        self.node_of_group = {languages[n]["Abbr"]: n for n in self.nodes}
        self.levels = levels

    def above(self, language):
        """The levels strictly above a language, lowest first."""
        if language in self.nodes:
            node = self.parents.get(language)
        else:
            node = self.node_of_group[self.languages[language]["Group"]]
        found = []
        while node is not None:
            if node in self.levels:
                found.append(node)
            node = self.parents.get(node)
        return found

    def lowest_above(self, languages):
        """The lowest level above every one of `languages`, or None."""
        lists = [self.above(lang) for lang in languages]
        common = set(lists[0]).intersection(*lists[1:])
        return next((n for n in lists[0] if n in common), None)


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
        self.borrowings = table("borrowings.csv")
        self.loan_members = collections.defaultdict(list)
        for r in self.borrowings:
            self.loan_members[r["Cfset_ID"]].append(r["Target_Form_ID"])
        self.groups = {kind: collections.defaultdict(set) for kind in ("doublet", "disjunct")}
        for set_id, rows in self.cognates.items():
            for r in rows:
                for kind in self.groups:
                    if r[f"{kind.capitalize()}_Set"]:
                        self.groups[kind][r[f"{kind.capitalize()}_Set"]].add(set_id)
        self.main_entry = {
            s["Etymon_ID"]: set_id
            for set_id, s in self.sets.items()
            if s["Is_Main_Entry"] == "true"
        }
        levels = {self.forms[s["Form_ID"]]["Language_ID"] for s in self.sets.values()}
        (media,) = table("media.csv")
        newick_text = base64.b64decode(media["Download_URL"].split(",", 1)[1]).decode()
        self.tree = Tree(newick_text, self.languages, levels)

    @cached_property
    def bib(self):
        entries = {}
        for key, body in BIB_ENTRY.findall((CLDF / "sources.bib").read_text()):
            fields = dict(BIB_FIELD.findall(body))
            entries[key] = fields
        return entries

    def language_of_set(self, set_id):
        return self.forms[self.sets[set_id]["Form_ID"]]["Language_ID"]

    def members(self, set_id):
        """A cognate set's rows other than its reconstruction's."""
        s = self.sets[set_id]
        return [r for r in self.cognates[set_id] if r["Form_ID"] != s["Form_ID"]]


def mapped_categories():
    """The kind map's mapped categories, by the table they are found in."""
    found = collections.defaultdict(set)
    for k in json.loads(KINDS.read_text())["kinds"]:
        if k["status"] == "mapped":
            found[k["where"]].add(k["category"])
    return found


class Converter:
    def __init__(self, release, extraction):
        self.r = release
        self.lang = Languages(NAME)
        self.src = source(NAME)
        self.out = A.Writer()
        self.extraction = extraction["id"]
        self.kinds = mapped_categories()
        self.counts = collections.Counter()
        for o in (
            self.src,
            extraction,
            agent(EDITORS, "editors", "Robert Blust and Stephen Trussel (the ACD's authors)"),
        ):
            self.out.add(o)
        self.in_sets = collections.defaultdict(list)
        self.alternatives = []
        for kind, groups in self.r.groups.items():
            for group_id, set_ids in sorted(groups.items()):
                if len(set_ids) < 2:
                    self.counts[f"{kind} groups naming one cognate set"] += 1
                    continue
                alt = A.alternative_set(
                    sorted(f"{NAME}:{m}" for m in set_ids), exclusive=kind == "disjunct"
                )
                if any(alt["id"] == other["id"] for _, _, other in self.alternatives):
                    self.counts[f"{kind} groups with the same cognate sets as another"] += 1
                self.alternatives.append((kind, group_id, alt))
                for m in set_ids:
                    self.in_sets[m].append(alt["id"])

    def convert(self):
        for set_id in self.r.sets:
            self.cognate_set(set_id)
        for kind, group_id, alt in self.alternatives:
            self.out.add(alt)
            self.add_assertion(
                alt["id"], "endorses", f"cognates.csv#{kind.capitalize()}_Set={group_id}"
            )
        for cf_id in self.r.cf:
            self.convert_cf(cf_id)
        for b in self.r.borrowings:
            self.borrowing(b)
        self.compile()

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

    def data_credits(self, form_ids):
        """The dictionaries members' forms are taken from: credits for data, not etymology."""
        refs = {s for i in form_ids for s in self.r.forms[i]["Source"].split(";") if s}
        credits = []
        for ref in sorted(refs):
            key, page = SOURCE_REF.match(ref.strip()).groups()
            credit = {"work": self.work(key.strip()), "role": "data", "scope": "inline"}
            if page:
                credit["locator"] = page
            credits.append(credit)
        return credits

    def add_assertion(self, target, stance, locator, holder=EDITORS, asserter=EDITORS, **fields):
        a = A.assertion(
            self.src["id"],
            locator,
            target,
            stance,
            asserter=asserter,
            holder=holder,
            extraction=self.extraction,
            **fields,
        )
        return self.out.add(a)

    def cognate_set(self, set_id):
        s = self.r.sets[set_id]
        main = s["Is_Main_Entry"] == "true"
        category = "cognate set" if main else "cognate set, sub-entry"
        if not main and "cognate set, not a main entry" not in self.kinds["cognatesets.csv"]:
            raise ValueError("the kind map no longer maps sub-entries")
        rows = self.r.members(set_id)
        cs = {
            "type": "cognate_set",
            "id": f"{NAME}:{set_id}",
            "source": self.src["id"],
            "reconstruction": self.record(s["Form_ID"]),
            "members": [self.record(r["Form_ID"]) for r in rows],
            "category": category,
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
        return self.add_assertion(
            cs["id"],
            "endorses",
            f"cognatesets.csv#{set_id}",
            hedges=hedges,
            credits=self.data_credits([r["Form_ID"] for r in rows]),
            in_sets=sorted(set(self.in_sets[set_id])),
        )

    def convert_cf(self, cf_id):
        """A cf.csv group, by its category in the kind map."""
        c = self.r.cf[cf_id]
        category = c["Category"]
        if category not in self.kinds["cf.csv"]:
            raise ValueError(f"{cf_id}: category {category!r} is not mapped in {KINDS.name}")
        if category == "loan":
            form_ids = self.r.loan_members[cf_id]
        else:
            form_ids = [i["Form_ID"] for i in self.r.cf_items[cf_id]]
        cs = {
            "type": "cognate_set",
            "id": f"{NAME}:{cf_id}",
            "source": self.src["id"],
            "reconstruction": None,
            "members": [self.record(i) for i in form_ids],
            "category": category,
        }
        ext = {}
        if c["Description"]:
            ext["gloss"] = c["Description"]
        if category in ("root", "loan") and c["Name"]:
            ext["name"] = c["Name"]
        if category == "also":
            ext["of_set"] = f"{NAME}:{c['Cognateset_ID']}"
        if ext:
            cs["x"] = {NAME: ext}
        self.out.add(cs)
        stance, hedges = "endorses", []
        if category == "noise":
            stance = "rejects"
        elif category == "near":
            hedges = [
                {"level": "possible", "words": NEAR, "member": i} for i in range(len(form_ids))
            ]
        self.add_assertion(
            cs["id"],
            stance,
            f"cf.csv#{cf_id}",
            hedges=hedges,
            credits=self.data_credits(form_ids),
        )
        if c["Dempwolff_Etymology"]:
            self.dempwolff(c, cs)

    def dempwolff(self, c, group):
        """Dempwolff's reconstruction of a cf group, reported by the ACD and rejected."""
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
            "members": group["members"],
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

    def borrowing(self, b):
        """A borrowings.csv row: the form is borrowed, from a donor the ACD does not name."""
        target = self.record(b["Target_Form_ID"])
        if b["Source_Form_ID"]:
            raise ValueError(f"borrowings.csv#{b['ID']} names a donor form: convert it")
        donor = self.out.add(
            {
                "type": "record",
                "id": f"{target}:donor",
                "source": self.src["id"],
                "language": {
                    "label": "a donor the ACD does not name",
                    "code": None,
                    "relation": "unresolved",
                },
                "form": None,
                "reconstructed": False,
            }
        )
        link = {"child": target, "parents": [donor], "relation": "derivation", "mode": "borrowed"}
        h = A.hypothesis(target, [link])
        self.out.add(h)
        self.add_assertion(
            h["id"],
            "endorses",
            f"borrowings.csv#{b['ID']}",
            credits=self.data_credits([b["Target_Form_ID"]]),
            quote=b["Comment"] or None,
        )

    def compiled_link(self, what, link, locator, credit_locator, hedges=()):
        h = A.hypothesis(link["child"], [link])
        self.out.add(h)
        self.counts[f"compiled: {what}"] += 1
        self.parents[link["child"]].update(link["parents"])
        return self.add_assertion(
            h["id"],
            "endorses",
            locator,
            holder=RULE,
            asserter=RULE,
            hedges=list(hedges),
            credits=[
                {"work": RELEASE, "role": "follows", "scope": "entry", "locator": credit_locator}
            ],
            compiled={"from": TREE, "words": IMPLIED},
        )

    def compile(self):
        """The links the ACD's tree implies, held by the compile rule."""
        p = profile(NAME)
        self.out.add(
            agent(
                RULE,
                "rule",
                "wety's ACD tree rule, v1: links compiled from the ACD's cognate sets, "
                "etyma and tree",
            )
        )
        self.out.add(work(RELEASE, p["cite_as"], source=self.src["id"], year=2020))
        self.parents = collections.defaultdict(set)
        tree = self.r.tree
        for set_id, s in self.r.sets.items():
            reconstruction = f"{NAME}:{s['Form_ID']}"
            level = self.r.language_of_set(set_id)
            for row in self.r.members(set_id):
                f = self.r.forms[row["Form_ID"]]
                if f["Loan"] == "true":
                    self.counts["members flagged as loans: no link"] += 1
                    continue
                if level not in tree.above(f["Language_ID"]):
                    self.counts["members the tree does not place below: no link"] += 1
                    continue
                link = {
                    "child": f"{NAME}:{row['Form_ID']}",
                    "parents": [reconstruction],
                    "relation": "derivation",
                    "mode": "inherited",
                }
                hedges = []
                if row["Doubt"] == "true":
                    hedges.append({"level": "possible", "words": "Doubt (membership)", "link": 0})
                self.compiled_link(
                    "member < its set's reconstruction (inherited)",
                    link,
                    f"cognates.csv#{row['ID']}",
                    f"cognatesets.csv#{set_id}",
                    hedges,
                )
            if s["Is_Main_Entry"] == "true":
                continue
            main = self.r.main_entry[s["Etymon_ID"]]
            main_level = self.r.language_of_set(main)
            if main_level == level:
                self.counts["sub-entries at their main entry's level: no link"] += 1
            elif main_level not in tree.above(level):
                self.counts["sub-entries the tree does not place below their main entry"] += 1
            else:
                link = {
                    "child": reconstruction,
                    "parents": [f"{NAME}:{self.r.sets[main]['Form_ID']}"],
                    "relation": "ultimate",
                }
                self.compiled_link(
                    "sub-entry < its main entry (ultimate)",
                    link,
                    f"cognatesets.csv#{set_id}",
                    f"etyma.csv#{s['Etymon_ID']}",
                )
        for cf_id, c in self.r.cf.items():
            if c["Category"] == "near":
                self.unnamed_ancestor(cf_id)

    def named_ancestors(self, record):
        seen, todo = set(), [record]
        while todo:
            for p in self.parents.get(todo.pop(), ()):
                if p not in seen:
                    seen.add(p)
                    todo.append(p)
        return seen

    def unnamed_ancestor(self, cf_id):
        """A near group's common ancestor, not named, where its members share no named one."""
        form_ids = [i["Form_ID"] for i in self.r.cf_items[cf_id]]
        members = [f"{NAME}:{i}" for i in form_ids]
        if len(set(members)) < 2:
            self.counts["near groups with one member: no ancestor"] += 1
            return
        if set.intersection(*(self.named_ancestors(m) for m in members)):
            self.counts["near groups sharing a named ancestor: no ancestor"] += 1
            return
        languages = [self.r.forms[i]["Language_ID"] for i in form_ids]
        level = self.r.tree.lowest_above(languages)
        if level is None:
            self.counts["near groups with no level above all members: no ancestor"] += 1
            return
        ancestor = self.out.add(
            {
                "type": "record",
                "id": f"{NAME}:{cf_id}:ancestor",
                "source": self.src["id"],
                "language": self.lang(level, self.r.languages[level]["Name"]),
                "form": None,
                "reconstructed": True,
                "locator": f"cf.csv#{cf_id}",
            }
        )
        self.counts["unnamed ancestors"] += 1
        for m in dict.fromkeys(members):
            link = {"child": m, "parents": [ancestor], "relation": "ultimate"}
            self.compiled_link(
                "near member < unnamed ancestor (ultimate, possible)",
                link,
                f"cf.csv#{cf_id}:{m.split(':', 1)[1]}",
                f"cf.csv#{cf_id}",
                [{"level": "possible", "words": NEAR, "link": 0}],
            )


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--out", type=Path, required=True, help="the assertions file to write")
    p.add_argument("--counts", type=Path, help="write the converter's own counts here (JSON)")
    args = p.parse_args(argv)
    extraction = A.extraction(
        "converter",
        "packages/etymology/convert/acd.py",
        [CLDF / t for t in INPUTS]
        + [SOURCES / NAME / "etc/languages.json", KINDS, SOURCES / NAME / "etc/source.json"],
    )
    c = Converter(Release(), extraction)
    c.convert()
    n = c.out.write(args.out)
    print(f"{args.out}: {n} objects")
    if args.counts:
        args.counts.write_text(json.dumps(dict(sorted(c.counts.items())), indent=1) + "\n")


if __name__ == "__main__":
    main()
