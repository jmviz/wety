"""LiLa's lemonEty encoding of de Vaan 2008 to assertions.

Each lemonEty Etymology is one hypothesis that de Vaan endorses: its EtyLinks, followed
from the Latin entry back to the starting link, are a chain of inheritances. A "?" that
ends a reconstruction is de Vaan's doubt about the reconstruction, on its record. LiLa
encodes no other certainty and no credits.
"""

import argparse
import json
import re
from pathlib import Path

from rdflib import RDF, RDFS, Graph, Namespace, URIRef

from etymology import assertions as A
from etymology.convert import SOURCES, Languages, agent, profile, source

NAME = "lila-edl"
ETY = Namespace("http://lari-datasets.ilc.cnr.it/lemonEty#")
LIME = Namespace("http://www.w3.org/ns/lemon/lime#")
ONTOLEX = Namespace("http://www.w3.org/ns/lemon/ontolex#")
BASE = "http://lila-erc.eu/data/lexicalResources/BrillEDL/id/"
DE_VAAN = "agent:de-vaan"
MODES = {"inheritance": "inherited"}
HEDGE = re.compile(r"\s*\?$")


def local(uri):
    """LiLa's own key for a resource: the Brill entry id, or the path after /id/."""
    uri = str(uri)
    if "dictionary=latin&id=" in uri:
        return uri.rsplit("id=", 1)[1]
    return uri.removeprefix(BASE)


class Converter:
    def __init__(self, graph, extraction):
        self.g = graph
        self.lang = Languages(NAME)
        self.src = source(NAME)
        self.out = A.Writer()
        self.extraction = extraction["id"]
        self.entry_of = {
            ety: entry
            for entry in graph.subjects(RDF.type, ONTOLEX.LexicalEntry)
            for ety in graph.objects(entry, ETY.etymology)
        }
        for o in (self.src, extraction, agent(DE_VAAN, "person", "Michiel de Vaan")):
            self.out.add(o)

    def forms(self, node):
        found = []
        for prop, kind in ((ONTOLEX.canonicalForm, "canonical"), (ONTOLEX.otherForm, "other")):
            for f in self.g.objects(node, prop):
                if isinstance(f, URIRef) and not (f, ONTOLEX.writtenRep, None) in self.g:
                    continue
                for rep in self.g.objects(f, ONTOLEX.writtenRep):
                    features = sorted(
                        local(o).rsplit("#", 1)[-1]
                        for p, o in self.g.predicate_objects(f)
                        if p != ONTOLEX.writtenRep and p != RDF.type
                    )
                    found.append((kind, str(rep), features))
        return sorted(found)

    def entry(self, node):
        rec = {
            "type": "record",
            "id": f"{NAME}:{local(node)}",
            "source": self.src["id"],
            "language": self.lang("Latin"),
            "form": str(self.g.value(node, RDFS.label)),
            "reconstructed": False,
            "locator": str(node),
        }
        lemma = self.g.value(node, ONTOLEX.canonicalForm)
        if isinstance(lemma, URIRef):
            rec["x"] = {NAME: {"lila_lemma": str(lemma)}}
        return self.out.add(rec)

    def etymon(self, node):
        forms = self.forms(node)
        canonical = next((f for k, f, _ in forms if k == "canonical"), None)
        rec = {
            "type": "record",
            "id": f"{NAME}:{local(node)}",
            "source": self.src["id"],
            "language": self.lang(str(self.g.value(node, LIME.language))),
            "form": canonical,
            "reconstructed": True,
            "locator": str(node),
        }
        if canonical and HEDGE.search(canonical):
            rec["form"] = HEDGE.sub("", canonical)
            rec["certainty"] = {"level": "possible", "words": "?"}
        ext = {}
        others = [{"form": f, "features": feats} for k, f, feats in forms if k == "other"]
        if others:
            ext["forms"] = others
        cognates = sorted(local(c) for c in self.g.objects(node, ETY.cognate))
        if cognates:
            ext["cognates"] = cognates
        if ext:
            rec["x"] = {NAME: ext}
        return self.out.add(rec)

    def record(self, node):
        if (node, RDF.type, ONTOLEX.LexicalEntry) in self.g or "dictionary=latin" in str(node):
            return self.entry(node)
        return self.etymon(node)

    def etymology(self, key):
        ety = URIRef(f"{BASE}{key}")
        entry = self.entry_of[ety]
        by_target = {
            self.g.value(link, ETY.etyTarget): link for link in self.g.objects(ety, ETY.hasEtyLink)
        }
        word = self.record(entry)
        links, node = [], entry
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
            node = parent
        if by_target:
            raise ValueError(f"{key}: links that do not chain from the entry: {sorted(by_target)}")
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
        )
        return self.out.add(a)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("sample", type=Path, help="JSON with `etymologies`: LiLa ids (etymology/N)")
    p.add_argument("--out", type=Path, required=True)
    args = p.parse_args(argv)
    ttl = Path(A.REPO / profile(NAME)["raw"])
    g = Graph()
    g.parse(ttl, format="turtle")
    extraction = A.extraction(
        "converter",
        "packages/etymology/convert/lila.py",
        [ttl, SOURCES / NAME / "etc/languages.json", args.sample],
    )
    c = Converter(g, extraction)
    for key in json.loads(args.sample.read_text())["etymologies"]:
        c.etymology(key)
    print(f"{args.out}: {c.out.write(args.out)} objects")


if __name__ == "__main__":
    main()
