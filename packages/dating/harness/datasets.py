"""Linked datasets (plan §B4b): the data and supplementary files that modern
papers deposit on Zenodo, GitHub, OSF, Dryad, or link in the text. They are
usually open even when the paper is paywalled, and they are where the
calibrations, trees, and per-language tables actually live.

Discovery
  - links in the retrieved text / abstract (zenodo, github, osf, dryad, figshare, doi.org/10.5281)
  - Zenodo search by the paper's DOI (records whose related_identifiers name it)
Retrieval (cached under cache/datasets/<source_id>/)
  - Zenodo: record metadata + files; GitHub: repository tree + files; Dryad/OSF: file lists
  - allowed extensions only, per-file and per-dataset size caps, zips expanded in memory
Processing
  - tree files (nexus/newick with branch lengths in years): parsed mechanically, nodes
    mapped to Glottolog (taxa -> glottocode via a companion languages/taxa CSV with a
    Glottocode column, else by name within the task subtree) -> phylo_node_age claims
  - text / tabular / PDF files: turned into text and attached to the paper's extraction
    as additional documents, so the model reads the calibration tables and README files
"""

from __future__ import annotations

import csv
import hashlib
import io
import json
import os
import re
import time
import urllib.parse
import urllib.request
import zipfile

from . import glotto
from pathlib import Path
from workflow_tools.paths import REPO, historical
from source_tools.acquire import fetch_bytes
from .config import CACHE_DIR, USER_AGENT, load_env, now
from .schema import Claim, MethodNotes, Source, TimeValue
from .store import upsert_claims, upsert_source
from .time_utils import from_bp

DATA_CACHE = os.path.join(CACHE_DIR, "datasets")
TEXT_EXT = {
    ".csv",
    ".tsv",
    ".txt",
    ".md",
    ".json",
    ".bib",
    ".yaml",
    ".yml",
    ".tab",
    ".rst",
}  # .log/.xml need bounded configuration/trace parsers; they contain methodological evidence
TREE_EXT = {".nex", ".nexus", ".trees", ".tre", ".tree", ".newick", ".nwk", ".mcc", ".con.tre"}
PDF_EXT = {".pdf"}
ALLOWED = TEXT_EXT | TREE_EXT | PDF_EXT
MAX_FILE = 25_000_000  # bytes per file
MAX_ZIP = 80_000_000  # bytes per archive downloaded
MAX_FILES = 200  # files kept per dataset
MAX_TEXT_PER_FILE = 200_000  # chars attached per file
MAX_TEXT_PER_SOURCE = 400_000

LINK_RES = [
    ("zenodo", re.compile(r"zenodo\.org/(?:records?|record)/(\d+)", re.I)),
    ("zenodo", re.compile(r"10\.5281/zenodo\.(\d+)", re.I)),
    (
        "github",
        re.compile(r"github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+?)(?:[/\s).,;\"']|$)", re.I),
    ),
    ("osf", re.compile(r"osf\.io/([a-z0-9]{5})\b", re.I)),
    ("dryad", re.compile(r"10\.5061/dryad\.([A-Za-z0-9.]+)", re.I)),
    ("figshare", re.compile(r"figshare\.com/(?:articles|s)/[^\s\"')]+", re.I)),
]
SUPPLEMENT_WORDS = re.compile(
    r"supplement|dataset|data and|results|material|replication|code|analys|calibrat|cognate|lexical|phylogen",
    re.I,
)


def _get(url: str, binary: bool = False, headers: dict | None = None, timeout: int = 300):
    load_env()
    h = {"User-Agent": USER_AGENT, **(headers or {})}
    if "github.com" in url and os.environ.get("GITHUB_TOKEN"):
        h["Authorization"] = f"Bearer {os.environ['GITHUB_TOKEN']}"
    data = fetch_bytes(url, "research-http", headers=h)
    return data if binary else json.loads(data.decode("utf-8"))


def find_links(text: str) -> list[dict]:
    out, seen = [], set()
    for kind, rx in LINK_RES:
        for m in rx.finditer(text or ""):
            if kind == "github":
                owner, repo = m.group(1), m.group(2).removesuffix(".git")
                if owner.lower() in (
                    "features",
                    "topics",
                    "site",
                    "about",
                    "settings",
                    "orgs",
                    "marketplace",
                    "sponsors",
                ):
                    continue
                key = ("github", f"{owner}/{repo}")
                url = f"https://github.com/{owner}/{repo}"
            elif kind == "figshare":
                key = ("figshare", m.group(0))
                url = "https://" + m.group(0)
            else:
                key = (kind, m.group(1))
                url = {
                    "zenodo": f"https://zenodo.org/records/{m.group(1)}",
                    "osf": f"https://osf.io/{m.group(1)}",
                    "dryad": f"https://doi.org/10.5061/dryad.{m.group(1)}",
                }[kind]
            if key not in seen:
                seen.add(key)
                out.append({"kind": kind, "id": key[1], "url": url, "found": "text"})
    return out


