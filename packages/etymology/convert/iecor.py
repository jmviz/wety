"""IE-CoR's CLDF release to assertions (Heggarty, Anderson and Scarborough, Indo-European
Cognate Relationships, v1.2.1).

IE-CoR codes the words for 170 meanings in 160 Indo-European varieties into cognate
sets; its editors endorse each set, with the works its `Source` cites (a key, pages and
the work's words: credits of role `follows`) and its `Justification` as their words. A
set's root (`Root_Form` in `Root_Language`) is its reconstruction: a proto-form, or the
attested word a set of loans or of a language's descendants goes back to. A root
written in brackets is a source outside IE-CoR's sample ("[Persian]"), one opening
with "?" is doubtful (the record's certainty, possible). A form IE-CoR gives in its
native script (Greek, Cyrillic, Arabic) is recorded in it; a root written "χρόνος /
chrónos" is recorded as its first part.

Links are compiled by `RULE`, crediting IE-CoR, never stated: each member of a set
with a root descends from the root by unstated steps (ultimate); IE-CoR codes cognacy
within a meaning, not the derivation, so the member may be a derivative. A member in
the root's own language gets no link. A set `loans.csv` marks as a parallel loan event
has each member borrowed (mode borrowed) from the source form it names (the root, when
they are one); a single loan event into an ancestor adds nothing beside the root.
`proposedAsCognateTo` (208 sets) and supersets are not read yet.
"""

import collections
import json
import re

from etymology import assertions as A
from etymology.convert import SOURCES, Languages, agent, profile, source, work
from etymology.convert.etymdict import BIB_ENTRY, BIB_FIELD, main_for, read_table

NAME = "iecor"
CLDF = SOURCES / NAME / "raw/lexibank-iecor-dd1ebc9/cldf"
EDITORS = "agent:iecor-editors"
RULE = "agent:wety-iecor-root@1"
RELEASE = f"work:{NAME}:release"
IMPLIED = "implied by IE-CoR's cognate set and root"
UNNAMED = {"label": "a language IE-CoR does not name", "code": None, "relation": "unresolved"}
CITATION = re.compile(r"(\w+)\[([^\[{\]]*)(?:\{(.*?)\})?\]")
INPUTS = (
    "languages.csv",
    "forms.csv",
    "parameters.csv",
    "cognatesets.csv",
    "cognates.csv",
    "loans.csv",
    "sources.bib",
)


class Roots(Languages):
    """IE-CoR's root languages and loan sources, by name (`etc/roots.json`)."""

    def __init__(self):
        data = json.loads((SOURCES / NAME / "etc/roots.json").read_text())
        self.rows = {r["id"]: r for r in data["rows"]}


def root_form(text):
    """A root as recorded: its first part where it is given twice ("χρόνος / chrónos"),
    without the brackets of a source outside the sample, and whether it is doubtful."""
    form = text.strip()
    if form.startswith("[") and form.endswith("]"):
        form = form[1:-1].strip()
    doubtful = form.startswith("?")
    form = form.lstrip("?").strip().split(" / ")[0].strip()
    return form, doubtful


class IecorRelease:
    def __init__(self):
        self.cldf = CLDF
        self.languages = {r["ID"]: r for r in read_table(CLDF / "languages.csv")}
        self.forms = {r["ID"]: r for r in read_table(CLDF / "forms.csv")}
        self.meanings = {r["ID"]: r["Name"] for r in read_table(CLDF / "parameters.csv")}
        self.sets = {r["ID"]: r for r in read_table(CLDF / "cognatesets.csv")}
        self.cognates = collections.defaultdict(list)
        for r in read_table(CLDF / "cognates.csv"):
            self.cognates[r["Cognateset_ID"]].append(r)
        self.loans = {r["Cognateset_ID"]: r for r in read_table(CLDF / "loans.csv")}
        self.bib = {}
        for key, body in BIB_ENTRY.findall((CLDF / "sources.bib").read_text()):
            self.bib[key] = dict(BIB_FIELD.findall(body))


