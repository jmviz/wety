"""Literature discovery (plan §B3): fixed query templates against OpenAlex
(primary) and Semantic Scholar (secondary), every query logged verbatim with
its hit count; candidates screened on title + abstract by a cheap model with a
fixed prompt; every include/exclude decision logged.

Environment: OPENALEX_API_KEY (optional; the free pool allows only a handful of
searches per day), S2_API_KEY (optional)."""

from __future__ import annotations
from .io_utils import atomic_json

import json
import os
import re
import time
import urllib.parse
import urllib.request

from . import glotto
from .config import CACHE_DIR, USER_AGENT, family_dir, load_env, now
from .store import append_log
from .tasks import task_subtree

OPENALEX = "https://api.openalex.org/works"
S2 = "https://api.semanticscholar.org/graph/v1/paper/search"
MAILTO = "jmvizdev@gmail.com"

QUERY_TEMPLATES = {
    "phylo": '"{name}" (phylogenetic OR phylogeny OR Bayesian OR "divergence" OR "time depth" OR chronology OR dating OR homeland)',
    "proto": '"Proto-{name}" (date OR age OR dating OR homeland OR chronology)',
    "methods": '"{name}" (calibration OR critique OR reconstruction OR loanwords OR "relative chronology" OR subgrouping)',
    "attest": '"{name} languages" (attestation OR inscription OR "earliest" OR extinct OR "last speaker" OR history)',
}


def _get_json(url: str, headers: dict | None = None, retries: int = 4) -> dict:
    load_env()
    h = {"User-Agent": USER_AGENT, **(headers or {})}
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers=h)
            with urllib.request.urlopen(req, timeout=120) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:  # noqa: PERF203
            if e.code in (429, 500, 502, 503) and attempt < retries - 1:
                time.sleep(10 * (attempt + 1))
                continue
            raise
    raise RuntimeError("unreachable")


def _abstract(inv: dict | None) -> str | None:
    if not inv:
        return None
    words: dict[int, str] = {}
    for w, positions in inv.items():
        for p in positions:
            words[p] = w
    return " ".join(words[i] for i in sorted(words))


def openalex_search(query: str, per_page: int = 50) -> list[dict]:
    params = {"search": query, "per-page": per_page, "mailto": MAILTO,
              "select": "id,doi,title,publication_year,authorships,abstract_inverted_index,open_access,primary_location,locations,ids,cited_by_count,type"}
    key = os.environ.get("OPENALEX_API_KEY")
    if key:
        params["api_key"] = key
    data = _get_json(OPENALEX + "?" + urllib.parse.urlencode(params))
    out = []
    for w in data.get("results", []):
        out.append({
            "provider": "openalex", "id": w["id"], "doi": (w.get("doi") or "").replace("https://doi.org/", "") or None,
            "title": w.get("title"), "year": w.get("publication_year"),
            "authors": [a["author"]["display_name"] for a in w.get("authorships", [])[:6] if a.get("author")],
            "abstract": _abstract(w.get("abstract_inverted_index")),
            "oa_url": (w.get("open_access") or {}).get("oa_url"),
            "pdf_url": ((w.get("primary_location") or {}).get("pdf_url")),
            "venue": ((w.get("primary_location") or {}).get("source") or {}).get("display_name"),
            "cited_by": w.get("cited_by_count"), "type": w.get("type"),
            "pmcid": (w.get("ids") or {}).get("pmcid"),
            "oa_locations": sorted({u for loc in (w.get("locations") or []) if loc.get("is_oa")
                                    for u in (loc.get("pdf_url"), loc.get("landing_page_url")) if u}),
        })
    return out


def s2_search(query: str, limit: int = 100) -> list[dict]:
    from . import s2
    return s2.search_bulk(query, limit=limit)