def zenodo_search_by_doi(doi: str) -> list[dict]:
    """Zenodo records that name the paper's DOI. Relation types are unreliable
    (real supplements are often tagged isCitedBy), so keep records that carry
    non-PDF data files or a supplement-like title, and let the model judge the rest."""
    q = urllib.parse.quote(f'related.identifier:"{doi}"')
    try:
        d = _get(f"https://zenodo.org/api/records?q={q}&size=10")
    except Exception:  # noqa: BLE001
        return []
    out = []
    for h in d.get("hits", {}).get("hits", []):
        m = h.get("metadata", {})
        files = h.get("files", []) or []
        title = m.get("title", "")
        has_data = any(os.path.splitext(f["key"])[1].lower() not in PDF_EXT for f in files)
        rel = next(
            (
                r.get("relation")
                for r in m.get("related_identifiers", [])
                if doi.lower() in (r.get("identifier") or "").lower()
            ),
            None,
        )
        if rel in ("cites", "references", "isContinuedBy", "continues"):
            continue  # another work that cites the paper, not its data
        if (
            has_data
            or SUPPLEMENT_WORDS.search(title)
            or SUPPLEMENT_WORDS.search(m.get("description", "")[:400])
        ):
            out.append(
                {
                    "kind": "zenodo",
                    "id": str(h["id"]),
                    "url": f"https://zenodo.org/records/{h['id']}",
                    "found": f"zenodo-search:{rel}",
                    "title": title,
                }
            )
    return out


# --------------------------------------------------------------------------- #
# retrieval
# --------------------------------------------------------------------------- #


def _keep(name: str) -> bool:
    n = name.lower()
    return any(n.endswith(e) for e in ALLOWED) and not n.startswith("__macosx")


def _store(dirpath: str, name: str, data: bytes) -> dict:
    safe = re.sub(r"[^A-Za-z0-9._/-]", "_", name).strip("/")
    if any(part == ".." for part in Path(safe).parts):
        raise ValueError("unsafe dataset member " + name)
    path = os.path.join(dirpath, safe)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)
    return {
        "name": name,
        "path": str(Path(path).resolve().relative_to(REPO)),
        "bytes": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
    }


def _expand_zip(dirpath: str, name: str, data: bytes) -> list[dict]:
    out = []
    try:
        zf = zipfile.ZipFile(io.BytesIO(data))
    except zipfile.BadZipFile:
        return out
    for info in zf.infolist():
        if len(out) >= MAX_FILES:
            break
        if info.is_dir() or not _keep(info.filename) or info.file_size > MAX_FILE:
            continue
        out.append(_store(dirpath, f"{name}/{info.filename}", zf.read(info)))
    return out


def fetch_zenodo(rec_id: str, dirpath: str) -> dict:
    d = _get(f"https://zenodo.org/api/records/{rec_id}")
    m = d.get("metadata", {})
    files_meta = d.get("files", []) or []
    kept, skipped = [], []
    # prefer the smallest archive when several are offered (Full / Light variants)
    zips = sorted(
        (f for f in files_meta if f["key"].lower().endswith(".zip")), key=lambda f: f["size"]
    )
    for f in files_meta:
        name, size = f["key"], f["size"]
        url = f["links"]["self"]
        if name.lower().endswith(".zip"):
            if f is not (zips[0] if zips else None) or size > MAX_ZIP:
                skipped.append(
                    {"name": name, "bytes": size, "why": "archive too large or a larger variant"}
                )
                continue
            kept += _expand_zip(dirpath, name, _get(url, binary=True, timeout=900))
        elif _keep(name) and size <= MAX_FILE:
            kept.append(_store(dirpath, name, _get(url, binary=True, timeout=600)))
        else:
            skipped.append({"name": name, "bytes": size, "why": "type or size"})
        time.sleep(0.3)
    return {
        "title": m.get("title"),
        "doi": d.get("doi"),
        "license": (m.get("license") or {}).get("id"),
        "files": kept,
        "skipped": skipped,
        "citation": f"{m.get('title')} ({m.get('publication_date', '')}). Zenodo. https://doi.org/{d.get('doi')}",
    }


