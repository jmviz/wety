"""Semantic Scholar (S2) Academic Graph + Recommendations API client.

Rate limit: the key allows 1 request per second, cumulative across ALL
endpoints and ALL processes using the key. Every request goes through a
file-locked limiter (cache/s2.lock) so parallel harness runs share one clock.

What we use it for (see docs/language-dating-plan.md §B3):
  search_bulk      boolean-syntax search over title+abstract (up to 1000 hits/page), with
                   fieldsOfStudy / year / openAccessPdf filters
  enrich           POST /paper/batch: externalIds (DOI, PMCID, ArXiv), openAccessPdf, tldr,
                   citation counts, publicationTypes for up to 500 candidates in one request
  citations /      citation walks from seed papers, with the citation *contexts* — the
  references       sentences in which a citing paper describes the seed's result
  snippets         full-text snippet search (~500-word excerpts from body text), optionally
                   restricted to given papers: extractable text for paywalled papers
  recommend        positive/negative-example recommendations: included papers as positives,
                   excluded as negatives, for a second discovery round
"""

from __future__ import annotations

import fcntl
import json
import os
import re
import time
import urllib.error
import urllib.parse
import urllib.request

from .config import CACHE_DIR, USER_AGENT, load_env

GRAPH = "https://api.semanticscholar.org/graph/v1"
RECS = "https://api.semanticscholar.org/recommendations/v1"
LOCK = os.path.join(CACHE_DIR, "s2.lock")
MIN_INTERVAL = 1.05          # seconds between requests, globally
PAPER_FIELDS = "paperId,corpusId,externalIds,title,year,publicationDate,venue,publicationVenue,authors,abstract,citationCount,influentialCitationCount,isOpenAccess,openAccessPdf,publicationTypes,s2FieldsOfStudy"
DETAIL_FIELDS = PAPER_FIELDS + ",tldr"   # tldr is not available on the bulk endpoint


class S2Unavailable(RuntimeError):
    pass


def _key() -> str | None:
    load_env()
    return os.environ.get("S2_API_KEY")


def _throttle() -> None:
    """Block until at least MIN_INTERVAL has passed since the last request made
    by ANY process on this machine (fcntl lock + timestamp in the lock file)."""
    os.makedirs(CACHE_DIR, exist_ok=True)
    with open(LOCK, "a+") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        try:
            f.seek(0)
            try:
                last = float(f.read().strip() or 0)
            except ValueError:
                last = 0.0
            wait = last + MIN_INTERVAL - time.time()
            if wait > 0:
                time.sleep(wait)
            f.seek(0)
            f.truncate()
            f.write(repr(time.time()))
            f.flush()
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)


def request(path: str, params: dict | None = None, body: dict | None = None, base: str = GRAPH, retries: int = 5) -> dict | list:
    key = _key()
    if not key:
        raise S2Unavailable("S2_API_KEY is not set")
    url = base + path + ("?" + urllib.parse.urlencode({k: v for k, v in (params or {}).items() if v is not None}, doseq=False) if params else "")
    headers = {"User-Agent": USER_AGENT, "x-api-key": key}
    data = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        headers["Content-Type"] = "application/json"
    for attempt in range(retries):
        _throttle()
        try:
            req = urllib.request.Request(url, data=data, headers=headers, method="POST" if body is not None else "GET")
            with urllib.request.urlopen(req, timeout=120) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return {}
            if e.code in (429, 500, 502, 503, 504) and attempt < retries - 1:
                time.sleep(2 ** attempt * 2)
                continue
            raise RuntimeError(f"S2 HTTP {e.code} {path}: {e.read().decode('utf-8', errors='replace')[:200]}") from e
        except (urllib.error.URLError, TimeoutError):
            if attempt < retries - 1:
                time.sleep(2 ** attempt * 2)
                continue
            raise
    raise RuntimeError("unreachable")


# ---- query syntax ----------------------------------------------------------- #

def to_bulk_query(q: str) -> str:
    """OpenAlex-style 'A OR B' / 'A AND B' -> S2 bulk syntax ('|', '+', quotes kept)."""
    q = re.sub(r"\s+OR\s+", " | ", q)
    q = re.sub(r"\s+AND\s+", " + ", q)
    return q


# ---- records ----------------------------------------------------------------- #

def _record(w: dict) -> dict:
    ext = w.get("externalIds") or {}
    return {
        "provider": "s2", "id": "s2:" + (w.get("paperId") or ""), "s2_id": w.get("paperId"), "corpus_id": w.get("corpusId"),
        "doi": ext.get("DOI"), "pmcid": ext.get("PubMedCentral"), "arxiv": ext.get("ArXiv"),
        "title": w.get("title"), "year": w.get("year"), "authors": [a["name"] for a in (w.get("authors") or [])[:6]],
        "abstract": w.get("abstract"), "tldr": (w.get("tldr") or {}).get("text"),
        "oa_url": (w.get("openAccessPdf") or {}).get("url"), "pdf_url": (w.get("openAccessPdf") or {}).get("url"),
        "venue": w.get("venue") or ((w.get("publicationVenue") or {}).get("name")),
        "cited_by": w.get("citationCount"), "influential_citations": w.get("influentialCitationCount"),
        "type": ",".join(w.get("publicationTypes") or []) or None,
        "s2_fields": [f["category"] for f in (w.get("s2FieldsOfStudy") or [])],
    }


