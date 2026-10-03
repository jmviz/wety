"""Dated phylogenies from D-PLACE (github.com/D-PLACE/dplace-data/phylogenies).

For every published tree whose `scaling` in index.csv is years or millennia,
the summary (MCC) tree is parsed, node ages are computed from root-to-tip
distances (the youngest tip = present), each internal node is mapped to the
Glottolog MRCA of its tips, and — when the tree's tips under that Glottolog
node are exactly the node's tips (topology consistent with Glottolog) — a
`phylo_node_age` crown claim is emitted. Trees with `scaling` = change are
topology-only and skipped. Present is assumed 2000 CE (recorded)."""

from __future__ import annotations

import csv
import io
import json
import os
import re
import time
import urllib.request

from .. import glotto
from ..config import CACHE_DIR, USER_AGENT, now
from ..schema import Claim, MethodNotes, Source, TimeValue
from ..time_utils import from_bp
from . import claim_id, write_claims

RAW = "https://raw.githubusercontent.com/D-PLACE/dplace-data/master/phylogenies/"
PRESENT = 2000


def _get(url: str) -> str:
    os.makedirs(os.path.join(CACHE_DIR, "dplace"), exist_ok=True)
    path = os.path.join(CACHE_DIR, "dplace", re.sub(r"[^A-Za-z0-9._-]", "_", url.removeprefix(RAW)))
    if os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            return f.read()
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=120) as r:
        text = r.read().decode("utf-8")
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)
    time.sleep(0.2)
    return text


# ---- newick / nexus -------------------------------------------------------- #


class Node:
    __slots__ = ("name", "length", "children", "parent", "annotations")

    def __init__(self) -> None:
        self.annotations: dict = {}
        self.name: str | None = None
        self.length: float = 0.0
        self.children: list[Node] = []
        self.parent: Node | None = None

    def tips(self) -> list["Node"]:
        return [self] if not self.children else [t for c in self.children for t in c.tips()]

    def walk(self):
        yield self
        for c in self.children:
            yield from c.walk()


def parse_newick(s: str) -> Node:
    """Parse Newick without destroying BEAST node annotations."""
    import json

    s = s.strip()
    i = 0

    def space():
        nonlocal i
        while i < len(s) and s[i].isspace():
            i += 1

    def comments(n):
        nonlocal i
        space()
        while i < len(s) and s[i] == "[":
            j = s.index("]", i)
            raw = s[i + 1 : j].lstrip("&")
            n.annotations.setdefault("_raw", []).append(raw)
            for m in re.finditer(r"([A-Za-z0-9_%]+)=({[^}]*}|[^,]+)", raw):
                k, v = m.groups()
                try:
                    n.annotations[k] = (
                        [float(x) for x in v[1:-1].split(",")] if v.startswith("{") else float(v)
                    )
                except ValueError:
                    n.annotations[k] = v
            i = j + 1
            space()

    def label():
        nonlocal i
        space()
        if i < len(s) and s[i] == "'":
            i += 1
            out = ""
            while i < len(s):
                if s[i] == "'":
                    i += 1
                    if i < len(s) and s[i] == "'":
                        out += "'"
                        i += 1
                        continue
                    return out
                out += s[i]
                i += 1
            raise ValueError("unterminated quoted label")
        j = i
        while i < len(s) and s[i] not in ",:;()[]":
            i += 1
        return s[j:i].strip()

    def node():
        nonlocal i
        space()
        n = Node()
        comments(n)
        if i < len(s) and s[i] == "(":
            i += 1
            while True:
                c = node()
                c.parent = n
                n.children.append(c)
                space()
                if i >= len(s):
                    raise ValueError("unclosed tree")
                ch = s[i]
                i += 1
                if ch == ")":
                    break
                if ch != ",":
                    raise ValueError("invalid child separator")
        n.name = label() or None
        comments(n)
        if i < len(s) and s[i] == ":":
            i += 1
            j = i
            while i < len(s) and s[i] not in ",;()[]":
                i += 1
            n.length = float(s[j:i])
            comments(n)
            if n.length < 0:
                raise ValueError("negative branch length")
        return n

    root = node()
    space()
    if i < len(s) and s[i] == ";":
        i += 1
    space()
    if i != len(s):
        raise ValueError("trailing unparsed tree content")
    return root