LANGUAGE_WORDS = re.compile(
    r"\blanguage|linguist|cognate|lexic|phylogen|glotto|etymolog|dialect|proto-|wordlist|swadesh|cldf|lexibank",
    re.I,
)
DATA_EXT = {".csv", ".tsv", ".tab", ".nex", ".nexus", ".trees", ".tre", ".tree", ".newick", ".nwk"}


def fetch_github(owner_repo: str, dirpath: str) -> dict:
    """Fetch a repository's data files — but only when it looks like a language
    dataset (description/README mention language data AND the tree holds data
    files). Papers also cite software repositories (aligners, admixture tools);
    those are recorded and skipped."""
    meta = _get(f"https://api.github.com/repos/{owner_repo}")
    branch = meta.get("default_branch", "main")
    commit = _get(f"https://api.github.com/repos/{owner_repo}/commits/{branch}")["sha"]
    tree = _get(f"https://api.github.com/repos/{owner_repo}/git/trees/{commit}?recursive=1")
    if tree.get("truncated"):
        raise ValueError("GitHub tree response truncated; refuse incomplete dataset")
    blobs = [it for it in tree.get("tree", []) if it.get("type") == "blob"]
    has_data = any(os.path.splitext(it["path"])[1].lower() in DATA_EXT for it in blobs)
    about = f"{meta.get('description') or ''} {meta.get('full_name')} {' '.join((meta.get('topics') or []))}"
    readme = ""
    try:
        rd = _get(f"https://api.github.com/repos/{owner_repo}/readme?ref={commit}")
        import base64

        readme = base64.b64decode(rd.get("content", "")).decode("utf-8", errors="replace")[:5000]
    except Exception:  # noqa: BLE001
        pass
    blurb = re.sub(
        r"programming language|language model|natural language processing",
        " ",
        about + " " + readme,
        flags=re.I,
    )
    # "phylogen" alone is genetics; require a word that is specifically about languages
    relevant = len(LANGUAGE_WORDS.findall(blurb)) >= 2 and bool(
        re.search(
            r"\blanguages?\b|linguist|cognate|lexic|glotto|etymolog|dialect|wordlist|swadesh|cldf|lexibank",
            blurb,
            re.I,
        )
    )
    base = {
        "title": meta.get("full_name"),
        "description": meta.get("description"),
        "license": (meta.get("license") or {}).get("spdx_id"),
        "commit": commit,
        "tree_sha": tree.get("sha"),
        "citation": f"{meta.get('full_name')} (GitHub, {branch}@{commit[:10]}). https://github.com/{owner_repo}",
    }
    if not (relevant and has_data):
        return {
            **base,
            "files": [],
            "skipped": [
                {
                    "name": "*",
                    "why": f"not a language dataset (relevant={relevant}, data files={has_data})",
                }
            ],
        }
    kept, skipped = [], []
    for item in blobs:
        if len(kept) >= MAX_FILES:
            skipped.append({"name": item["path"], "why": "file cap"})
            continue
        if not _keep(item["path"]) or (item.get("size") or 0) > MAX_FILE:
            continue
        raw = _get(
            f"https://raw.githubusercontent.com/{owner_repo}/{commit}/{urllib.parse.quote(item['path'])}",
            binary=True,
        )
        kept.append(_store(dirpath, item["path"], raw))
        time.sleep(0.1)
    return {**base, "files": kept, "skipped": skipped}


def fetch_dryad(doi_suffix: str, dirpath: str) -> dict:
    doi = urllib.parse.quote(f"doi:10.5061/dryad.{doi_suffix}", safe="")
    meta = _get(f"https://datadryad.org/api/v2/datasets/{doi}")
    data = _get(f"https://datadryad.org/api/v2/datasets/{doi}/download", binary=True, timeout=900)
    kept = _expand_zip(dirpath, "dryad.zip", data) if len(data) <= MAX_ZIP else []
    return {
        "title": meta.get("title"),
        "files": kept,
        "skipped": [] if kept else [{"name": "dryad.zip", "bytes": len(data), "why": "too large"}],
        "citation": f"{meta.get('title')}. Dryad. https://doi.org/10.5061/dryad.{doi_suffix}",
    }


