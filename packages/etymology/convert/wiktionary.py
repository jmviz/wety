"""Annotated claims of Wiktionary entries to assertions: the core-2 sample.

The processor will emit Wiktionary's assertions from templates (core-4) and a model will
extract them from prose (syn-2). Until then this converts a sample: claims a model wrote
per entry (`claims.json`), plus the entry-scope credits parsed from each entry's
References section (`sample.json`, from the pages dump).

A claims file:

    {"model": "...", "agents": {key: {kind, name}}, "works": {key: {citation, ...}},
     "entries": [{"page", "entry": "lang:term", "homograph"?, "x"?, "claims": [...],
                  "sets": [{"members": [claim keys], "exclusive", "preference"?}],
                  "cognates": [{"key", "members": [terms], "stance"?, "holder"?, ...}]}]}

A claim: {"key", "stance" (endorses), "holder" (the editors), "links", "hedges",
"credits", "quote", "x"}. A link: {"parents": [terms], "mode", "relation" (derivation),
"head", "child"}: a link's child is the head parent of the link before it (the entry
for the first), or the entry when `child` is "entry". A term is `lang:form`, or `lang:-` for a language with no word; `<id:x>` names a
homograph and `<t:x>` gives a gloss, as in {{etymon}}.
"""

import argparse
import json
import re
from pathlib import Path

from etymology import assertions as A
from etymology import wikitext
from etymology.convert import agent, source, work

NAME = "wiktionary"
EDITORS = "agent:wiktionary-editors"
TERM = re.compile(r"^([a-z0-9-]+):([^<]+)((?:<[a-z]+:[^>]+>)*)$")
MODIFIER = re.compile(r"<([a-z]+):([^>]+)>")


def r_templates(references):
    """(name, page or pages argument) of each {{R:...}} template in References sections
    outside footnotes, read as the whole-dump reading does (`wiktionary_references`)."""
    found = []
    for text in references:
        body, _ = wikitext.strip_refs(text)
        for name, args in wikitext.templates(body, "R:"):
            named = wikitext.named_args(args)
            found.append((name, named.get("page") or named.get("pages")))
    return found


