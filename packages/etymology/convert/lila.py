"""LiLa's lemonEty encoding of de Vaan 2008 to assertions.

Every resource of the release enters. Each lemonEty Etymology is one hypothesis that de
Vaan endorses: its EtyLinks, followed from the Latin entry back to the starting link, are
a chain of inheritances. An entry with two etymologies offers them as exclusive
alternatives, with no preference. Every LexicalEntry and every Etymon is a record, also
those no etymology uses (Latin entries that are another part of speech of a main entry,
and etymons LiLa reaches only by a cognate link).

A writtenRep sometimes holds more than one form: a grammatical label (`pr.`, `[n.]`), a
hedge (`?`, `(?)`, `(vel sim.)`), conversion damage, or several forms with credits.
`read` applies the rules below; the written forms they cannot read are read in the
curation file `etc/forms.json`, and any other is an error. A record's `form` is its first
form; its other forms, each with the words LiLa wrote, stay in `x.lila-edl.forms`. A
hedge on the record's form is doubt about the reconstruction, its `certainty`. Credits
LiLa writes inside forms or comments (`thus Schrijver`) are de Vaan's credits, on the
assertion of every etymology that uses the record.

LiLa's comments (its transcription of de Vaan's line, and its curators' review notes) and
the forms as written are kept in the store, never served: they may hold de Vaan's words.
"""

import argparse
import collections
import json
import re
import unicodedata
from pathlib import Path

from rdflib import RDF, RDFS, BNode, Graph, Literal, Namespace, URIRef

from etymology import assertions as A
from etymology.convert import SOURCES, Languages, agent, profile, source, work

NAME = "lila-edl"
ETY = Namespace("http://lari-datasets.ilc.cnr.it/lemonEty#")
LIME = Namespace("http://www.w3.org/ns/lemon/lime#")
ONTOLEX = Namespace("http://www.w3.org/ns/lemon/ontolex#")
LEXINFO = "https://lexinfo.net/ontology/2.0/lexinfo.owl#"
LILA_LINK = URIRef("http://example.net/lila_link")
BASE = "http://lila-erc.eu/data/lexicalResources/BrillEDL/id/"
FORMS = SOURCES / NAME / "etc/forms.json"
DE_VAAN = "agent:de-vaan"
MODES = {"inheritance": "inherited"}
# LiLa's lexinfo values that are misspelt or under the wrong property, read as meant.
FEATURES = {
    ("gender", "femine"): ("gender", "feminine"),
    ("gender", "femminine"): ("gender", "feminine"),
    ("gender", "neutor"): ("gender", "neuter"),
    ("gender", "noun"): ("partOfSpeech", "noun"),
    ("case", "neuter"): ("gender", "neuter"),
    ("case", "ablattive"): ("case", "ablative"),
    ("case", "nominativeCase"): ("case", "nominative"),
    ("case", "obliqueCase"): ("case", "oblique"),
    ("number", "masculine"): ("gender", "masculine"),
    ("tense", "genitive"): ("case", "genitive"),
    ("tense", "perferct"): ("tense", "perfect"),
    ("degree", "superative"): ("degree", "superlative"),
}
LABEL_WORDS = {
    # inflection and stems
    "pr", "pf", "aor", "ppp", "ipv", "red", "comp", "caus", "nom", "acc", "gen", "dat",
    "abl", "loc", "ins", "voc", "sg", "pl", "du",
    # gender and part of speech
    "m", "f", "n", "adj", "adv", "v",
}  # fmt: skip
LEADING = re.compile(r"^((?:[a-z]+\.)+) (?=[*(])")
BRACKET = re.compile(r" ?\[([a-z.]+\.\??|in cp\.)\]$")
HEDGE = re.compile(r" ?(\(\?\)|\?|\(vel sim\.\))$")
SIMPLE = re.compile(r"^(\(\*[^\s()]+\))?\*?[^\s;<>,*]+( \+ \*?[^\s;<>,*]+)*$")
# Conversion damage: a Greek omicron for o (*koχο‑); h₃ written ʰ3 (bōs: *gʷʰ3‑eu‑m).
CHARACTERS = str.maketrans({"ο": "o"})
LARYNGEAL = re.compile(r"ʰ(\(?)([123])")