def s2_enrich_candidates(found: dict[str, dict]) -> int:
    """One POST /paper/batch per 500 candidates: S2 ids, PMCID/ArXiv ids, open-access
    PDF links, TL;DRs, citation counts — for candidates that came from OpenAlex too."""
    from . import s2
    ids, keys = [], []
    for key, c in found.items():
        if c.get("s2_id") or c.get("s2_unknown"):
            continue
        if c.get("doi"):
            ids.append("DOI:" + c["doi"]); keys.append(key)
    if not ids:
        return 0
    got = s2.enrich(ids)
    n = 0
    for rid, key in zip(ids, keys):
        rec = got.get(rid)
        if not rec:
            found[key]["s2_unknown"] = True
            continue
        c = found[key]
        for k, v in rec.items():
            if v not in (None, [], "") and k not in ("provider", "id") and not c.get(k):
                c[k] = v
        n += 1
    return n


def s2_citation_walk(family: str, found: dict[str, dict], seeds: list[dict], per_seed: int = 200, must_mention: list[str] | None = None) -> int:
    """Candidates citing / cited by the seed papers, carrying the citation contexts
    (the sentences in which the seed's result is described)."""
    from . import s2
    added = 0
    for seed in seeds:
        sid = seed.get("s2_id") or (f"DOI:{seed['doi']}" if seed.get("doi") else None)
        if not sid:
            continue
        for direction, fn in (("cites", s2.citations), ("cited-by", s2.references)):
            try:
                hits = fn(sid, limit=per_seed)
            except Exception as e:  # noqa: BLE001
                append_log(family, "search_log.jsonl", {"provider": "s2", "template": f"walk:{direction}", "seed": sid, "error": str(e)})
                continue
            append_log(family, "search_log.jsonl", {"provider": "s2", "template": f"walk:{direction}", "seed": sid, "hits": len(hits), "ids": [h["id"] for h in hits]})
            for h in hits:
                key = h["doi"] or h["id"]
                if key not in found:
                    found[key] = {"key": key, **h, "found_by": [], "screen": None}
                    # Family-name absence is a relevance feature, never a hard exclusion.
                    found[key]["family_name_match"] = any(n.casefold() in f"{h.get('title')} {h.get('abstract')}".casefold() for n in (must_mention or []))
                    added += 1
                tag = f"s2:{direction}:{seed.get('key', sid)}"
                if tag not in found[key]["found_by"]:
                    found[key]["found_by"].append(tag)
                if h.get("contexts"):
                    found[key].setdefault("citation_contexts", {})[seed.get("key", sid)] = h["contexts"][:5]
            time.sleep(0.2)
    return added


def s2_recommend(family: str, found: dict[str, dict], limit: int = 100) -> int:
    """Second discovery round from the screen decisions: included papers as
    positive examples, excluded as negative."""
    from . import s2
    pos = [c["s2_id"] for c in found.values() if c.get("s2_id") and (c.get("screen") or {}).get("decision") == "include"]
    neg = [c["s2_id"] for c in found.values() if c.get("s2_id") and (c.get("screen") or {}).get("decision") == "exclude"]
    if len(pos) < 2:
        return 0
    hits = s2.recommend(pos, neg, limit=limit)
    append_log(family, "search_log.jsonl", {"provider": "s2", "template": "recommend", "positives": len(pos), "negatives": len(neg), "hits": len(hits), "ids": [h["id"] for h in hits]})
    added = 0
    for h in hits:
        key = h["doi"] or h["id"]
        if key not in found:
            found[key] = {"key": key, **h, "found_by": ["s2:recommend"], "screen": None}
            added += 1
    return added


