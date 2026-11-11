"""The ACD's CLDF release to assertions: the whole release, and the links its tree implies.

What pyetymdict sources share is in `etymdict`. The ACD adds: its doublet groups are
alternative sets that are not exclusive (each variant is supported), its disjunct groups
exclusive ones with no preference; a set belongs to a group when its reconstruction or
one of its members carries the group. Its cf groups convert by the kind map
(`etc/kinds.json`): a noise group is a cognacy the ACD rejects, a near group one it holds
possible, member by member; root, also and loan groups are endorsed under the ACD's own
category. Where Dempwolff reconstructed a cf group, the ACD reports his reconstruction,
held by him, and rejects it.

Its links are compiled by its own rule (`RULE`), crediting the ACD, never stated:

- each member of a cognate set descends (inherited) from the set's reconstruction, where
  the ACD's tree places the member's language below the reconstruction's level; members
  flagged as loans, and members the tree does not place below, get no link. Unlike the
  nested rule of the other pyetymdict sources, a member at an intermediate level (a POc
  form in a PMP set) is not made the parent of the members below it: the ACD's matcher
  counts reflexes by this rule;
- a sub-entry's reconstruction descends by unstated steps (ultimate) from its etymon's
  main entry, where the tree places it lower; at the same level the tree implies nothing;
- a near group whose members share no named ancestor gets an unnamed ancestor (form
  null) at the lowest level above all its members, and each member descends from it by
  unstated steps, at "possible".

The tree's levels are its nodes at which the ACD reconstructs a set: Formosan is a node
but not a level, so Formosan languages sit directly below PAN. A language hangs from the
node its Group names.
"""

import collections
from functools import cached_property

from etymology import assertions as A
from etymology.convert import SOURCES, agent
from etymology.convert.etymdict import Converter, Release, Tree, main_for, newick

NAME = "acd"
CLDF = SOURCES / NAME / "raw/lexibank-acd-16085f2/cldf"
DEMPWOLFF = "agent:dempwolff"


class AcdRelease(Release):
    def __init__(self):
        super().__init__(NAME, CLDF)
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

    @cached_property
    def tree(self):
        parents, root = newick(self.newick_text())
        nodes = set(parents) | {root}
        node_of_group = {self.languages[n]["Abbr"]: n for n in nodes}
        place = {
            lid: node_of_group[language["Group"]]
            for lid, language in self.languages.items()
            if lid not in nodes
        }
        levels = {self.language_of_set(s) for s in self.sets}
        return Tree(parents, root, levels, place, self.languages)


class AcdConverter(Converter):
    NAME = NAME
    LABEL = "ACD"
    EDITORS = "agent:acd-editors"
    EDITORS_NAME = "Robert Blust and Stephen Trussel (the ACD's authors)"
    RULE = "agent:wety-acd-tree@1"
    RULE_NAME = (
        "wety's ACD tree rule, v1: links compiled from the ACD's cognate sets, etyma and tree"
    )
    IMPLIED = "implied by the ACD's tree"
    YEAR = 2020

    def __init__(self, release, extraction):
        super().__init__(release, extraction)
        self.alternative_sets = []
        for kind, groups in self.r.groups.items():
            for group_id, set_ids in sorted(groups.items()):
                if len(set_ids) < 2:
                    self.counts[f"{kind} groups naming one cognate set"] += 1
                    continue
                alt = A.alternative_set(
                    sorted(f"{NAME}:{m}" for m in set_ids), exclusive=kind == "disjunct"
                )
                if any(alt["id"] == other["id"] for _, _, other in self.alternative_sets):
                    self.counts[f"{kind} groups with the same cognate sets as another"] += 1
                self.alternative_sets.append((kind, group_id, alt))
                for m in set_ids:
                    self.in_sets[m].append(alt["id"])

    def alternatives(self):
        for kind, group_id, alt in self.alternative_sets:
            self.out.add(alt)
            self.add_assertion(
                alt["id"], "endorses", f"cognates.csv#{kind.capitalize()}_Set={group_id}"
            )

    def set_category(self, set_id):
        if self.r.sets[set_id]["Is_Main_Entry"] == "true":
            return "cognate set"
        if "cognate set, not a main entry" not in self.kinds["cognatesets.csv"]:
            raise ValueError("the kind map no longer maps sub-entries")
        return "cognate set, sub-entry"

    def set_extension(self, set_id, rows):
        irregular = {
            kind: [i for i, r in enumerate(rows) if r[kind] == "true"]
            for kind in ("Metathesis", "Assimilation")
        }
        ext = {"etymon": self.r.sets[set_id]["Etymon_ID"]}
        ext.update({k.lower(): v for k, v in irregular.items() if v})
        return ext

    def cf_extension(self, c):
        ext = {}
        if c["Description"]:
            ext["gloss"] = c["Description"]
        if c["Category"] in ("root", "loan") and c["Name"]:
            ext["name"] = c["Name"]
        return ext

    def convert_cf(self, cf_id):
        c, cs = super().convert_cf(cf_id)
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

    def compile(self):
        """The links the ACD's tree implies, held by the ACD's rule (module docstring)."""
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
        hedge = self.kinds["cf.csv"]["near"]["hedge"]
        for m in dict.fromkeys(members):
            link = {"child": m, "parents": [ancestor], "relation": "ultimate"}
            self.compiled_link(
                "near member < unnamed ancestor (ultimate, possible)",
                link,
                f"cf.csv#{cf_id}:{m.split(':', 1)[1]}",
                f"cf.csv#{cf_id}",
                [{**hedge, "link": 0}],
            )


def main(argv=None):
    main_for(AcdConverter, AcdRelease(), argv)


if __name__ == "__main__":
    main()