def local(uri):
    """LiLa's own key for a resource: the Brill entry id, or the path after /id/."""
    uri = str(uri)
    if "dictionary=latin&id=" in uri:
        return uri.rsplit("id=", 1)[1]
    return uri.removeprefix(BASE)


def name(uri):
    return str(uri).rsplit("#", 1)[-1]


def hyphens(form):
    """LiLa writes both '-' and the non-breaking '‑'; compare them as one."""
    return unicodedata.normalize("NFC", form).replace("-", "‑")


def spellings(form):
    """A regex for the spellings of a form in de Vaan's notation: `(x)` optional, `a/b`
    one of two segments; a segment is a letter with its marks (ʰ, ʷ, ₁, accents)."""
    clusters = []
    for ch in hyphens(form):
        if clusters and (unicodedata.category(ch) == "Mn" or ch in "ʰʷ₁₂₃"):
            clusters[-1] += ch
        else:
            clusters.append(ch)
    pieces, alternative = [], False
    for c in clusters:
        if c == "(":
            pieces.append("(?:")
        elif c == ")":
            pieces.append(")?")
        elif c == "/":
            alternative = True
        elif alternative:
            pieces[-1] = f"(?:{pieces[-1]}|{re.escape(c)})"
            alternative = False
        else:
            pieces.append(re.escape(c))
    return "".join(pieces)


def label_words(label):
    """The words of a grammatical label, which must all be known: `nom.sg.` is nom, sg."""
    words = [w for w in re.split(r"[. ?]+", label) if w]
    unknown = [w for w in words if w not in LABEL_WORDS and label != "in cp."]
    if unknown:
        raise ValueError(f"label {label!r}: unknown words {unknown}")
    return words


def read(written):
    """One writtenRep of a reconstruction read by the rules: a part {form, label?,
    hedge?}, and the rules applied; None when the rules cannot read it."""
    text = written.strip()
    rules = ["whitespace"] if text != written else []
    if text.translate(CHARACTERS) != text:
        text = text.translate(CHARACTERS)
        rules.append("a Greek letter for a Latin one")
    if LARYNGEAL.search(text):
        text = LARYNGEAL.sub(lambda m: "h" + m.group(1) + "₁₂₃"[int(m.group(2)) - 1], text)
        rules.append("a laryngeal written ʰ and a digit")
    for damage in (" ,", " >"):
        if text.endswith(damage):
            text = text.removesuffix(damage).strip()
            rules.append(f"dangling {damage.strip()!r}")
    part = {}
    m = HEDGE.search(text)
    if m:
        part["hedge"] = m.group(1)
        text = text[: m.start()]
    m = LEADING.match(text) or BRACKET.search(text)
    if m:
        part["label"] = m.group(1)
        label_words(m.group(1))
        text = text[m.end() :] if m.re is LEADING else text[: m.start()]
        m = HEDGE.search(text)
        if m and "hedge" not in part:
            part["hedge"] = m.group(1)
            text = text[: m.start()]
    if text[:1].isalpha():
        text = "*" + text
        rules.append("added the missing '*'")
    if not SIMPLE.match(text):
        return None, rules
    return {"form": text, **part}, rules


class Readings:
    """`etc/forms.json`: the curated readings of written forms the rules cannot read,
    the credits LiLa writes in comments, and the works those credits name."""

    def __init__(self, path=FORMS):
        data = json.loads(path.read_text())
        self.works = data["works"]
        self.forms = {(r["etymon"], r["written"]): r for r in data["forms"]}
        self.comments = {(r["etymon"], r["written"]): r for r in data["comments"]}
        self.used = set()

    def form(self, etymon, written):
        r = self.forms.get((etymon, written))
        if r:
            self.used.add(("form", etymon, written))
        return r

    def comment(self, etymon, written):
        r = self.comments.get((etymon, written))
        if r:
            self.used.add(("comment", etymon, written))
        return r

    def unused(self):
        keys = {("form", *k) for k in self.forms} | {("comment", *k) for k in self.comments}
        return sorted(keys - self.used)


