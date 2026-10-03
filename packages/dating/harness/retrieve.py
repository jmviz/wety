"""Full-text retrieval (plan §B4) for included candidates: open-access PDF or
HTML only, cached under its sha256 in cache/texts/, text extracted with
PyMuPDF; abstract-only when nothing open is available (recorded)."""

from __future__ import annotations
from .io_utils import atomic_json

import hashlib
import json
import os
import re
import time
import urllib.request

from .config import CACHE_DIR, USER_AGENT, family_dir, now
from .schema import Source
from .store import upsert_source

TEXTS = os.path.join(CACHE_DIR, "texts")


def source_id_for(cand: dict) -> str:
    if cand.get("doi"):
        return "src_" + re.sub(r"[^a-z0-9]+", "_", cand["doi"].lower()).strip("_")
    return "src_" + re.sub(r"[^a-z0-9]+", "_", cand["key"].lower()).strip("_")


RETRIEVAL_VERSION = 2
MAX_RESPONSE_BYTES = 30 * 1024 * 1024


def assess_text(text, kind):
    """A conservative content diagnostic, not a scientific quality judgment."""
    sections = {
        w
        for w in ("introduction", "methods", "results", "discussion", "conclusions", "conclusion")
        if re.search(r"(?:^|\n)\s*(?:[0-9.]+\s+)?" + w + r"\b", text, re.I)
    }
    if len(text) < 8000 or len(sections) < 2:
        return "partial_or_landing"
    return "fulltext_candidate"


def fetch_text(url: str, _visited=None) -> tuple[str | None, str | None, str | None]:
    """Cache raw bytes and text under their OWN hashes; follow publisher PDF links.

    The return hash always hashes the returned UTF-8 text. A sidecar records the
    raw hash, retrieval URL, parser and quality diagnostics. Never call snippets
    or a catalog page a full text simply because they are long.
    """
    import html as html_module
    import urllib.parse
    from html.parser import HTMLParser
    from .io_utils import atomic_text, atomic_json

    visited = set() if _visited is None else _visited
    if url in visited or len(visited) >= 5:
        return None, None, None
    visited.add(url)
    req = urllib.request.Request(
        url,
        headers={"User-Agent": USER_AGENT, "Accept": "application/pdf, text/html;q=0.9, */*;q=0.5"},
    )
    with urllib.request.urlopen(req, timeout=60) as r:
        raw = r.read(MAX_RESPONSE_BYTES + 1)
        ctype = r.headers.get("Content-Type", "")
        final_url = r.url
    if len(raw) > MAX_RESPONSE_BYTES:
        raise ValueError("response exceeds retrieval size cap")
    rawsha = hashlib.sha256(raw).hexdigest()
    rawdir = os.path.join(CACHE_DIR, "raw")
    os.makedirs(rawdir, exist_ok=True)
    rawpath = os.path.join(rawdir, rawsha)
    # Atomic byte preservation; do not reinterpret source encoding.
    import tempfile

    fd, tmp = tempfile.mkstemp(dir=rawdir)
    with os.fdopen(fd, "wb") as f:
        f.write(raw)
    os.replace(tmp, rawpath)
    links = []
    if raw.startswith(b"%PDF-"):
        import pymupdf

        with pymupdf.open(stream=raw, filetype="pdf") as doc:
            text = "\n".join(
                "[PAGE " + str(i + 1) + "]\n" + page.get_text() for i, page in enumerate(doc)
            )
        kind = "pdf"
    elif "xml" in ctype or raw.lstrip().startswith(b"<?xml"):
        from xml.etree import ElementTree as ET

        root = ET.fromstring(raw)
        text = "\n".join(
            " ".join(e.itertext())
            for e in root.iter()
            if e.tag.rsplit("}", 1)[-1] in ("article-title", "title", "p", "table", "table-wrap")
        )
        kind = "xml"
    else:
        html = raw.decode("utf-8", errors="replace")

        class Parser(HTMLParser):
            def __init__(self):
                super().__init__(convert_charrefs=True)
                self.skip = 0
                self.parts = []
                self.links = []

            def handle_starttag(self, tag, attrs):
                d = dict(attrs)
                if tag in ("script", "style", "nav"):
                    self.skip += 1
                if tag in ("h1", "h2", "h3", "p", "div", "section", "tr", "br"):
                    self.parts.append("\n")
                if (
                    tag == "meta"
                    and d.get("name", "").lower() in ("citation_pdf_url", "eprints.document_url")
                    and d.get("content")
                ):
                    self.links.append(d["content"])
                if tag == "a" and re.search(
                    r"\.pdf(?:[?#]|$)|/pdf(?:[/?]|$)|FULLTEXT", d.get("href", ""), re.I
                ):
                    self.links.append(d["href"])

            def handle_endtag(self, tag):
                if tag in ("script", "style", "nav"):
                    self.skip = max(0, self.skip - 1)

            def handle_data(self, data):
                if not self.skip:
                    self.parts.append(data)

        parser = Parser()
        parser.feed(html)
        text = " ".join(parser.parts)
        kind = "html"
        links = [
            urllib.parse.urljoin(final_url, l)
            for l in parser.links
            if urllib.parse.urljoin(final_url, l).startswith(("https://", "http://"))
        ]
    text = re.sub(r"[ \t]+", " ", text)
    text = re.sub(r"\n\s*\n+", "\n", text)
    quality = assess_text(text, kind)
    if quality != "fulltext_candidate":
        for link in list(dict.fromkeys(links))[:3]:
            try:
                result = fetch_text(link, visited)
                if result[0] and assess_text(result[0], result[2]) == "fulltext_candidate":
                    return result
            except (OSError, ValueError):
                continue
    if not text.strip():
        return None, None, None
    sha = hashlib.sha256(text.encode()).hexdigest()
    os.makedirs(TEXTS, exist_ok=True)
    atomic_text(os.path.join(TEXTS, sha + ".txt"), text)
    atomic_json(
        os.path.join(TEXTS, sha + ".meta.json"),
        {
            "raw_sha256": rawsha,
            "text_sha256": sha,
            "url": final_url,
            "requested_url": url,
            "retrieved": now(),
            "parser": "wety-text-v2",
            "version": RETRIEVAL_VERSION,
            "quality": quality,
            "linked_fulltexts": links,
        },
    )
    return text, sha, kind


