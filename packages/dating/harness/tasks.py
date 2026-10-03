"""Task generation (plan §B2): one task per top-level family, split by Glottolog
subgroup for the big ones, ranked by wety relevance = Wiktionary codes in the
subtree weighted by Wiktionary lemma counts (from the 'X lemmas' category
sizes, fetched once and cached)."""

from __future__ import annotations
from workflow_tools.paths import DATING_PRODUCT, DATING_GENERATIONS, DATING_HISTORY

import json
import os
import time
import urllib.parse
import urllib.request

from . import glotto
from .config import CACHE_DIR, DATA_DIR, USER_AGENT, now

SPLIT_ABOVE = 60  # split a family into subgroup tasks when it has more languages than this
MIN_TASK_LANGS = 8  # ... but do not make subgroup tasks smaller than this
API = "https://en.wiktionary.org/w/api.php"
LEMMA_CACHE = os.path.join(CACHE_DIR, "wiktionary_lemma_counts.json")


def lemma_counts(names: list[str], refresh: bool = False) -> dict[str, int]:
    os.makedirs(CACHE_DIR, exist_ok=True)
    cache: dict[str, int] = {}
    if os.path.exists(LEMMA_CACHE) and not refresh:
        with open(LEMMA_CACHE, encoding="utf-8") as f:
            cache = json.load(f)
    todo = [n for n in names if n not in cache]
    for i in range(0, len(todo), 50):
        batch = todo[i : i + 50]
        params = {
            "action": "query",
            "prop": "categoryinfo",
            "titles": "|".join(f"Category:{n} lemmas" for n in batch),
            "format": "json",
            "formatversion": 2,
            "maxlag": 5,
        }
        req = urllib.request.Request(
            API + "?" + urllib.parse.urlencode(params), headers={"User-Agent": USER_AGENT}
        )
        try:
            with urllib.request.urlopen(req, timeout=120) as r:
                data = json.load(r)
        except Exception as e:  # noqa: BLE001
            print(f"  lemma counts: batch failed ({e}); continuing", flush=True)
            time.sleep(5)
            continue
        norm = {x["from"]: x["to"] for x in data["query"].get("normalized", [])}
        pages = {p["title"]: p for p in data["query"]["pages"]}
        for n in batch:
            t = norm.get(f"Category:{n} lemmas", f"Category:{n} lemmas")
            p = pages.get(t, {})
            cache[n] = p.get("categoryinfo", {}).get("pages", 0) if p else 0
        time.sleep(0.2)
        if (i // 50) % 20 == 0:
            print(f"  lemma counts: {min(i + 50, len(todo))}/{len(todo)}", flush=True)
    with open(LEMMA_CACHE, "w", encoding="utf-8") as f:
        json.dump(cache, f, ensure_ascii=False)
    return cache


def build_tasks(refresh: bool = False) -> list[dict]:
    g = glotto.load()
    names = sorted(
        {rec["name"] for rec in g.wikt.values() if rec.get("kind") in ("regular", "reconstructed")}
    )
    counts = lemma_counts(names, refresh=refresh)
    # weight per Glottolog node: Wiktionary codes mapped there and their lemma counts
    node_codes: dict[str, list[str]] = {}
    for code, rec in g.wikt.items():
        node = (
            rec.get("glottocode")
            if rec.get("kind") != "reconstructed"
            else rec.get("glottolog_family_glottocode")
        )
        if node:
            node_codes.setdefault(node, []).append(code)

    def subtree_stats(root: str, excluding: list[str] = ()) -> tuple[int, int, int]:
        nodes, stack = [root], [root]
        while stack:
            n = stack.pop()
            for k in g.children.get(n, []):
                if k in excluding:
                    continue
                nodes.append(k)
                stack.append(k)
        langs = sum(1 for n in nodes if g.rows[n]["level"] == "language")
        codes = [c for n in nodes for c in node_codes.get(n, [])]
        lemmas = sum(counts.get(g.wikt[c]["name"], 0) for c in codes)
        return langs, len(codes), lemmas

    tasks = []
    tops = sorted({g.top_family(gid) for gid in g.rows if g.rows[gid]["level"] != "dialect"})
    for top in tops:
        if top in (glotto.ISOLATES_BUCKET, glotto.UNCLASSIFIED_BUCKET):
            members = [
                gid
                for gid in g.rows
                if g.rows[gid]["level"] == "language" and g.top_family(gid) == top
            ]
            codes = [c for n in members for c in node_codes.get(n, [])]
            tasks.append(
                {
                    "glottocode": top,
                    "name": top,
                    "parent_task": None,
                    "languoids": len(members),
                    "wiktionary_codes": len(codes),
                    "lemmas": sum(counts.get(g.wikt[c]["name"], 0) for c in codes),
                    "subtree_lemmas": sum(counts.get(g.wikt[c]["name"], 0) for c in codes),
                    "members": members if top == glotto.ISOLATES_BUCKET else None,
                }
            )
            continue

        # split big families at subgroup nodes; chain nodes (one big child, tiny residue)
        # are folded into the nearest task above them
        def emit(node: str, parent_task: str | None) -> None:
            langs, _, _ = subtree_stats(node)
            kids = [k for k in g.children.get(node, []) if g.rows[k]["level"] == "family"]
            big = (
                [k for k in kids if subtree_stats(k)[0] >= MIN_TASK_LANGS]
                if langs > SPLIT_ABOVE
                else []
            )
            r_langs, r_codes, r_lemmas = subtree_stats(node, big)
            is_task = node == top or r_langs >= MIN_TASK_LANGS or not big
            if is_task:
                _, _, all_lemmas = subtree_stats(node)
                t = {
                    "glottocode": node,
                    "name": g.rows[node]["name"],
                    "parent_task": parent_task,
                    "languoids": r_langs,
                    "wiktionary_codes": r_codes,
                    "lemmas": r_lemmas,
                    "subtree_lemmas": all_lemmas,
                }
                if big:
                    t["excluding"] = big
                    t["note"] = (
                        "covers this node's own age and the members not in the listed subgroup tasks"
                    )
                tasks.append(t)
                parent_task = node
            else:
                # chain node: its own node age belongs to the parent task; record that
                for t in tasks:
                    if t["glottocode"] == parent_task:
                        t.setdefault("also_covers_nodes", []).append(node)
            for k in big:
                emit(k, parent_task)

        emit(top, None)
    # rank by the whole subtree's weight: a node's own age matters to everything below it
    tasks.sort(key=lambda t: (-t["subtree_lemmas"], -t["wiktionary_codes"], -t["languoids"]))
    for i, t in enumerate(tasks, 1):
        t["rank"] = i
    d = os.path.join(DATING_PRODUCT)
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "tasks.json"), "w", encoding="utf-8") as f:
        json.dump(
            {"generated": now(), "split_above": SPLIT_ABOVE, "tasks": tasks},
            f,
            ensure_ascii=False,
            indent=1,
        )
    return tasks


def task_subtree(glottocode: str) -> list[str]:
    """Languoids (language + family level) a task is responsible for."""
    g = glotto.load()
    path = os.path.join(DATING_PRODUCT, "tasks.json")
    excluding: set[str] = set()
    if os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            all_tasks = json.load(f)["tasks"]
        for t in all_tasks:
            if t["glottocode"] == glottocode:
                if t.get("members"):
                    return t["members"]
        # everything that is another task's root is excluded from this one
        excluding = {t["glottocode"] for t in all_tasks if t["glottocode"] != glottocode}
    out = [glottocode]
    stack = [glottocode]
    while stack:
        n = stack.pop()
        for k in g.children.get(n, []):
            if k in excluding or g.rows[k]["level"] == "dialect":
                continue
            out.append(k)
            stack.append(k)
    return out