class Converter:
    def __init__(self, graph, extraction, readings=None):
        self.g = graph
        self.lang = Languages(NAME)
        self.src = source(NAME)
        self.readings = readings or Readings()
        self.out = A.Writer()
        self.extraction = extraction["id"]
        self.decisions = collections.defaultdict(list)
        self.credits_of = {}
        self.made = {}
        self.entry_of = {
            ety: entry
            for entry in graph.subjects(RDF.type, ONTOLEX.LexicalEntry)
            for ety in graph.objects(entry, ETY.etymology)
        }
        for o in (self.src, extraction, agent(DE_VAAN, "person", "Michiel de Vaan")):
            self.out.add(o)

    def decide(self, kind, resource, written, reading):
        self.decisions[kind].append({"resource": resource, "written": written, "read": reading})

    def features(self, node):
        """A node's lexinfo features as `property=value`, with LiLa's slips corrected; and
        the slips, as written."""
        found, slips = [], []
        for p, o in self.g.predicate_objects(node):
            if not str(p).startswith(LEXINFO):
                continue
            key = (name(p), name(o))
            if key in FEATURES:
                slips.append("=".join(key))
                key = FEATURES[key]
            found.append("=".join(key))
        return sorted(set(found)), sorted(slips)

    def cognates(self, node):
        """LiLa's cognate links from a resource, the misspelt lemonEty:congnate included:
        record ids, or LiLa lemmas where the link names only a lemma."""
        found = []
        for prop in (ETY.cognate, ETY.congnate):
            for o in self.g.objects(node, prop):
                if prop == ETY.congnate:
                    self.decide("congnate", local(node), "lemonEty:congnate", "lemonEty:cognate")
                if isinstance(o, BNode):
                    found.append(str(self.g.value(o, ONTOLEX.canonicalForm)))
                else:
                    found.append(f"{NAME}:{local(o)}")
        return sorted(found)

    def comments(self, node, ext):
        cs = sorted(str(c) for c in self.g.objects(node, RDFS.comment))
        if cs:
            ext["comments"] = cs
        return cs

    def entry(self, node):
        written = str(self.g.value(node, RDFS.label))
        form, homograph = written, None
        m = re.match(r"^(.*) \(([a-z]+\.)\)$", written)
        if m:
            form, homograph = m.groups()
        rec = {
            "type": "record",
            "id": f"{NAME}:{local(node)}",
            "source": self.src["id"],
            "language": self.lang("Latin"),
            "form": form,
            "reconstructed": form.startswith("*"),
            "locator": str(node),
        }
        if homograph:
            rec["homograph"] = homograph
        ext = {}
        lemma = self.g.value(node, ONTOLEX.canonicalForm)
        if isinstance(lemma, URIRef):
            ext["lila_lemma"] = str(lemma)
        if form != written:
            ext["label"] = written
        self.comments(node, ext)
        cognates = self.cognates(node)
        if cognates:
            ext["cognates"] = cognates
        if ext:
            rec["x"] = {NAME: ext}
        return self.out.add(rec)

    def parts(self, key, written):
        """The parts of one writtenRep: by the rules, else from `etc/forms.json`."""
        part, rules = read(written)
        if part is not None:
            for rule in rules:
                self.decide("rule", key, written, f"{rule}: {part['form']}")
            return [part]
        r = self.readings.form(key, written)
        if r is None:
            raise ValueError(f"{key}: {written!r} is neither read by the rules nor in {FORMS}")
        self.decide("reading", key, written, r["decision"])
        for p in r["parts"]:
            if "credit" in p:
                self.credits_of.setdefault(key, []).append(p["credit"])
        return [dict(p) for p in r["parts"]]

    def forms(self, node, key):
        """Every form of an etymon, each written form read into parts: canonical forms
        first, the one LiLa's label begins with first of all. `group` numbers LiLa's
        form nodes, so parts of one node are variants of each other."""
        label = str(self.g.value(node, RDFS.label))
        found = []
        nodes = [
            (f, kind)
            for prop, kind in ((ONTOLEX.canonicalForm, "canonical"), (ONTOLEX.otherForm, "other"))
            for f in sorted(self.g.objects(node, prop), key=self._reps)
        ]
        for group, (f, kind) in enumerate(nodes):
            feats, slips = self.features(f)
            for slip in slips:
                self.decide("feature", key, slip, "=".join(FEATURES[tuple(slip.split("="))]))
            for nested in self.g.objects(f, ONTOLEX.otherForm):
                # pit0596: an otherForm hung on the canonical form, not on the etymon.
                self.decide("nested form", key, str(nested), "an other form of the etymon")
                nodes.append((Literal(str(nested)), "other"))
            reps = self._reps(f) if not isinstance(f, Literal) else [str(f)]
            for written in reps:
                for n, p in enumerate(self.parts(key, written)):
                    e = {"kind": kind, "group": group, **p}
                    if p["form"] != written:
                        e["written"] = written
                    if feats:
                        e["features"] = feats
                    at = label.find(written)
                    e["_rank"] = (kind != "canonical", "relation" in p, at if at >= 0 else 1e9, n)
                    found.append(e)
        found.sort(key=lambda e: e["_rank"])
        for e in found:
            del e["_rank"]
        return found

    def label_form(self, key, label, forms):
        """LiLa's label as the record's form, where it is one form in de Vaan's notation
        and LiLa's forms of its first node only spell out its optional and alternative
        segments (*leǵ‑e/o‑ written *leǵ‑e‑ and *leǵ‑o‑)."""
        part, _ = read(label)
        if part is None or hyphens(part["form"]) == hyphens(forms[0]["form"]):
            return None
        spelled = [e for e in forms if e["group"] == forms[0]["group"]]
        try:
            pattern = re.compile(spellings(part["form"]))
        except re.error:  # an alternative between groups, as in *x(a)/(b): not read
            return None
        if not all(pattern.fullmatch(hyphens(e["form"])) for e in spelled):
            return None
        self.decide("label", key, label, f"the record's form, which {len(spelled)} forms spell out")
        return part

    def _reps(self, f):
        return sorted(str(r) for r in self.g.objects(f, ONTOLEX.writtenRep))

    def etymon(self, node):
        key = local(node)
        forms = self.forms(node, key)
        label = str(self.g.value(node, RDFS.label))
        first = self.label_form(key, label, forms) or forms[0]
        rec = {
            "type": "record",
            "id": f"{NAME}:{key}",
            "source": self.src["id"],
            "language": self.lang(str(self.g.value(node, LIME.language))),
            "form": first["form"],
            "reconstructed": True,
            "locator": str(node),
        }
        if "hedge" in first:
            rec["certainty"] = {"level": "possible", "words": first["hedge"]}
        sense = self.g.value(node, ONTOLEX.sense)
        if sense is not None:
            rec["gloss"] = str(sense)
            self.decide("sense", key, str(sense), "the record's gloss, as LiLa wrote it")
        ext = {"forms": forms}
        if label != first["form"]:
            ext["label"] = label
        feats, slips = self.features(node)
        if feats:
            ext["features"] = feats
        for slip in slips:
            self.decide("feature", key, slip, "=".join(FEATURES[tuple(slip.split("="))]))
        for c in self.comments(node, ext):
            r = self.readings.comment(key, c)
            if r:
                self.decide("comment credit", key, c, r["decision"])
                for credit in r["credits"]:
                    self.credits_of.setdefault(key, []).append(credit)
                    for e in forms:
                        if e["form"] == credit["form"]:
                            e["credit"] = {"work": credit["work"], "role": credit["role"]}
        lila_link = sorted(str(v) for v in self.g.objects(node, LILA_LINK))
        if lila_link:
            ext["lila_link"] = lila_link[0] if len(lila_link) == 1 else lila_link
        cognates = self.cognates(node)
        if cognates:
            ext["cognates"] = cognates
        rec["x"] = {NAME: ext}
        return self.out.add(rec)

    def record(self, node):
        """The record of an entry or an etymon, made once."""
        if node not in self.made:
            entry = (node, RDF.type, ONTOLEX.LexicalEntry) in self.g
            self.made[node] = self.entry(node) if entry else self.etymon(node)
        return self.made[node]

    def credits(self, records):
        found = []
        for r in records:
            for c in self.credits_of.get(r.split(":", 1)[1], []):
                w = self.readings.works[c["work"]]
                wid = self.out.add(work(f"work:{NAME}:{c['work']}", w["citation"]))
                credit = {"work": wid, "role": c["role"], "scope": "inline"}
                if credit not in found:
                    found.append(credit)
        return found

    def etymology(self, ety):
        """The hypothesis of one Etymology and de Vaan's assertion of it."""
        key = local(ety)
        entry = self.entry_of[ety]
        by_target = {
            self.g.value(link, ETY.etyTarget): link for link in self.g.objects(ety, ETY.hasEtyLink)
        }
        start = self.g.value(ety, ETY.startingLink)
        word = self.record(entry)
        links, node, records = [], entry, [word]
        while node in by_target:
            link = by_target.pop(node)
            parent = self.g.value(link, ETY.etySource)
            kind = str(self.g.value(link, ETY.etyLinkType))
            links.append(
                {
                    "child": self.record(node),
                    "parents": [self.record(parent)],
                    "relation": "derivation",
                    "mode": MODES[kind],
                }
            )
            records.append(links[-1]["parents"][0])
            node = parent
        if by_target:
            raise ValueError(f"{key}: links that do not chain from the entry: {sorted(by_target)}")
        if link != start:
            raise ValueError(f"{key}: the chain does not end at the starting link")
        listed = {f"{NAME}:{local(e)}" for e in self.g.objects(ety, ETY.etymon)}
        if listed != set(records[1:]):
            raise ValueError(f"{key}: its etymons are not the records its links chain")
        h = A.hypothesis(word, links)
        self.out.add(h)
        a = A.assertion(
            self.src["id"],
            key,
            h["id"],
            "endorses",
            asserter=DE_VAAN,
            holder=DE_VAAN,
            extraction=self.extraction,
            credits=self.credits(records),
        )
        return self.out.add(a)

    def alternatives(self, entry, assertions):
        """An entry's etymologies as one exclusive set, asserted by de Vaan."""
        targets = [self.out.objects[a]["target"] for a in assertions]
        alt = A.alternative_set(sorted(targets), True)
        self.out.add(alt)
        for a in assertions:
            o = self.out.objects[a]
            o["in_sets"] = sorted(set(o.get("in_sets", [])) | {alt["id"]})
        a = A.assertion(
            self.src["id"],
            local(entry),
            alt["id"],
            "endorses",
            asserter=DE_VAAN,
            holder=DE_VAAN,
            extraction=self.extraction,
        )
        return self.out.add(a)

    def all(self):
        """Every etymology, every entry and every etymon of the release."""
        by_entry = collections.defaultdict(list)
        for ety in sorted(self.g.subjects(RDF.type, ETY.Etymology), key=local):
            by_entry[self.entry_of[ety]].append(self.etymology(ety))
        for entry, made in sorted(by_entry.items()):
            if len(made) > 1:
                self.alternatives(entry, made)
        for t in (ONTOLEX.LexicalEntry, ETY.Etymon):
            for node in sorted(self.g.subjects(RDF.type, t)):
                self.record(node)
        unused = self.readings.unused()
        if unused:
            raise ValueError(f"readings in {FORMS} that match nothing: {unused}")


