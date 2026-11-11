"""Comparative dictionaries in pyetymdict's CLDF model to assertions: what they share.

pyetymdict (Forkel, lexibank) publishes a comparative dictionary as a CLDF Wordlist: a
cognate set names its reconstruction, a form of a proto-language, and its members;
`cf.csv` groups forms that are not a cognate set (loans, look-alikes, forms compared with
a set) through `cfitems.csv`; `borrowings.csv` marks forms as borrowed; `media.csv` holds
a Newick tree of the source's subgroups. The ACD, the MCD and TLOPO are published this
way; each has a module here naming its release and agents and adding what only it has.

A cognate set is the source's editors' claim that its forms share an origin; they endorse
it, hedged at "possible" where a membership carries the source's Doubt flag. A set's own
Doubt flag is its reconstruction's certainty. A cf group converts by the source's kind map (`etc/kinds.json`): its stance,
its hedge on every member, and whether its members are the borrowings that name it. A
group given beside a cognate set names that set (`of_set`). A borrowing's donor is not
named by any of these sources.

Links are compiled by the source's tree rule, crediting the source, never stated
(`compiled`). The nested rule (`Converter.compile`): each form of a set, its
reconstruction included, descends (inherited) from the form of the same set at the
lowest level the tree places above it, the nearest one in the set where several share
that level: the nearest before it where the source gives a reconstruction before its
reflexes (TLOPO lists POc *qa-Rumaq "at home", and then its reflexes, after POc *Rumaq),
the nearest after it where it gives the reflexes first (the MCD,
`RECONSTRUCTION_FIRST`). A second reconstruction at a level its set already reconstructs gets
no parent: the source does not say how it relates to the first. Forms flagged as loans,
and forms with no form of their set above them, get no link. The ACD keeps its own rule.

The tree is the source's Newick. Where it stops short (TLOPO's covers Oceanic, the
MCD's Micronesian), `etc/tree.json` extends it: the levels above the tree's root, and
the node each language hangs from that is not a node itself. `python -m
etymology.convert.etymdict place <source>` derives that file from the curated
`etc/tree.reviewed.json` and Glottolog's classification in the catalog bundle.
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

SUBSCRIPT = re.compile(r"[₀-₉]+$")
BIB_ENTRY = re.compile(r"@\w+\{([^,]+),(.*?)\n\}", re.DOTALL)
BIB_FIELD = re.compile(r"(\w+) = \{(.*)\},?$", re.MULTILINE)
SOURCE_REF = re.compile(r"^([^\[]+)(?:\[(.*)\])?$")
TREE = "media.csv#tree"
TABLES = (
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


def read_table(path):
    with open(path, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def newick(text):
    """The parent of each node of a Newick tree, and its root. A node's [comment] is
    skipped, and a node with no label is named by its position (`_<n>`)."""
    parents, stack, label, closed, comment, unnamed = {}, [[]], "", None, False, 0
    for ch in text.strip().rstrip(";") + ",":
        if comment:
            comment = ch != "]"
        elif ch == "[":
            comment = True
        elif ch == "(":
            stack.append([])
        elif ch in ",)":
            node = label.strip()
            if not node:
                node, unnamed = f"_{unnamed}", unnamed + 1
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
    """A source's tree of subgroups. `levels` are the nodes at which the source
    reconstructs; a language that is not a node hangs from the node `place` gives it,
    which may itself be a placed language (a proto-language the tree leaves out)."""

    def __init__(self, parents, root, levels, place, languages=None):
        self.parents, self.root = parents, root
        self.languages = languages or {}
        self.nodes = set(parents) | {root}
        self.levels = levels
        self.place = place

    def parent(self, node):
        """A node's parent in the tree, or the node a placed language hangs from."""
        return self.parents.get(node) if node in self.nodes else self.place.get(node)

    def above(self, language):
        """The levels strictly above a language, lowest first; none for a language the
        tree does not place."""
        found, node = [], self.parent(language)
        while node is not None:
            if node in self.levels:
                found.append(node)
            node = self.parent(node)
        return found

    def lowest_above(self, languages):
        """The lowest level above every one of `languages`, or None."""
        lists = [self.above(lang) for lang in languages]
        common = set(lists[0]).intersection(*lists[1:])
        return next((n for n in lists[0] if n in common), None)