class IecorConverter:
    NAME = NAME
    LABEL = "IE-CoR"

    def __init__(self, release, extraction):
        self.r = release
        self.lang = Languages(NAME)
        self.roots = Roots()
        self.src = source(NAME)
        self.out = A.Writer()
        self.extraction = extraction["id"]
        self.counts = collections.Counter()
        for o in (
            self.src,
            extraction,
            agent(
                EDITORS,
                "editors",
                "Paul Heggarty, Cormac Anderson and Matthew Scarborough (IE-CoR's editors)",
            ),
            agent(
                RULE, "rule", "wety's IE-CoR rule, v1: a cognate set's members go back to its root"
            ),
            work(RELEASE, profile(NAME)["cite_as"], source=self.src["id"], year=2024),
        ):
            self.out.add(o)

    def record(self, form_id):
        f = self.r.forms[form_id]
        language = self.r.languages[f["Language_ID"]]
        rec = {
            "type": "record",
            "id": f"{NAME}:{form_id}",
            "source": self.src["id"],
            "language": self.lang(f["Language_ID"], language["Name"]),
            "form": f["native_script"] or f["Value"],
            "reconstructed": False,
            "gloss": f["Gloss"] or self.r.meanings[f["Parameter_ID"]],
            "locator": f"forms.csv#{form_id}",
        }
        return self.out.add(rec)

    def named(self, key, language, form, locator, gloss=None):
        """A root or a loan's source form, in a language IE-CoR names."""
        form, doubtful = root_form(form)
        lang = self.roots(language) if language.strip() else UNNAMED
        rec = {
            "type": "record",
            "id": f"{NAME}:{key}",
            "source": self.src["id"],
            "language": lang,
            "form": form.lstrip("*") or None,
            "reconstructed": form.startswith("*") or (lang["code"] or "").endswith("-pro"),
            "locator": locator,
        }
        if gloss:
            rec["gloss"] = gloss
        if doubtful:
            rec["certainty"] = {"level": "possible", "words": "? (IE-CoR's doubtful root)"}
        return self.out.add(rec)

    def credits(self, text):
        out = []
        for key, pages, _words in CITATION.findall(text or ""):
            fields = self.r.bib.get(key)
            if fields is None:
                self.counts["citations of a key not in sources.bib"] += 1
                continue
            citation = fields.get("note") or fields.get("title") or key
            w = work(f"work:{NAME}:{key}", citation)
            if fields.get("year", "")[:4].isdigit():
                w["year"] = int(fields["year"][:4])
            self.out.add(w)
            credit = {"work": w["id"], "role": "follows", "scope": "inline"}
            if pages.strip():
                credit["locator"] = pages.strip()
            out.append(credit)
        return out

    def convert(self):
        for set_id, s in self.r.sets.items():
            rows = self.r.cognates[set_id]
            members = [self.record(r["Form_ID"]) for r in rows]
            root = None
            if s["Root_Form"].strip():
                gloss = s.get("Root_Gloss") or None
                root = self.named(
                    f"{set_id}:root",
                    s["Root_Language"],
                    s["Root_Form"],
                    f"cognatesets.csv#{set_id}",
                    gloss,
                )
            if not members and root is None:
                self.counts["sets with neither members nor root"] += 1
                continue
            cs = {
                "type": "cognate_set",
                "id": f"{NAME}:{set_id}",
                "source": self.src["id"],
                "reconstruction": root,
                "members": members,
                "category": "cognate set",
            }
            self.out.add(cs)
            self.endorse(
                cs["id"],
                EDITORS,
                f"cognatesets.csv#{set_id}",
                credits=self.credits(s["Source"]),
                quote=s["Justification"] or None,
            )
            self.compile(set_id, s, members, root)

    def compile(self, set_id, s, members, root):
        loan = self.r.loans.get(set_id)
        donor = None
        if loan and loan["Source_form"].strip() and loan["Parallel_loan_event"] == "true":
            donor = self.named(
                f"{set_id}:loan",
                loan["Source_languoid"],
                loan["Source_form"],
                f"loans.csv#{set_id}",
            )
            if root and self.out.objects[root]["form"] == self.out.objects[donor]["form"]:
                del self.out.objects[donor]
                donor = root
        for m in dict.fromkeys(members):
            code = self.out.objects[m]["language"]["code"]
            if donor:
                link = {
                    "child": m,
                    "parents": [donor],
                    "relation": "derivation",
                    "mode": "borrowed",
                }
                self.compiled(link, set_id, "member < the source of a parallel loan (borrowed)")
            if root and root != donor:
                if code and code == self.out.objects[root]["language"]["code"]:
                    self.counts["members in the root's own language: no link"] += 1
                    continue
                link = {"child": m, "parents": [root], "relation": "ultimate"}
                self.compiled(link, set_id, "member < its set's root (ultimate)")

    def compiled(self, link, set_id, what):
        h = A.hypothesis(link["child"], [link])
        self.out.add(h)
        self.counts[f"compiled: {what}"] += 1
        self.endorse(
            h["id"],
            RULE,
            f"cognatesets.csv#{set_id}:{link['child'].split(':', 1)[1]}",
            credits=[
                {
                    "work": RELEASE,
                    "role": "follows",
                    "scope": "entry",
                    "locator": f"cognatesets.csv#{set_id}",
                }
            ],
            compiled={"from": f"cognatesets.csv#{set_id}", "words": IMPLIED},
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
    main_for(IecorConverter, IecorRelease(), argv, tables=INPUTS)


if __name__ == "__main__":
    main()