def fetch_osf(node: str, dirpath: str) -> dict:
    d = _get(f"https://api.osf.io/v2/nodes/{node}/files/osfstorage/?page[size]=100")
    kept, skipped = [], []
    for item in d.get("data", []):
        a = item.get("attributes", {})
        name = a.get("name", "")
        if a.get("kind") != "file" or not _keep(name) or (a.get("size") or 0) > MAX_FILE:
            continue
        link = item.get("links", {}).get("download")
        if link:
            kept.append(_store(dirpath, name, _get(link, binary=True)))
    return {
        "title": f"osf.io/{node}",
        "files": kept,
        "skipped": skipped,
        "citation": f"OSF project {node}. https://osf.io/{node}",
    }


def landing_page_links(doi: str) -> list[dict]:
    """Links on the publisher's page for the DOI (data availability statements,
    supplementary material links) — visible even when the article is paywalled."""
    try:
        html = _get(
            f"https://doi.org/{urllib.parse.quote(doi)}",
            binary=True,
            headers={"Accept": "text/html"},
            timeout=60,
        ).decode("utf-8", errors="replace")
    except Exception:  # noqa: BLE001
        return []
    html = re.sub(r"<(script|style)[^>]*>.*?</\1>", " ", html, flags=re.S | re.I)
    links = find_links(html)
    for l in links:
        l["found"] = "landing-page"
    return links


def crossref_relations(doi: str) -> list[dict]:
    """Crossref `relation` metadata: is-supplemented-by / has-part etc. pointing at data DOIs."""
    try:
        d = _get(f"https://api.crossref.org/works/{urllib.parse.quote(doi)}")
    except Exception:  # noqa: BLE001
        return []
    out = []
    for rel, items in (d.get("message", {}).get("relation") or {}).items():
        if rel in (
            "is-supplemented-by",
            "has-part",
            "is-referenced-by",
            "references",
            "has-version",
            "is-derived-from",
        ):
            for it in items:
                if it.get("id-type") == "doi":
                    for l in find_links("https://doi.org/" + it["id"]):
                        l["found"] = f"crossref:{rel}"
                        out.append(l)
    return out


def retrieve_datasets(cand: dict, text: str | None) -> list[dict]:
    """Discover and fetch the datasets linked to one candidate paper, in this order:
    links in the paper's own text/abstract; links on the publisher landing page;
    Crossref relation metadata; and, as a supplementary route, a Zenodo search by
    the paper's DOI. Idempotent: results are recorded on the candidate under `datasets`."""
    links = find_links((text or "") + "\n" + (cand.get("abstract") or ""))
    for l in links:
        l["found"] = "paper-text"
    seen = {(l["kind"], l["id"]) for l in links}

    def add(more: list[dict]) -> None:
        for l in more:
            if (l["kind"], l["id"]) not in seen:
                seen.add((l["kind"], l["id"]))
                links.append(l)

    if cand.get("doi"):
        add(landing_page_links(cand["doi"]))
        add(crossref_relations(cand["doi"]))
        add(zenodo_search_by_doi(cand["doi"]))  # supplementary: records that name the DOI
    done = {(d["kind"], d["id"]) for d in cand.get("datasets", [])}
    out = list(cand.get("datasets", []))
    for l in links:
        if (l["kind"], l["id"]) in done or l["kind"] == "figshare":
            continue
        dirpath = os.path.join(
            DATA_CACHE,
            cand.get("source_id", "unknown"),
            f"{l['kind']}_{re.sub(r'[^A-Za-z0-9_.-]', '_', l['id'])}",
        )
        rec = {
            "kind": l["kind"],
            "id": l["id"],
            "url": l["url"],
            "found": l["found"],
            "retrieved": now(),
        }
        try:
            if l["kind"] == "zenodo":
                rec.update(fetch_zenodo(l["id"], dirpath))
            elif l["kind"] == "github":
                rec.update(fetch_github(l["id"], dirpath))
            elif l["kind"] == "dryad":
                rec.update(fetch_dryad(l["id"], dirpath))
            elif l["kind"] == "osf":
                rec.update(fetch_osf(l["id"], dirpath))
        except Exception as e:  # noqa: BLE001
            rec["error"] = str(e)[:300]
        out.append(rec)
    cand["datasets"] = out
    return out


# --------------------------------------------------------------------------- #
# processing: text for extraction, trees for mechanical node ages
# --------------------------------------------------------------------------- #


def _read_text(path: str) -> str | None:
    ext = os.path.splitext(path)[1].lower()
    if ext in PDF_EXT:
        import pymupdf

        try:
            doc = pymupdf.open(path)
            return "\n".join(p.get_text() for p in doc)
        except Exception:  # noqa: BLE001
            return None
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            return f.read()
    except OSError:
        return None


