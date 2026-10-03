"""The Glottolog tree and the Wiktionary cross-reference, loaded from
data/products/catalogs/ (build that first)."""

from __future__ import annotations

import csv
import hashlib
from pathlib import Path
import json
import os
from collections import defaultdict
from functools import lru_cache

from .config import LANGCODES_DIR

ISOLATES_BUCKET = "isolates"
UNCLASSIFIED_BUCKET = "unclassified"


class Glottolog:
    def __init__(self) -> None:
        root=Path(LANGCODES_DIR)
        from catalogs import registry as catalog
        errors=catalog.validate_current(root)
        if errors:raise ValueError('invalid cross-reference registry: '+str(errors[:5]))
        bundle=catalog.current_bundle(root)
        pointer={'generation':catalog.digest(bundle)}
        self.crossref_generation=pointer['generation']
        self.rows={r['glottocode']:r for r in bundle['glottolog']}
        self.children=defaultdict(list)
        for g in self.rows:
            p=self.parent(g)
            if p:self.children[p].append(g)
        self.wikt=bundle['languages']
        self.wikt_families=bundle['families']
        self.wikt_by_glotto: dict[str, list[str]] = defaultdict(list)
        for code, rec in self.wikt.items():
            if rec.get("glottocode"):
                self.wikt_by_glotto[rec["glottocode"]].append(code)
        self.proto_by_family_node: dict[str, list[str]] = defaultdict(list)
        # An explicit family.protoLanguage relation is context, not equality
        # between the reconstructed stage and a phylogenetic crown.
        for family, rec in self.wikt_families.items():
            proto=rec.get('proto_language');gid=rec.get('glottocode')
            if proto in self.wikt and gid:self.proto_by_family_node[gid].append(proto)
        self.wikt_family_by_node: dict[str, list[str]] = defaultdict(list)
        for code, rec in self.wikt_families.items():
            if rec.get("glottocode"):
                self.wikt_family_by_node[rec["glottocode"]].append(code)

    def path(self, g: str) -> list[str]:
        r = self.rows.get(g)
        if not r:
            return []
        cls = r["classification"] or ""
        return (cls.split("/") if cls else []) + [g]

    def parent(self, g: str) -> str | None:
        p = self.path(g)
        return p[-2] if len(p) > 1 else None

    def top_family(self, g: str) -> str:
        """The top-level bucket a languoid belongs to (its family_id, itself if a
        top-level family, or the isolate/unclassified buckets)."""
        r = self.rows[g]
        if not self.genetic_path(g):return UNCLASSIFIED_BUCKET
        if r['family_id']:return r['family_id']
        return g if r['level']=='family' else ISOLATES_BUCKET

    def genetic_path(self, g: str) -> list[str]:
        """Only catalog classifications marked as spoken L1 genealogy.

        Sign-language families can have genealogy; this broad Glottolog category
        is not itself one family. Those need a separately reviewed graph.
        """
        path=self.path(g)
        nongenealogical={"Unclassifiable","Bookkeeping","Unattested","Artificial_Language","Pidgin","Mixed_Language","Speech_Register","Sign_Language"}
        return [] if any(self.rows[x].get('category') in nongenealogical for x in path) else path

    def descendants(self, g: str) -> list[str]:
        out, stack = [], [g]
        while stack:
            x = stack.pop()
            for c in self.children.get(x, []):
                out.append(c)
                stack.append(c)
        return out

    def mrca(self, codes: list[str]) -> str | None:
        if not codes or any(c not in self.rows for c in codes):return None
        paths = [self.genetic_path(c) for c in codes]
        if any(not p for p in paths):return None
        if not paths:
            return None
        common = paths[0]
        for p in paths[1:]:
            n = 0
            while n < len(common) and n < len(p) and common[n] == p[n]:
                n += 1
            common = common[:n]
        return common[-1] if common else None

    def status(self, g: str) -> str:
        r = self.rows[g]
        # Catalog AES concerns the catalog entity only. ISO/Wikidata labels
        # on a linked concept cannot resolve its transmission history.
        if r['level']=='family':return 'unknown'
        if r.get('category')=='Artificial_Language':return 'constructed'
        if r.get('aes')=='6':return 'extinct'
        if r.get('aes') in ('1','2','3','4','5'):return 'living'
        return 'unknown'

    def wiktionary_info(self, g: str) -> dict:
        return {
            "codes": sorted(self.wikt_by_glotto.get(g, [])),
            "families": sorted(self.wikt_family_by_node.get(g, [])),
            "proto_languages": sorted(self.proto_by_family_node.get(g, [])),
        }


@lru_cache(maxsize=1)
def load() -> Glottolog:
    return Glottolog()