class Release:
    """A pyetymdict release's tables."""

    def __init__(self, name, cldf):
        self.name, self.cldf = name, cldf
        self.languages = {r["ID"]: r for r in self.table("languages.csv")}
        self.forms = {r["ID"]: r for r in self.table("forms.csv")}
        self.sets = {r["ID"]: r for r in self.table("cognatesets.csv")}
        self.cognates = collections.defaultdict(list)
        for r in self.table("cognates.csv"):
            self.cognates[r["Cognateset_ID"]].append(r)
        self.cf = {r["ID"]: r for r in self.table("cf.csv")}
        self.cf_items = collections.defaultdict(list)
        for r in self.table("cfitems.csv"):
            self.cf_items[r["Cfset_ID"]].append(r)
        self.doubted = {s["Form_ID"] for s in self.sets.values() if s["Doubt"] == "true"}
        self.borrowings = self.table("borrowings.csv") if self.has("borrowings.csv") else []
        self.loan_members = collections.defaultdict(list)
        for r in self.borrowings:
            self.loan_members[r["Cfset_ID"]].append(r["Target_Form_ID"])

    def has(self, name):
        return (self.cldf / name).exists()

    def table(self, name):
        return read_table(self.cldf / name)

    @cached_property
    def bib(self):
        entries = {}
        if self.has("sources.bib"):
            for key, body in BIB_ENTRY.findall((self.cldf / "sources.bib").read_text()):
                entries[key] = dict(BIB_FIELD.findall(body))
        return entries

    def newick_text(self):
        (media,) = [m for m in self.table("media.csv") if m["ID"] == "tree"]
        url = media["Download_URL"]
        if url.startswith("data:"):
            return base64.b64decode(url.split(",", 1)[1]).decode()
        return (self.cldf / url).read_text(encoding="utf-8")

    def is_proto(self, form_id):
        return self.languages[self.forms[form_id]["Language_ID"]]["Is_Proto"] == "true"

    def language_of(self, form_id):
        return self.forms[form_id]["Language_ID"]

    def language_of_set(self, set_id):
        return self.language_of(self.sets[set_id]["Form_ID"])

    def members(self, set_id):
        """A cognate set's rows other than its reconstruction's."""
        s = self.sets[set_id]
        return [r for r in self.cognates[set_id] if r["Form_ID"] != s["Form_ID"]]

    def levels(self):
        """The languages at which the source reconstructs a form in a cognate set."""
        return {
            self.language_of(r["Form_ID"])
            for rows in self.cognates.values()
            for r in rows
            if self.is_proto(r["Form_ID"])
        } | {self.language_of_set(s) for s, row in self.sets.items() if row["Form_ID"]}

    @cached_property
    def tree(self):
        """The source's tree, extended by `etc/tree.json`."""
        parents, root = newick(self.newick_text())
        extension = json.loads((SOURCES / self.name / "etc/tree.json").read_text())
        if extension.get("above"):
            upper, upper_root = newick(extension["above"])
            if root not in upper:
                raise ValueError("etc/tree.json: the levels above do not hold the tree's root")
            parents, root = {**parents, **upper}, upper_root
        place = {k: v["node"] for k, v in extension["place"].items() if v["node"]}
        return Tree(parents, root, self.levels(), place, self.languages)


def mapped_categories(name):
    """A kind map's mapped kinds, by table and category."""
    found = collections.defaultdict(dict)
    for k in json.loads((SOURCES / name / "etc/kinds.json").read_text())["kinds"]:
        if k["status"] == "mapped":
            found[k["where"]][k["category"]] = k
    return found