def parse_nexus_trees(text: str) -> list[tuple[str, Node]]:
    translate: dict[str, str] = {}
    m = re.search(r"\btranslate\b(.*?);", text, re.S | re.I)
    if m:
        for part in m.group(1).split(","):
            kv = part.strip().split(None, 1)
            if len(kv) == 2:
                translate[kv[0]] = kv[1].strip().strip("'")
    trees = []
    for tm in re.finditer(r"\btree\s+(\S+)\s*=\s*(?:\[[^\]]*\]\s*)*(\(.*?;)", text, re.S | re.I):
        root = parse_newick(tm.group(2))
        if translate:
            for n in root.walk():
                if n.name in translate:
                    n.name = translate[n.name]
        trees.append((tm.group(1), root))
    return trees


def node_ages(root: Node) -> dict[int, float]:
    """Age of every node = (max root-to-tip distance) - (root-to-node distance)."""
    depth: dict[int, float] = {}

    def rec(n: Node, d: float) -> None:
        depth[id(n)] = d
        for c in n.children:
            rec(c, d + c.length)

    rec(root, 0.0)
    height = max(depth[id(t)] for t in root.tips())
    return {k: height - v for k, v in depth.items()}


# ---- mapping ----------------------------------------------------------------- #


def map_tree(
    root: Node, taxa: dict[str, str], g: glotto.Glottolog, scale: float
) -> tuple[list[dict], dict]:
    ages = node_ages(root)
    tip_g = {id(t): taxa.get(t.name or "") for t in root.tips()}
    in_tree = {v for v in tip_g.values() if v}
    out, stats = [], {"internal": 0, "mapped": 0, "no_mrca": 0, "inconsistent": 0, "dup": 0}
    seen: dict[str, dict] = {}
    for n in root.walk():
        if not n.children:
            continue
        stats["internal"] += 1
        descendants = n.tips()
        # Unmapped descendants can change clade identity. Never silently prune them.
        if any(not tip_g.get(id(t)) for t in descendants):
            stats["inconsistent"] += 1
            continue
        tips = [tip_g[id(t)] for t in descendants]
        if len(set(tips)) < 2:
            continue
        mrca = g.mrca(tips)
        if not mrca or g.rows[mrca]["level"] != "family":
            stats["no_mrca"] += 1
            continue
        # consistency: every tree tip under the Glottolog node must be under this tree node
        under = {t for t in in_tree if mrca in g.path(t)[:-1]}
        if under != set(tips):
            stats["inconsistent"] += 1
            continue
        age = ages[id(n)] * scale
        rec = {
            "glottocode": mrca,
            "age_bp": age,
            "n_tips": len(set(tips)),
            "taxa": sorted(t.name for t in descendants),
            "annotations": n.annotations,
            "catalog_relation": "sampled_subset",
        }
        if mrca in seen:
            stats["dup"] += 1
            if len(set(tips)) < seen[mrca]["n_tips"] or (
                len(set(tips)) == seen[mrca]["n_tips"] and age >= seen[mrca]["age_bp"]
            ):
                continue
        seen[mrca] = rec
    stats["mapped"] = len(seen)
    return list(seen.values()), stats


def run(only: list[str] | None = None) -> dict:
    """D-PLACE trees use the same explicit analysis manifests as deposited trees.

    Existing v1 results remain archived candidates. A source's scaling field alone
    does not establish its epoch, sampling model, or exact clade identity.
    """
    from ..tree_ingest import ingest_manifests

    return ingest_manifests(studies=only)