def s2_followups(family: str, found: dict[str, dict]) -> int:
    """Primary sources that extracted claims only report second-hand ("secondary:
    reported from Janhunen 2009") become search targets: S2 bulk search on the
    citation string plus the family name; new hits enter the candidate pool."""
    from . import glotto, s2
    from .store import load_claims
    name = glotto.load().rows[family]["name"] if family in glotto.load().rows else family
    wanted: dict[str, int] = {}
    for c in load_claims(family):
        n = c.get("notes") or ""
        if n.startswith("secondary: reported from "):
            ref = n.removeprefix("secondary: reported from ").strip(" .")
            if 3 < len(ref) < 120:
                wanted[ref] = wanted.get(ref, 0) + 1
    added = 0
    for ref, n in sorted(wanted.items(), key=lambda kv: -kv[1])[:25]:
        q = " ".join(t for t in re.findall(r"[A-Za-zÀ-ž]{3,}|\d{4}", ref) if t.lower() not in ("and", "et", "al", "the"))
        year = re.search(r"\b(1[89]\d\d|20\d\d)\b", ref)
        try:
            hits = s2.search_bulk(f'"{name}" {q}', limit=5, year=year.group(1) if year else None)
        except Exception as e:  # noqa: BLE001
            append_log(family, "search_log.jsonl", {"provider": "s2", "template": "followup", "ref": ref, "error": str(e)})
            continue
        append_log(family, "search_log.jsonl", {"provider": "s2", "template": "followup", "ref": ref, "mentions": n, "hits": len(hits), "ids": [h["id"] for h in hits]})
        for h in hits:
            key = h["doi"] or h["id"]
            if key not in found:
                found[key] = {"key": key, **h, "found_by": [f"s2:followup:{ref[:40]}"], "screen": None}
                added += 1
    return added


def candidates_path(family: str) -> str:
    return os.path.join(family_dir(family), "candidates.json")


DATING_WORDS = re.compile(r"\b(phylogen\w*|bayesian|divergence|time depth|time-depth|chronolog\w*|dating|dated|date[sd]?\b|homeland|"
                          r"glottochronolog\w*|lexicostatist\w*|attest\w*|inscription\w*|extinct\w*|last speaker|years? (?:bp|ago)|kya|"
                          r"millenni\w*|century|centuries|bce?\b|calibrat\w*|origin[s]? of|age of|expansion|dispersal)\b", re.I)
GENETICS_WORDS = re.compile(r"\b(genom\w*|ancient dna|admixture|haplogroup|allele|mitochondrial|y-chromosom\w*|snp)\b", re.I)


def keyword_screen(cand: dict) -> dict:
    """Deterministic fallback screen: dating vocabulary in title/abstract and not a
    genetics paper. Clearly labelled so it is never mistaken for the model screen."""
    text = f"{cand.get('title') or ''} {cand.get('abstract') or ''}"
    dating = len(DATING_WORDS.findall(text))
    genetics = len(GENETICS_WORDS.findall(text))
    methodology = bool(re.search(r"calibrat|critique|reconstruct|loanword|relative chronology|subgroup",text,re.I))
    include = (dating >= 2 or methodology) and genetics <= dating and (cand.get("type") in (None, "article", "book-chapter", "preprint", "book", "dissertation"))
    return {"decision": "include" if include else "exclude", "reason": f"keyword screen: dating={dating} genetics={genetics}", "model": "keyword", "at": now()}