def retrieve_family(family: str, datasets: bool = True) -> dict:  # noqa: C901
    """Full texts for included candidates, then their linked datasets (plan §B4b)."""
    from .datasets import retrieve_datasets

    path = os.path.join(family_dir(family), "candidates.json")
    with open(path, encoding="utf-8") as f:
        cands = json.load(f)
    got = abstract_only = failed = partial = 0
    ds_stats = {"papers_with_datasets": 0, "datasets": 0, "files": 0, "errors": 0}
    for c in cands:
        if not c.get("screen") or c["screen"].get("decision") != "include":
            continue
        if (
            datasets
            and c.get("datasets_discovery") != DATASETS_DISCOVERY_VERSION
            and c.get("retrieved")
        ):
            _datasets_for(c, ds_stats)
        snippets_possible = bool(
            os.environ.get("S2_API_KEY")
            and (c.get("doi") or c.get("s2_id"))
            and not c.get("s2_snippets_tried")
        )
        if (
            c.get("retrieved", {}).get("version") == RETRIEVAL_VERSION
            and c.get("retrieved", {}).get("quality") == "fulltext_candidate"
            and cached_text(c) is not None
        ):
            if c["retrieved"].get("sha256"):
                got += 1
            elif c.get("abstract"):
                abstract_only += 1
            else:
                failed += 1
            continue
        sid = source_id_for(c)
        previous_retrieval = c.get("retrieved", {})
        text = sha = kind = None
        best = None
        urls = []
        if c.get("pmcid"):
            pmc = c["pmcid"].rsplit("/", 1)[-1].upper()
            urls.append(f"https://www.ebi.ac.uk/europepmc/webservices/rest/{pmc}/fullTextXML")
        urls += [c.get("pdf_url"), c.get("oa_url"), *(c.get("oa_locations") or [])]
        seen_urls: set[str] = set()
        for url in urls:
            if not url or url in seen_urls:
                continue
            seen_urls.add(url)
            try:
                text, sha, kind = fetch_text(url)
                if text and (best is None or len(text) > len(best[0])):
                    best = (text, sha, kind)
                if text and assess_text(text, kind) == "fulltext_candidate":
                    best = (text, sha, kind)
                    break
            except Exception as e:  # noqa: BLE001
                c.setdefault("retrieve_errors", []).append(f"{url}: {e}")
            time.sleep(1.0)
        if best:
            text, sha, kind = best
        if not text and snippets_possible:
            # paywalled: read inside the paper through Semantic Scholar's full-text snippet index
            text, sha, kind = s2_snippet_text(c, family)
            c["s2_snippets_tried"] = True
        meta_path = os.path.join(TEXTS, (sha or "") + ".meta.json")
        meta = json.load(open(meta_path)) if os.path.exists(meta_path) else {}
        src = Source(
            id=sid,
            type="paper",
            title=c.get("title") or sid,
            authors=c.get("authors") or [],
            year=c.get("year"),
            doi=c.get("doi"),
            url=c.get("oa_url")
            or c.get("pdf_url")
            or (f"https://doi.org/{c['doi']}" if c.get("doi") else None),
            retrieved=now(),
            sha256=sha if text else None,
            raw_sha256=meta.get("raw_sha256"),
            retrieval_quality=meta.get("quality", "excerpt_or_abstract"),
        )
        upsert_source(src)
        c["source_id"] = sid
        if text:
            c["retrieved"] = {
                "sha256": sha,
                "raw_sha256": meta.get("raw_sha256"),
                "kind": kind,
                "chars": len(text),
                "at": now(),
                "version": RETRIEVAL_VERSION,
                "quality": meta.get("quality", "excerpt"),
                "url": meta.get("url"),
            }
            got += int(meta.get("quality") == "fulltext_candidate")
            partial += int(meta.get("quality") != "fulltext_candidate")
        else:
            c["retrieved"] = {
                "sha256": None,
                "kind": "abstract-only",
                "chars": len(c.get("abstract") or ""),
                "at": now(),
            }
            abstract_only += 1 if c.get("abstract") else 0
            failed += 0 if c.get("abstract") else 1
        if previous_retrieval and previous_retrieval.get("sha256") != c["retrieved"].get("sha256"):
            c.setdefault("retrieval_history", []).append(previous_retrieval)
            if c.get("extracted"):
                c.setdefault("extraction_history", []).append(c.pop("extracted"))
        if datasets and c.get("datasets_discovery") != DATASETS_DISCOVERY_VERSION:
            _datasets_for(c, ds_stats)
            atomic_json(path, cands)
    atomic_json(path, cands)
    return {
        "family": family,
        "fulltext_candidates": got,
        "partial_or_excerpt": partial,
        "abstract_only": abstract_only,
        "nothing": failed,
        **ds_stats,
    }