def search_bulk(query: str, limit: int = 200, year: str | None = None, fields_of_study: str | None = None, open_access: bool = False) -> list[dict]:
    params = {"query": to_bulk_query(query), "fields": PAPER_FIELDS, "sort": "citationCount:desc"}
    if year:
        params["year"] = year
    if fields_of_study:
        params["fieldsOfStudy"] = fields_of_study
    if open_access:
        params["openAccessPdf"] = ""
    out, token = [], None
    while len(out) < limit:
        if token:
            params["token"] = token
        d = request("/paper/search/bulk", params)
        out += [_record(w) for w in d.get("data", [])]
        token = d.get("token")
        if not token or not d.get("data"):
            break
    return out[:limit]


def enrich(ids: list[str]) -> dict[str, dict]:
    """POST /paper/batch for up to 500 ids ('DOI:...', 'PMCID:...', s2 sha, 'CorpusId:...')."""
    out: dict[str, dict] = {}
    for i in range(0, len(ids), 500):
        chunk = ids[i : i + 500]
        try:
            res = request("/paper/batch", {"fields": DETAIL_FIELDS}, body={"ids": chunk})
        except RuntimeError as e:
            if "No valid paper ids" in str(e):
                continue
            raise
        for req_id, w in zip(chunk, res if isinstance(res, list) else []):
            if w:
                out[req_id] = _record(w)
    return out


def match_title(title: str) -> dict | None:
    d = request("/paper/search/match", {"query": title, "fields": PAPER_FIELDS})
    data = d.get("data") if isinstance(d, dict) else None
    if data and data[0].get("matchScore", 0) >= 0.9:
        return _record(data[0])
    return None


def citations(paper_id: str, limit: int = 500, with_contexts: bool = True) -> list[dict]:
    """Papers citing `paper_id`, each with the contexts (sentences) of the citation."""
    fields = "contexts,intents,isInfluential," + ",".join("citingPaper." + f for f in PAPER_FIELDS.split(","))
    out, offset = [], 0
    while offset < limit:
        d = request(f"/paper/{paper_id}/citations", {"fields": fields, "offset": offset, "limit": min(500, limit - offset)})
        for item in (d.get("data") or []):
            if not item.get("citingPaper") or not item["citingPaper"].get("paperId"):
                continue
            rec = _record(item["citingPaper"])
            rec.update({"contexts": item.get("contexts") or [], "intents": item.get("intents") or [], "is_influential": item.get("isInfluential")})
            out.append(rec)
        if d.get("next") is None:
            break
        offset = d["next"]
    return out


def references(paper_id: str, limit: int = 500) -> list[dict]:
    fields = "contexts,intents,isInfluential," + ",".join("citedPaper." + f for f in PAPER_FIELDS.split(","))
    out, offset = [], 0
    while offset < limit:
        d = request(f"/paper/{paper_id}/references", {"fields": fields, "offset": offset, "limit": min(500, limit - offset)})
        for item in (d.get("data") or []):
            if not item.get("citedPaper") or not item["citedPaper"].get("paperId"):
                continue
            rec = _record(item["citedPaper"])
            rec.update({"contexts": item.get("contexts") or [], "intents": item.get("intents") or [], "is_influential": item.get("isInfluential")})
            out.append(rec)
        if d.get("next") is None:
            break
        offset = d["next"]
    return out


def snippets(query: str, limit: int = 20, paper_ids: list[str] | None = None, fields_of_study: str | None = None) -> list[dict]:
    """Body-text excerpts (~500 words) matching the query, best first; restrict to
    papers with paper_ids (DOI:..., CorpusId:..., sha) to read inside a specific paper."""
    params = {"query": query, "limit": min(limit, 1000), "fields": "snippet.text,snippet.snippetKind,snippet.section,snippet.snippetOffset"}
    if paper_ids:
        params["paperIds"] = ",".join(paper_ids[:100])
    if fields_of_study:
        params["fieldsOfStudy"] = fields_of_study
    d = request("/snippet/search", params)
    out = []
    for item in (d.get("data") or []):
        sn, paper = item.get("snippet") or {}, item.get("paper") or {}
        out.append({"text": sn.get("text"), "kind": sn.get("snippetKind"), "section": sn.get("section"), "score": item.get("score"),
                    "corpus_id": paper.get("corpusId"), "title": paper.get("title"), "open_access": paper.get("openAccessInfo")})
    return out


def recommend(positive: list[str], negative: list[str] | None = None, limit: int = 100) -> list[dict]:
    d = request("/papers/", {"fields": PAPER_FIELDS, "limit": min(limit, 500)},
                body={"positivePaperIds": positive[:100], "negativePaperIds": (negative or [])[:100]}, base=RECS)
    return [_record(w) for w in (d.get("recommendedPapers") or [])]