def discover(family: str, max_results: int = 60, screen: str = "llm", providers: tuple[str, ...] | None = None, batch: bool = False,
             walk: bool = True, recommend: bool = False, followups: bool = True) -> dict:
    load_env()
    if providers is None:
        # Semantic Scholar rate-limits hard without a key and each failed query costs ~2 min of backoff
        providers = ("openalex", "s2") if os.environ.get("S2_API_KEY") else ("openalex",)
    g = glotto.load()
    name = g.rows[family]["name"] if family in g.rows else family
    subtree = sorted({family,*g.descendants(family)}) if family in g.rows else task_subtree(family)
    # names of the family's own subgroups, to find subgroup studies
    aliases={"indo1319":["Indo-Anatolian","Indo-Hittite"],"ural1272":["Finno-Ugric","Finno-Permic"],"sino1245":["Trans-Himalayan","Tibeto-Burman"]}
    names = [name] + aliases.get(family,[]) + [g.rows[x]["name"] for x in subtree if g.rows[x]["level"] == "family" and x != family][:10]
    queries = []
    for tpl_name, tpl in QUERY_TEMPLATES.items():
        for n in names[: (1 if tpl_name != "phylo" else 6)]:
            q = tpl.format(name=n)
            if n != name:
                q = q   # a subgroup name alone ("Mari", "Kuki") is too generic; anchor it to the family
            queries.append((tpl_name, q))
    found: dict[str, dict] = {}
    path = candidates_path(family)
    if os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            found = {c["key"]: c for c in json.load(f)}
    for tpl_name, q in queries:
        for provider in providers:
            try:
                hits = openalex_search(q, min(max_results, 50)) if provider == "openalex" else s2_search(q, min(max_results, 50))
            except Exception as e:  # noqa: BLE001
                append_log(family, "search_log.jsonl", {"provider": provider, "template": tpl_name, "query": q, "error": str(e)})
                print(f"  {provider} {tpl_name}: error {e}", flush=True)
                continue
            append_log(family, "search_log.jsonl", {"provider": provider, "template": tpl_name, "query": q, "hits": len(hits),
                                                     "ids": [h["id"] for h in hits]})
            print(f"  {provider} {tpl_name} '{q[:60]}': {len(hits)} hits", flush=True)
            for h in hits:
                key = h["doi"] or h["id"]
                if key not in found:
                    found[key] = {"key": key, **h, "found_by": [], "screen": None}
                else:  # refresh metadata fields (new OA locations, ids) without touching screen/retrieval state
                    for k, v in h.items():
                        if v not in (None, [], "") and k not in ("provider", "id"):
                            found[key][k] = v
                if f"{provider}:{tpl_name}" not in found[key]["found_by"]:
                    found[key]["found_by"].append(f"{provider}:{tpl_name}")
            time.sleep(1.0)
    summary = {"family": family, "queries": len(queries), "candidates": len(found), "at": now()}
    if os.environ.get("S2_API_KEY"):
        try:
            summary["s2_enriched"] = s2_enrich_candidates(found)
        except Exception as e:  # noqa: BLE001
            summary["s2_enrich_error"] = str(e)[:200]
    atomic_json(path, sorted(found.values(), key=lambda c: -(c.get("cited_by") or 0)))
    if screen == "llm":
        from .llm import screen_candidates
        summary["screened"] = screen_candidates(family, subtree_names=names, batch=batch)
        if (walk or recommend or followups) and os.environ.get("S2_API_KEY") and not batch:
            with open(path, encoding="utf-8") as f:
                found = {c["key"]: c for c in json.load(f)}
            try:
                added = 0
                if walk:
                    # seeds: the most-cited papers the screen INCLUDED (never raw citation counts —
                    # off-topic hits from the search are often the most-cited candidates)
                    seeds = sorted((c for c in found.values() if (c.get("screen") or {}).get("decision") == "include" and (c.get("doi") or c.get("s2_id"))),
                                   key=lambda c: -(c.get("influential_citations") or 0) * 10 - (c.get("cited_by") or 0))[:5]
                    try:
                        added += s2_citation_walk(family, found, seeds, must_mention=names)
                    except Exception as e:
                        summary["s2_walk_error"] = str(e)[:200]
                    try: s2_enrich_candidates(found)
                    except Exception as e: summary["s2_enrich_error"] = str(e)[:200]
                if recommend:
                    try: added += s2_recommend(family, found, limit=50)
                    except Exception as e: summary["s2_recommend_error"] = str(e)[:200]
                if followups:
                    added += s2_followups(family, found)
                if added:
                    atomic_json(path, list(found.values()))
                    summary["s2_walk_and_recommend_added"] = added
                    summary["screened_round2"] = screen_candidates(family, subtree_names=names, batch=batch)
            except Exception as e:  # noqa: BLE001
                summary["s2_recommend_error"] = str(e)[:200]
    elif screen == "keyword":
        with open(path, encoding="utf-8") as f:
            cands = json.load(f)
        counts = {"include": 0, "exclude": 0}
        for c in cands:
            if not c.get("screen") or c["screen"].get("model") == "keyword":
                c["screen"] = keyword_screen(c)
                append_log(family, "screen_log.jsonl", {"key": c["key"], "title": c.get("title"), "screen": c["screen"]})
            counts[c["screen"]["decision"]] += 1
        atomic_json(path, cands)
        summary["screened"] = counts
    return summary