class Converter:
    def __init__(self, claims, extraction):
        self.claims = claims
        self.src = source(NAME)
        self.out = A.Writer()
        self.extraction = extraction["id"]
        self.out.add(self.src)
        self.out.add(extraction)
        self.out.add(agent(EDITORS, "editors", "Wiktionary's editors"))
        self.agents = {}
        self.works = {}

    def agent(self, key):
        if key not in self.agents:
            a = self.claims["agents"][key]
            self.agents[key] = self.out.add(agent(f"agent:{key}", a["kind"], a["name"]))
        return self.agents[key]

    def work(self, key):
        if key not in self.works:
            # {{R:...}} templates need no entry in the claims file; other works do.
            spec = (
                self.claims["works"].get(key) if key.startswith("R:") else self.claims["works"][key]
            )
            spec = spec or {"citation": "{{" + key + "}}"}
            w = work(f"work:{NAME}:{key.replace(' ', '_')}", spec["citation"])
            for field in ("year", "source"):
                if field in spec:
                    w[field] = spec[field]
            if "authors" in spec:
                w["authors"] = [self.agent(a) for a in spec["authors"]]
            self.works[key] = self.out.add(w)
        return self.works[key]

    def record(self, term, entry_key, **fields):
        lang, form, modifiers = TERM.match(term).groups()
        modifiers = dict(MODIFIER.findall(modifiers))
        homograph = modifiers.get("id")
        unnamed = form == "-"
        rid = f"{NAME}:{lang}:{form}" + (f"<{homograph}>" if homograph else "")
        if unnamed:
            # A language with no word is a different record in each entry.
            rid += f"@{entry_key}"
        rec = {
            "type": "record",
            "id": rid,
            "source": self.src["id"],
            "language": {"label": lang, "code": lang, "relation": "exact"},
            "form": None if unnamed else form,
            "reconstructed": (not unnamed) and form.startswith("*"),
        }
        if homograph:
            rec["homograph"] = homograph
        if "t" in modifiers:
            rec["gloss"] = modifiers["t"]
        rec.update(fields)
        return self.out.add(rec)

    def credits(self, specs):
        found = []
        for c in specs:
            credit = {"work": self.work(c["work"]), "role": c["role"], "scope": c["scope"]}
            if c.get("locator"):
                credit["locator"] = c["locator"]
            found.append(credit)
        return found

    def add_assertion(self, target, stance, locator, holder=None, **fields):
        holder = self.agent(holder) if holder else EDITORS
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

    def entry(self, e, sample):
        key = e["entry"] + (f"#{e['homograph']}" if e.get("homograph") else "")
        fields = {"locator": e["page"]}
        if e.get("x"):
            fields["x"] = {NAME: e["x"]}
        term = e["entry"] + (f"<id:{e['homograph']}>" if e.get("homograph") else "")
        word = self.record(term, key, **fields)
        locator = e["page"] + (f"#Etymology {e['homograph']}" if e.get("homograph") else "")
        entry_credits = []
        for name, page in r_templates(sample["references_wikitext"]):
            credit = {"work": self.work(name), "role": "follows", "scope": "entry"}
            if page:
                credit["locator"] = page
            entry_credits.append(credit)
        made = {}
        for c in e["claims"]:
            links, child = [], word
            for spec in c["links"]:
                link = {
                    "child": word if spec.get("child") == "entry" else child,
                    "parents": [self.record(p, key) for p in spec.get("parents", [])],
                    "relation": spec.get("relation", "derivation"),
                }
                if "mode" in spec:
                    link["mode"] = spec["mode"]
                if "head" in spec:
                    link["head"] = spec["head"]
                links.append(link)
                if link["parents"]:
                    child = link["parents"][spec.get("head", 0)]
            h = A.hypothesis(word, links)
            self.out.add(h)
            stance = c.get("stance", "endorses")
            credits = self.credits(c.get("credits", []))
            if stance == "endorses":
                credits += entry_credits
            made[c["key"]] = self.add_assertion(
                h["id"],
                stance,
                f"{locator}/{c['key']}",
                holder=c.get("holder"),
                hedges=c.get("hedges", []),
                credits=credits,
                quote=c.get("quote"),
                x={NAME: c["x"]} if c.get("x") else None,
            )
        for s in e.get("sets", []):
            targets = [self.out.objects[made[k]]["target"] for k in s["members"]]
            preference = s.get("preference")
            if preference:
                preference = [self.out.objects[made[k]]["target"] for k in preference]
            alt = A.alternative_set(targets, s["exclusive"], preference)
            self.out.add(alt)
            for k in s["members"]:
                a = self.out.objects[made[k]]
                a["in_sets"] = sorted(set(a.get("in_sets", [])) | {alt["id"]})
            self.add_assertion(
                alt["id"],
                s.get("stance", "endorses"),
                f"{locator}/" + "|".join(s["members"]),
                holder=s.get("holder"),
            )
        for cg in e.get("cognates", []):
            members = [word] + [self.record(m, key) for m in cg["members"]]
            cs = {
                "type": "cognate_set",
                "id": f"{NAME}:{key}/{cg['key']}",
                "source": self.src["id"],
                "reconstruction": None,
                "members": members,
                "category": cg.get("category", "{{cog}}"),
            }
            self.out.add(cs)
            self.add_assertion(
                cs["id"],
                cg.get("stance", "endorses"),
                f"{locator}/{cg['key']}",
                holder=cg.get("holder"),
                hedges=cg.get("hedges", []),
                credits=self.credits(cg.get("credits", [])),
                quote=cg.get("quote"),
                x={NAME: cg["x"]} if cg.get("x") else None,
            )


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("sample", type=Path, help="sample.json: entries with References wikitext")
    p.add_argument("claims", type=Path, help="claims.json: the claims annotated per entry")
    p.add_argument("--out", type=Path, required=True)
    args = p.parse_args(argv)
    sample = {s["page"]: s for s in json.loads(args.sample.read_text())["entries"]}
    claims = json.loads(args.claims.read_text())
    extraction = A.extraction(
        "model",
        "packages/etymology/convert/wiktionary.py",
        [args.sample, args.claims],
        model=claims["model"],
        reviewer=claims.get("reviewer"),
    )
    c = Converter(claims, extraction)
    for e in claims["entries"]:
        c.entry(e, sample[e["page"]])
    print(f"{args.out}: {c.out.write(args.out)} objects")


if __name__ == "__main__":
    main()