def release_counts(g):
    """What the release holds, counted from the graph."""
    return {
        "Etymology": len(set(g.subjects(RDF.type, ETY.Etymology))),
        "Etymon": len(set(g.subjects(RDF.type, ETY.Etymon))),
        "EtyLink": len(set(g.subjects(RDF.type, ETY.EtyLink))),
        "LexicalEntry": len(set(g.subjects(RDF.type, ONTOLEX.LexicalEntry))),
        "cognate links": len(list(g.triples((None, ETY.cognate, None))))
        + len(list(g.triples((None, ETY.congnate, None)))),
        "writtenRep with '?'": sum("?" in str(o) for o in g.objects(None, ONTOLEX.writtenRep))
        + sum(
            "?" in str(o) for s, o in g.subject_objects(ONTOLEX.otherForm) if isinstance(o, Literal)
        ),
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--sample", type=Path, help="JSON with `etymologies`: LiLa ids (etymology/N)")
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--decisions", type=Path, help="write the decisions taken, as JSON")
    args = p.parse_args(argv)
    ttl = Path(A.REPO / profile(NAME)["raw"])
    g = Graph()
    g.parse(ttl, format="turtle")
    inputs = [ttl, SOURCES / NAME / "etc/languages.json", FORMS]
    extraction = A.extraction(
        "converter",
        "packages/etymology/convert/lila.py",
        inputs + ([args.sample] if args.sample else []),
    )
    c = Converter(g, extraction)
    if args.sample:
        for key in json.loads(args.sample.read_text())["etymologies"]:
            c.etymology(URIRef(f"{BASE}{key}"))
    else:
        c.all()
    print(f"{args.out}: {c.out.write(args.out)} objects")
    if args.decisions:
        args.decisions.write_text(
            json.dumps(
                {"release": release_counts(g), "decisions": c.decisions},
                ensure_ascii=False,
                indent=1,
            )
            + "\n"
        )


if __name__ == "__main__":
    main()