class Converter:
    """A pyetymdict release to assertions. A source's subclass names it (`NAME`, and
    `LABEL` as readers know it), its editors, the words a compiled link shows, and its
    tree rule where it keeps its own."""

    NAME = LABEL = EDITORS = EDITORS_NAME = IMPLIED = YEAR = None
    RULE = "agent:wety-etymdict-tree@1"
    RULE_NAME = (
        "wety's nested tree rule for pyetymdict sources, v1: each form of a cognate set "
        "descends from the form of its set at the lowest level the source's tree places "
        "above it"
    )
    SET_CATEGORY = "cognate set"
    RECONSTRUCTION_FIRST = True

    def __init__(self, release, extraction):
        self.r = release
        self.lang = Languages(self.NAME)
        self.src = source(self.NAME)
        self.out = A.Writer()
        self.extraction = extraction["id"]
        self.kinds = mapped_categories(self.NAME)
        self.counts = collections.Counter()
        self.release_work = f"work:{self.NAME}:release"
        self.in_sets = collections.defaultdict(list)
        for o in (self.src, extraction, agent(self.EDITORS, "editors", self.EDITORS_NAME)):
            self.out.add(o)

    def convert(self):
        for set_id in self.r.sets:
            self.cognate_set(set_id)
        self.alternatives()
        for cf_id in self.r.cf:
            self.convert_cf(cf_id)
        for b in self.r.borrowings:
            self.borrowing(b)
        self.out.add(agent(self.RULE, "rule", self.RULE_NAME))
        cite_as = profile(self.NAME)["cite_as"]
        self.out.add(work(self.release_work, cite_as, source=self.src["id"], year=self.YEAR))
        self.parents = collections.defaultdict(set)
        self.compile()

    def alternatives(self):
        """Alternative sets the source gives; none unless a subclass reads them."""

    def record(self, form_id):
        f = self.r.forms[form_id]
        language = self.r.languages[f["Language_ID"]]
        proto = language["Is_Proto"] == "true"
        rec = {
            "type": "record",
            "id": f"{self.NAME}:{form_id}",
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
        elif form_id in self.r.doubted:
            rec["certainty"] = {"level": "possible", "words": "Doubt (reconstruction)"}
        return self.out.add(rec)

    def work(self, key):
        fields = self.r.bib.get(key, {})
        citation = fields.get("key", key)
        if fields.get("title"):
            citation += f". {fields['title']}"
        w = work(f"work:{self.NAME}:{key}", citation)
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

    def add_assertion(self, target, stance, locator, holder=None, asserter=None, **fields):
        a = A.assertion(
            self.src["id"],
            locator,
            target,
            stance,
            asserter=asserter or self.EDITORS,
            holder=holder or self.EDITORS,
            extraction=self.extraction,
            **fields,
        )
        return self.out.add(a)

    def set_category(self, set_id):
        return self.SET_CATEGORY

    def set_extension(self, set_id, rows):
        """Fields only this source gives a cognate set, under `x.<source>`."""
        return {}

    def cognate_set(self, set_id):
        s = self.r.sets[set_id]
        rows = self.r.members(set_id)
        if not s["Form_ID"] and not rows:
            self.counts["sets with no forms: not converted"] += 1
            return None
        cs = {
            "type": "cognate_set",
            "id": f"{self.NAME}:{set_id}",
            "source": self.src["id"],
            "reconstruction": self.record(s["Form_ID"]) if s["Form_ID"] else None,
            "members": [self.record(r["Form_ID"]) for r in rows],
            "category": self.set_category(set_id),
        }
        ext = self.set_extension(set_id, rows)
        if ext:
            cs["x"] = {self.NAME: ext}
        self.out.add(cs)
        hedges = [
            {"level": "possible", "words": "Doubt (membership)", "member": i}
            for i, r in enumerate(rows)
            if r["Doubt"] == "true"
        ]
        return self.add_assertion(
            cs["id"],
            "endorses",
            f"cognatesets.csv#{set_id}",
            hedges=hedges,
            credits=self.data_credits([r["Form_ID"] for r in rows]),
            in_sets=sorted(set(self.in_sets[set_id])),
        )

    def cf_extension(self, c):
        """Fields only this source gives a cf group, under `x.<source>`."""
        return {}

    def convert_cf(self, cf_id):
        """A cf.csv group, by its category in the kind map."""
        c = self.r.cf[cf_id]
        kind = self.kinds["cf.csv"].get(c["Category"])
        if kind is None:
            raise ValueError(f"{cf_id}: category {c['Category']!r} is not mapped in etc/kinds.json")
        if kind.get("members") == "borrowings.csv":
            form_ids = self.r.loan_members[cf_id]
        else:
            form_ids = [i["Form_ID"] for i in self.r.cf_items[cf_id]]
        cs = {
            "type": "cognate_set",
            "id": f"{self.NAME}:{cf_id}",
            "source": self.src["id"],
            "reconstruction": None,
            "members": [self.record(i) for i in form_ids],
            "category": kind.get("label", c["Category"]),
        }
        if c["Cognateset_ID"]:
            cs["of_set"] = f"{self.NAME}:{c['Cognateset_ID']}"
        ext = self.cf_extension(c)
        if ext:
            cs["x"] = {self.NAME: ext}
        self.out.add(cs)
        hedges = []
        if kind.get("hedge"):
            hedges = [{**kind["hedge"], "member": i} for i in range(len(form_ids))]
        quote = c[kind["quote"]] if kind.get("quote") else ""
        self.add_assertion(
            cs["id"],
            kind.get("stance", "endorses"),
            f"cf.csv#{cf_id}",
            hedges=hedges,
            credits=self.data_credits(form_ids),
            quote=quote if quote and quote != c["ID"] else None,
        )
        return c, cs

    def borrowing(self, b):
        """A borrowings.csv row: the form is borrowed, from a donor the source does not name."""
        target = self.record(b["Target_Form_ID"])
        if b["Source_Form_ID"]:
            raise ValueError(f"borrowings.csv#{b['ID']} names a donor form: convert it")
        donor = self.out.add(
            {
                "type": "record",
                "id": f"{target}:donor",
                "source": self.src["id"],
                "language": {
                    "label": f"a donor the {self.LABEL} does not name",
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
            holder=self.RULE,
            asserter=self.RULE,
            hedges=list(hedges),
            credits=[
                {
                    "work": self.release_work,
                    "role": "follows",
                    "scope": "entry",
                    "locator": credit_locator,
                }
            ],
            compiled={"from": TREE, "words": self.IMPLIED},
        )

    def compile(self):
        """The nested rule (module docstring): each form of a set descends from the form
        of its set at the lowest level the tree places above it."""
        tree = self.r.tree
        for set_id, s in self.r.sets.items():
            rows = {r["Form_ID"]: r for r in self.r.cognates[set_id]}
            forms = [f for f in dict.fromkeys([s["Form_ID"], *rows]) if f]
            protos = [(i, f) for i, f in enumerate(forms) if self.r.is_proto(f)]
            first_at = {}
            for _, f in protos:
                first_at.setdefault(self.r.language_of(f), f)
            for i, f in enumerate(forms):
                language = self.r.language_of(f)
                if self.r.forms[f]["Loan"] == "true":
                    self.counts["forms flagged as loans: no link"] += 1
                    continue
                if self.r.is_proto(f) and first_at[language] != f:
                    self.counts["a second reconstruction at a level its set has: no link"] += 1
                    continue
                parent = None
                for level in tree.above(language):
                    found = [(j, g) for j, g in protos if self.r.language_of(g) == level]
                    if found and self.RECONSTRUCTION_FIRST:
                        before = [g for j, g in found if j < i]
                        parent = before[-1] if before else found[0][1]
                    elif found:
                        after = [g for j, g in found if j > i]
                        parent = after[0] if after else found[-1][1]
                    if parent:
                        break
                if parent is None:
                    what = "reconstructions" if f == s["Form_ID"] else "members"
                    self.counts[f"{what} with no form of their set above them: no link"] += 1
                    continue
                link = {
                    "child": f"{self.NAME}:{f}",
                    "parents": [f"{self.NAME}:{parent}"],
                    "relation": "derivation",
                    "mode": "inherited",
                }
                row = rows.get(f)
                hedges = []
                if row and f != s["Form_ID"] and row["Doubt"] == "true":
                    hedges.append({"level": "possible", "words": "Doubt (membership)", "link": 0})
                self.compiled_link(
                    "form < the form of its set at the lowest level above it (inherited)",
                    link,
                    f"cognates.csv#{row['ID']}" if row else f"cognatesets.csv#{set_id}",
                    f"cognatesets.csv#{set_id}",
                    hedges,
                )


def main_for(converter, release, argv=None, tables=TABLES):
    """The command line of a source's converter: its release's `tables` and its curation
    files are the extraction's inputs."""
    p = argparse.ArgumentParser(description=f"{converter.LABEL}'s CLDF release to assertions")
    p.add_argument("--out", type=Path, required=True, help="the assertions file to write")
    p.add_argument("--counts", type=Path, help="write the converter's own counts here (JSON)")
    args = p.parse_args(argv)
    etc = SOURCES / converter.NAME / "etc"
    inputs = [release.cldf / t for t in tables if (release.cldf / t).exists()]
    curation = (
        "languages.json",
        "donors.json",
        "roots.json",
        "kinds.json",
        "source.json",
        "tree.json",
    )
    inputs += [etc / f for f in curation]
    extraction = A.extraction(
        "converter",
        f"packages/etymology/convert/{converter.NAME}.py",
        [p for p in inputs if p.exists()],
    )
    c = converter(release, extraction)
    c.convert()
    n = c.out.write(args.out)
    print(f"{args.out}: {n} objects")
    if args.counts:
        args.counts.write_text(json.dumps(dict(sorted(c.counts.items())), indent=1) + "\n")
    return c


# Placing languages on a source's tree (`etc/tree.json`)


def derive_places(release, reviewed, glottolog):
    """The node each language that is not a node of the tree hangs from: the reviewed
    `place`, else the node its Group names (`groups`), else the deepest node whose
    glottocode is the language's own or one of its Glottolog ancestors'."""
    parents, root = newick(release.newick_text())
    if reviewed.get("above"):
        upper, upper_root = newick(reviewed["above"])
        parents, root = {**parents, **upper}, upper_root
    nodes = set(parents) | {root}

    def depth(node):
        n = 0
        while node in parents:
            node, n = parents[node], n + 1
        return n

    placed = {k for k, v in reviewed.get("place", {}).items() if v["node"]}
    by_glottocode = collections.defaultdict(list)
    codes = {
        **{k: v["Glottocode"] for k, v in release.languages.items()},
        **reviewed.get("glottocodes", {}),
    }
    for node in nodes:
        if codes.get(node):
            by_glottocode[codes[node]].append(node)
    place = {}
    for lid, language in sorted(release.languages.items()):
        if lid in nodes:
            continue
        row = {"name": language["Name"], "node": None, "by": None, "basis": "not placed"}
        if lid in reviewed.get("place", {}):
            row.update(reviewed["place"][lid], by="reviewed")
        elif language.get("Group") in reviewed.get("groups", {}):
            row["node"] = reviewed["groups"][language["Group"]]
            row["by"], row["basis"] = "group", f"its Group, {language['Group']}"
        else:
            g = glottolog.get(language["Glottocode"])
            lineage = (g["classification"].split("/") if g and g["classification"] else []) + [
                language["Glottocode"]
            ]
            for code in reversed(lineage):
                if by_glottocode.get(code):
                    row["node"] = max(by_glottocode[code], key=depth)
                    row["by"], row["basis"] = "glottolog", f"{code} in its Glottolog classification"
                    break
        if row["node"] and row["node"] not in nodes | placed:
            raise ValueError(f"{lid}: {row['node']} is not a node of the tree or placed")
        place[lid] = row
    return place


def place_main(argv):
    from catalogs import registry
    from workflow_tools.paths import CATALOG_PRODUCT

    p = argparse.ArgumentParser(description="Derive a source's etc/tree.json")
    p.add_argument("source")
    p.add_argument("--catalog", type=Path, default=CATALOG_PRODUCT)
    args = p.parse_args(argv)
    etc = SOURCES / args.source / "etc"
    reviewed = json.loads((etc / "tree.reviewed.json").read_text())
    release = Release(args.source, SOURCES / args.source / reviewed["cldf"])
    bundle = registry.current_bundle(args.catalog)
    glottolog = {g["glottocode"]: g for g in bundle["glottolog"]}
    place = derive_places(release, reviewed, glottolog)
    placed = collections.Counter(r["by"] if r["node"] else "not placed" for r in place.values())
    result = {
        "source": args.source,
        "catalog_generation": registry.digest(bundle),
        "reviewed": f"data/workspaces/etymology/sources/{args.source}/etc/tree.reviewed.json",
        "above": reviewed.get("above"),
        "summary": dict(sorted(placed.items())),
        "place": place,
    }
    (etc / "tree.json").write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n")
    print(json.dumps(result["summary"]))


if __name__ == "__main__":
    import sys

    if sys.argv[1:2] != ["place"]:
        raise SystemExit("usage: python -m etymology.convert.etymdict place <source>")
    place_main(sys.argv[2:])