def dataset_bundle(cand: dict) -> tuple[str, list[dict]]:
    """Return bounded evidence text and an inventory of every known attachment.

    Character counts exclude provenance headers. This inventories the supplied
    candidate, not all files that may exist at the publisher or repository.
    """
    parts, inventory, total = [], [], 0
    for ds in cand.get("datasets", []):
        for skipped in ds.get("skipped", []):
            inventory.append(
                {
                    "dataset_url": ds.get("url"),
                    **skipped,
                    "status": "not_retrieved",
                    "included_chars": 0,
                }
            )
        for f in ds.get("files", []):
            item = {
                "dataset_url": ds.get("url"),
                "name": f["name"],
                "path": f["path"],
                "declared_raw_sha256": f.get("sha256"),
                "included_chars": 0,
            }
            inventory.append(item)
            ext = os.path.splitext(f["name"])[1].lower()
            if ext in TREE_EXT:
                item["status"] = "tree_not_in_text_prompt"
                continue
            t = _read_text(str(historical(f["path"])))
            if not t or not t.strip():
                item["status"] = "unreadable_or_empty"
                continue
            item.update(
                {"available_chars": len(t), "text_sha256": hashlib.sha256(t.encode()).hexdigest()}
            )
            included = min(len(t), MAX_TEXT_PER_FILE, MAX_TEXT_PER_SOURCE - total)
            item.update(
                {
                    "included_chars": included,
                    "status": "included"
                    if included == len(t)
                    else "truncated"
                    if included
                    else "budget_omitted",
                    "included_text_sha256": hashlib.sha256(t[:included].encode()).hexdigest(),
                }
            )
            if not included:
                continue
            t = t[:included]
            parts.append(f"\n\n===== DATASET FILE: {f['name']} (from {ds['url']}) =====\n{t}")
            total += len(t)
    return "".join(parts), inventory


def dataset_text(cand: dict) -> str:
    """Compatibility accessor; extraction also retains dataset_bundle's inventory."""
    return dataset_bundle(cand)[0]


def _taxon_map(
    files: list[dict], tips: list[str], g: glotto.Glottolog, subtree: set[str]
) -> dict[str, str]:
    """Tip label -> glottocode: companion CSV with a Glottocode column, else names."""
    mapping: dict[str, str] = {}
    for f in files:
        if not f["name"].lower().endswith((".csv", ".tsv")):
            continue
        try:
            with open(historical(f["path"]), encoding="utf-8", errors="replace", newline="") as fh:
                sample = fh.read(4096)
                fh.seek(0)
                dialect = csv.Sniffer().sniff(sample) if sample else csv.excel
                rows = list(csv.DictReader(fh, dialect=dialect))
        except Exception:  # noqa: BLE001
            continue
        if not rows:
            continue
        gcol = next(
            (
                c
                for c in rows[0]
                if c
                and c.strip().lower()
                in ("glottocode", "glottolog", "glottolog_code", "glottolog_id")
            ),
            None,
        )
        if not gcol:
            continue
        for r in rows:
            gid = (r.get(gcol) or "").strip()
            if gid not in g.rows:
                continue
            for c, v in r.items():
                if v and v.strip() in tips:
                    mapping[v.strip()] = gid
    missing = [t for t in tips if t not in mapping]
    if missing:
        # names: Glottolog names in the subtree + Wiktionary names mapped into it,
        # compared with all non-alphanumerics removed, in both word orders
        def keys(name: str) -> list[str]:
            words = re.findall(r"[a-z0-9]+", name.lower())
            ks = ["".join(words)]
            if len(words) == 2:
                ks.append(words[1] + words[0])
            return ks

        names: dict[str, str] = {}
        for gid in subtree:
            for k in keys(g.rows[gid]["name"]):
                names.setdefault(k, gid)
        for code, rec in g.wikt.items():
            gid = rec.get("glottocode")
            if gid in subtree:
                for k in keys(rec["name"]):
                    names.setdefault(k, gid)
        for t in missing:
            k = "".join(re.findall(r"[a-z0-9]+", t.lower()))
            if k in names:
                mapping[t] = names[k]
    return mapping


def ingest_trees(family: str, cand: dict, run: str) -> dict:
    """Only explicitly described, hashed analyses may produce chronological claims."""
    from .tree_ingest import ingest_manifests

    return ingest_manifests(source_id=cand.get("source_id"), family=family)