DATASETS_DISCOVERY_VERSION = 2  # bump when discovery routes change so candidates are re-scanned


def _datasets_for(c: dict, stats: dict) -> None:
    from .datasets import retrieve_datasets

    try:
        ds = retrieve_datasets(c, text_for(c))
        c["datasets_discovery"] = DATASETS_DISCOVERY_VERSION
    except Exception as e:  # noqa: BLE001
        c.setdefault("retrieve_errors", []).append(f"datasets: {e}")
        stats["errors"] += 1
        return
    if ds:
        stats["papers_with_datasets"] += 1
        stats["datasets"] += len(ds)
        stats["files"] += sum(len(d.get("files", [])) for d in ds)
        stats["errors"] += sum(1 for d in ds if d.get("error"))


SNIPPET_QUERIES = [
    "{family} date divergence years BP",
    "Proto-{family} dated",
    "{family} split chronology BC",
    "earliest attested {family} language",
    "{family} extinct last speakers",
    "calibration {family} phylogenetic",
]


def s2_snippet_text(c: dict, family: str) -> tuple[str | None, str | None, str | None]:
    from . import glotto, s2
    from .config import load_env

    load_env()
    name = glotto.load().rows[family]["name"] if family in glotto.load().rows else family
    pid = c.get("s2_id") or f"DOI:{c['doi']}"
    seen, parts = set(), []
    for q in SNIPPET_QUERIES:
        try:
            hits = s2.snippets(q.format(family=name), limit=6, paper_ids=[pid])
        except Exception:  # noqa: BLE001
            break
        for h in hits:
            t = (h.get("text") or "").strip()
            if t and t not in seen:
                seen.add(t)
                parts.append(f"[snippet: section={h.get('section')}]\n{t}")
    if not parts:
        return None, None, None
    text = (
        f"[Semantic Scholar full-text snippets for {c.get('title')} — excerpts, not the whole paper]\n\n"
        + "\n\n".join(parts)
    )
    sha = hashlib.sha256(text.encode("utf-8")).hexdigest()
    os.makedirs(TEXTS, exist_ok=True)
    with open(os.path.join(TEXTS, sha + ".txt"), "w", encoding="utf-8") as f:
        f.write(text)
    return text, sha, "s2-snippets"


def cached_text(cand: dict) -> str | None:
    r = cand.get("retrieved") or {}
    if r.get("sha256"):
        p = os.path.join(TEXTS, r["sha256"] + ".txt")
        if os.path.exists(p):
            with open(p, "rb") as f:
                raw = f.read()
            if hashlib.sha256(raw).hexdigest() == r["sha256"]:
                return raw.decode("utf-8")
    return None


def text_for(cand: dict) -> str | None:
    return cached_text(cand) or cand.get("abstract")
