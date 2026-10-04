"""Sources the maintainer supplies (library copies of handbooks), with acquire's provenance.

A holding is a file the maintainer obtained, usually attached to a Zotero item. Its
bytes are kept as an immutable artifact (data/artifacts/dating-local-sources/<sha256>,
the layout `acquire` uses for downloads) with a receipt in data/registry/acquisitions,
and its parsed text is stored by hash in the retrieved-text store. A researcher finds
holdings with the `library` tool action and loads one with `open`; quotes then carry
exact offsets into that text, as for any acquired source. Zotero paths never become
evidence: claims cite the retained text.

A source request records a work a researcher needed but could not read: which part,
why, for which codes, and the routes tried. `requests push` files open requests in
Zotero (collection wety/dating/<topic>, tag wety:needs-fulltext); once the maintainer
attaches the file, `holdings sync` ingests it and marks the request supplied.

Tracked records (metadata only, never publisher text):
  data/registry/holdings.json          holdings by id
  data/registry/source-requests.json   the acquisition queue
Source bytes and text stay under data/artifacts and data/workspaces (ignored by git).
"""

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import tempfile
import uuid

from workflow_tools.artifacts import atomic_json, manifest, verify
from workflow_tools.paths import DATA, REPO

from .config import CACHE_DIR
from .io_utils import locked

COLLECTION = "dating-local-sources"
HOLDINGS = DATA / "registry/holdings.json"
REQUESTS = DATA / "registry/source-requests.json"
RECEIPTS = DATA / "registry/acquisitions"
NEEDS_FULLTEXT = "wety:needs-fulltext"
NEEDS_PART = "wety:needs-supplement"
SUPPLIED = "wety:ingested"
OCR_MIN_CHARACTERS = 25
PARSER = "wety-holdings-v2"  # bump when extract_pdf output or metadata changes
REQUEST_STATUSES = ("open", "filed", "supplied", "unobtainable", "withdrawn")
NEEDED = ("work", "chapter", "pages", "supplement", "dataset")


def now():
    return datetime.now(timezone.utc).isoformat()


def file_sha(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as f:
        for chunk in iter(lambda: f.read(8 * 1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def texts_dir():
    return Path(CACHE_DIR) / "texts"


def load(path, name):
    path = Path(path)
    if not path.exists():
        return {"schema_version": 1, name: {}}
    return json.loads(path.read_text())


# Text


def extract_pdf(path, ocr="auto", languages="eng", max_ocr_pages=None):
    """Text of a PDF with `[PDF page N]` markers (acquire's format) and its metadata.

    ocr: "auto" OCRs only pages without a text layer that hold an image (scans);
    "force" OCRs every page (a poor existing text layer); "off" never OCRs.
    max_ocr_pages: in auto mode, skip OCR when more pages need it (a research tool call
    cannot wait for a whole scanned book; the maintainer ingests those as holdings).
    """
    import pymupdf

    if ocr not in ("auto", "force", "off"):
        raise ValueError("ocr must be auto, force or off")
    parts, ocr_pages, empty, image_pages = [], [], [], []
    with pymupdf.open(path) as doc:
        labels = [page.get_label() for page in doc]
        texts = [page.get_text() for page in doc]
        scanned = [
            i
            for i, page in enumerate(doc)
            if len(texts[i].strip()) < OCR_MIN_CHARACTERS and page.get_images()
        ]
        skipped = ocr == "auto" and max_ocr_pages is not None and len(scanned) > max_ocr_pages
        for i, page in enumerate(doc):
            text = texts[i]
            if ocr == "force" or (ocr == "auto" and not skipped and i in scanned):
                textpage = page.get_textpage_ocr(language=languages, dpi=300, full=True)
                text = page.get_text(textpage=textpage)
                if len(text.strip()) >= OCR_MIN_CHARACTERS:
                    ocr_pages.append(i + 1)
            elif len(text.strip()) >= OCR_MIN_CHARACTERS and full_page_image(page):
                # A scan with an embedded text layer: someone else's OCR.
                image_pages.append(i + 1)
            if len(text.strip()) < OCR_MIN_CHARACTERS:
                empty.append(i + 1)
            parts.append(f"\n[PDF page {i + 1}]\n" + text)
        meta = {
            "parser": "pymupdf " + pymupdf.VersionBind,
            "pages": len(doc),
            "kind": "pdf_ocr_text" if ocr_pages else "pdf_text",
            "coverage": "PDF text extraction; figures/images uninspected until page tool, scanned text may be absent",
        }
    if ocr_pages:
        meta.update(
            ocr={
                "engine": "tesseract via pymupdf",
                "languages": languages,
                "dpi": 300,
                "pages": len(ocr_pages),
                "page_list": compact_pages(ocr_pages),
            },
            coverage="PDF text, partly or wholly OCR: check every quote against the page image (page tool); OCR may misread diacritics, numbers and non-Latin scripts",
        )
    if image_pages:
        meta["scan_text_layer"] = {
            "pages": len(image_pages),
            "page_list": compact_pages(image_pages),
            "note": "page images with an embedded text layer (OCR of unknown quality); check quotes against the page image",
        }
        if not ocr_pages:
            meta["coverage"] = (
                "PDF text layer of scanned pages (OCR of unknown origin): check every quote against the page image (page tool)"
            )
    if empty:
        meta["pages_without_text"] = compact_pages(empty)
    if skipped:
        meta["ocr_skipped"] = (
            f"{len(scanned)} scanned pages without text; too many to OCR here. "
            "Request the source so the maintainer can ingest it with OCR."
        )
    if any(labels):
        meta["page_labels"] = labels
    return "\n".join(parts), meta


def full_page_image(page):
    area = abs(page.rect)
    return area > 0 and any(
        abs(pymupdf_rect(info["bbox"]) & page.rect) >= 0.8 * area for info in page.get_image_info()
    )


def pymupdf_rect(bbox):
    import pymupdf

    return pymupdf.Rect(bbox)


def expand_pages(compact):
    """ "1-3,7" -> [1, 2, 3, 7]."""
    pages = []
    for run in filter(None, (compact or "").split(",")):
        first, _, last = run.partition("-")
        pages += range(int(first), int(last or first) + 1)
    return pages


def compact_pages(pages):
    """[1,2,3,7] -> "1-3,7"."""
    runs, start = [], None
    for i, p in enumerate(pages):
        if start is None:
            start = p
        if i + 1 == len(pages) or pages[i + 1] != p + 1:
            runs.append(str(start) if start == p else f"{start}-{p}")
            start = None
    return ",".join(runs)


def page_at(text, offset):
    """One-based PDF page holding a text offset, from the `[PDF page N]` markers."""
    found = None
    for m in re.finditer(r"\n\[PDF page (\d+)\]\n", text[: offset + 1]):
        found = int(m.group(1))
    return found


def identity_hint(text, title):
    """Whether the title's words occur in the first pages: a check, not proof."""
    words = {w for w in re.findall(r"\w{4,}", (title or "").lower())}
    if not words:
        return {"title_words_found": None}
    head = text[: text.find("\n[PDF page 16]\n")] if "\n[PDF page 16]\n" in text else text[:60000]
    found = {w for w in words if w in head.lower()}
    hint = {"title_words_found": f"{len(found)}/{len(words)}", "checked_pages": "1-15"}
    if len(found) < len(words):
        hint["missing"] = sorted(words - found)
    return hint


# Retention


def retain(path, provenance):
    """Copy bytes into the artifact store and write a receipt, as acquire does."""
    from workflow_tools.artifacts import sha as artifact_sha

    stage = DATA / "cache/downloads"
    stage.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=stage) as tmp:
        tmp = Path(tmp)
        (tmp / "files").mkdir()
        shutil.copyfile(path, tmp / "files/source")
        checksum = artifact_sha(tmp / "files/source")
        m = manifest(tmp, COLLECTION, "source", provenance)
        dest = DATA / "artifacts" / COLLECTION / checksum
        if dest.exists():
            if verify(dest):
                raise ValueError("existing artifact corrupted")
        else:
            atomic_json(tmp / "manifest.json", m)
            dest.parent.mkdir(parents=True, exist_ok=True)
            os.rename(tmp, dest)
    receipt = RECEIPTS / (uuid.uuid4().hex + ".json")
    atomic_json(
        receipt,
        {
            "schema_version": 1,
            "artifact": str(dest.relative_to(REPO)),
            "artifact_id": m["artifact_id"],
            "sha256": checksum,
            **provenance,
            "acquirer_sha256": file_sha(__file__),
        },
    )
    return dest, receipt


def ingest(path, bibliography, *, zotero=None, extent=None, ocr="auto", languages="eng"):
    """Retain a supplied PDF and its parsed text; record (or update) the holding."""
    path = Path(path)
    with path.open("rb") as f:
        if f.read(5) != b"%PDF-":
            raise ValueError("PDF required (convert EPUB or DjVu to PDF first): " + path.name)
    raw_sha = file_sha(path)
    hid = raw_sha[:16]
    provenance = {
        "origin": "maintainer_supplied",
        "original_filename": path.name,
        "bytes": path.stat().st_size,
        "ingested_at": now(),
        "zotero": zotero,
        "completeness": "file as supplied by the maintainer; identity and extent unverified until reviewed",
    }
    with locked(HOLDINGS):
        holdings = load(HOLDINGS, "holdings")
        old = holdings["holdings"].get(hid)
        settings = {"ocr": ocr, "languages": languages, "parser": PARSER}
        if (
            old
            and old["text"]["settings"] == settings
            and (texts_dir() / (old["text"]["sha256"] + ".txt")).exists()
        ):
            changed = refresh_bibliography(old, bibliography)
            if zotero and old.get("zotero") != zotero:
                old["zotero"] = zotero
                changed = True
            if changed:
                atomic_json(HOLDINGS, holdings)
            return old
        artifact = DATA / "artifacts" / COLLECTION / raw_sha
        if old and artifact.exists() and not verify(artifact):
            # Already retained: parse again (new settings or parser), no new receipt.
            dest, receipt = artifact, REPO / old["file"]["receipt"]
        else:
            dest, receipt = retain(path, provenance)
        text, meta = extract_pdf(dest / "files/source", ocr, languages)
        data = text.encode()
        tsha = hashlib.sha256(data).hexdigest()
        texts_dir().mkdir(parents=True, exist_ok=True)
        (texts_dir() / (tsha + ".txt")).write_bytes(data)
        holding = {
            "id": hid,
            **bibliography,
            "origin": "maintainer_supplied",
            "zotero": zotero,
            "extent": extent or "the whole file as supplied",
            "file": {
                "artifact": str(dest.relative_to(REPO)),
                "raw_path": str((dest / "files/source").relative_to(REPO)),
                "sha256": raw_sha,
                "bytes": provenance["bytes"],
                "original_filename": path.name,
                "receipt": str(receipt.relative_to(REPO)),
            },
            "text": {
                "sha256": tsha,
                "characters": len(text),
                "settings": settings,
                **{k: v for k, v in meta.items() if k != "page_labels"},
                "page_labels": bool(meta.get("page_labels")),
            },
            "identity_hint": identity_hint(text, bibliography.get("title")),
            "ingested_at": old["ingested_at"] if old else provenance["ingested_at"],
            "parsed_at": now(),
            "ingester_sha256": file_sha(__file__),
        }
        if old:
            holding["previous_texts"] = old.get("previous_texts", []) + [old["text"]]
        holdings["holdings"][hid] = holding
        atomic_json(HOLDINGS, holdings)
    mark_supplied(holding)
    return holding


BIB_FIELDS = (
    "item_type",
    "title",
    "authors",
    "editors",
    "year",
    "edition",
    "publisher",
    "place",
    "series",
    "container",
    "doi",
    "isbn",
    "url",
    "citation",
)


def refresh_bibliography(holding, bibliography):
    """Take corrected metadata (e.g. edited in Zotero) for unchanged bytes; the replaced
    values stay in metadata_history. Registered sources keep their own reviewed metadata."""
    old = {k: holding.get(k) for k in BIB_FIELDS if holding.get(k) is not None}
    new = {k: v for k, v in bibliography.items() if k in BIB_FIELDS}
    if old == new:
        return False
    for k in BIB_FIELDS:
        holding.pop(k, None)
    holding.update(new)
    holding.setdefault("metadata_history", []).append({"replaced": old, "at": now()})
    stored = texts_dir() / (holding["text"]["sha256"] + ".txt")
    holding["identity_hint"] = identity_hint(stored.read_bytes().decode(), new.get("title"))
    return True


# Zotero


def bibliography_from_zotero(item):
    d = item["data"]
    people = {"author": [], "editor": []}
    for c in d.get("creators", []):
        name = c.get("name") or ", ".join(x for x in (c.get("lastName"), c.get("firstName")) if x)
        people.setdefault(c.get("creatorType", "author"), []).append(name)
    year = (
        item.get("meta", {}).get("parsedDate", "")[:4]
        or (re.findall(r"\d{4}", d.get("date", "")) or [""])[0]
    )
    bib = {
        "item_type": d.get("itemType"),
        "title": d.get("title"),
        "authors": people["author"],
        "editors": people["editor"],
        "year": int(year) if year.isdigit() else None,
        "edition": d.get("edition") or None,
        "publisher": d.get("publisher") or None,
        "place": d.get("place") or None,
        "series": d.get("series") or None,
        "container": d.get("bookTitle") or d.get("publicationTitle") or None,
        "doi": d.get("DOI") or None,
        "isbn": d.get("ISBN") or None,
        "url": d.get("url") or None,
    }
    bib["citation"] = citation(bib)
    return {k: v for k, v in bib.items() if v not in (None, [], "")}


def citation(bib):
    names = bib.get("authors") or [n + " (ed.)" for n in bib.get("editors", [])]
    parts = [
        "; ".join(names),
        f"({bib['year']})" if bib.get("year") else "",
        (bib.get("title") or "") + ".",
        f"In {bib['container']}."
        if bib.get("container") and bib["container"] != bib.get("title")
        else "",
        (bib.get("edition") + ".") if bib.get("edition") else "",
        ", ".join(x for x in (bib.get("place"), bib.get("publisher")) if x) + ".",
        f"https://doi.org/{bib['doi']}" if bib.get("doi") else "",
    ]
    return " ".join(p for p in parts if p and p != ".").strip()


def ingest_zotero_item(client, key, *, extent=None, ocr="auto", languages="eng"):
    """Ingest every PDF attachment of a Zotero item (or of an attachment key's parent)."""
    item = client.item(key)
    if item["data"]["itemType"] == "attachment":
        attachments = [item]
        item = client.item(item["data"]["parentItem"])
    else:
        attachments = [
            c
            for c in client.children(key)
            if c["data"]["itemType"] == "attachment"
            and c["data"].get("contentType") == "application/pdf"
        ]
    bib = bibliography_from_zotero(item)
    results = []
    for att in attachments:
        path = client.file_path(att["key"])
        zot = {
            "item_key": item["key"],
            "attachment_key": att["key"],
            "attachment_title": att["data"].get("title"),
            "link_mode": att["data"].get("linkMode"),
            "item_version": item["version"],
            "attachment_md5": att["data"].get("md5"),
        }
        if path is None:
            results.append({"zotero": zot, "status": "file_not_on_this_computer"})
            continue
        holding = ingest(path, bib, zotero=zot, extent=extent, ocr=ocr, languages=languages)
        results.append({"holding": holding["id"], "title": holding["title"], "status": "ingested"})
    if not attachments:
        results.append({"zotero": {"item_key": item["key"]}, "status": "no_pdf_attachment"})
    return results


def sync(client, keys=()):
    """Ingest PDFs of the given items, of every item in the wety collection tree, and of
    filed requests. Unchanged files are skipped (same bytes, same text settings)."""
    wanted = list(keys)
    for collection in sorted(client.collection_tree()):
        wanted += [i["key"] for i in client.collection_items(collection)]
    requests = load(REQUESTS, "requests")["requests"]
    wanted += [
        r["zotero"]["item_key"]
        for r in requests.values()
        if r.get("zotero") and r["status"] == "filed"
    ]
    report = []
    for key in dict.fromkeys(wanted):
        report += ingest_zotero_item(client, key)
    return report


def mark_supplied(holding):
    """Close filed or open requests the holding answers (same Zotero item, DOI or ISBN).
    A request for a supplement or dataset is closed only by hand: an article is not its
    supplement."""
    with locked(REQUESTS):
        queue = load(REQUESTS, "requests")
        changed = []
        for key, r in queue["requests"].items():
            if r["status"] not in ("open", "filed") or r.get("needed") in ("supplement", "dataset"):
                continue
            same = (
                (
                    r.get("zotero")
                    and holding.get("zotero")
                    and r["zotero"]["item_key"] == holding["zotero"]["item_key"]
                )
                or (
                    r.get("doi")
                    and holding.get("doi")
                    and r["doi"].lower() == holding["doi"].lower()
                )
                or (
                    r.get("isbn")
                    and holding.get("isbn")
                    and set(isbns(r["isbn"])) & set(isbns(holding["isbn"]))
                )
            )
            if same:
                set_status(r, "supplied", holding=holding["id"])
                changed.append(key)
        if changed:
            atomic_json(REQUESTS, queue)
    return changed


def isbns(value):
    return [re.sub(r"[^0-9X]", "", x.upper()) for x in re.split(r"[ ,;]+", value or "") if x]


# Research access


def search(query=None, limit=30):
    """Holdings and requests matching a query (title, authors, editors, DOI, ISBN)."""
    holdings = load(HOLDINGS, "holdings")["holdings"].values()
    requests = load(REQUESTS, "requests")["requests"].values()
    terms = [t for t in re.split(r"\s+", (query or "").lower()) if t]

    def hit(record):
        hay = " ".join(
            str(record.get(k) or "")
            for k in ("title", "authors", "editors", "doi", "isbn", "citation", "container")
        ).lower()
        return all(t in hay for t in terms)

    found = [
        {
            "holding_id": h["id"],
            "citation": h.get("citation") or h.get("title"),
            "extent": h["extent"],
            "pages": h["text"].get("pages"),
            "text_kind": h["text"]["kind"],
            "ocr_pages": h["text"].get("ocr", {}).get("pages", 0),
            "identity_hint": h.get("identity_hint"),
        }
        for h in holdings
        if hit(h)
    ]
    asked = [
        {
            "request": r["key"],
            "title": r.get("title"),
            "status": r["status"],
            "needed": r.get("needed"),
        }
        for r in requests
        if hit(r) and r["status"] != "supplied"
    ]
    return {
        "holdings": found[:limit],
        "holdings_omitted": max(0, len(found) - limit),
        "requests": asked[:limit],
        "scope": "Sources the maintainer supplied (open them by holding_id) and sources already requested.",
    }


def open_holding(holding_id, folder):
    """Copy a holding's text into a run's evidence folder; source metadata in acquire's
    shape, so read, find, page, check and build-packet treat it like an acquisition."""
    holding = load(HOLDINGS, "holdings")["holdings"].get(holding_id)
    if not holding:
        raise ValueError("unknown holding_id; call library")
    stored = texts_dir() / (holding["text"]["sha256"] + ".txt")
    if not stored.exists() or file_sha(stored) != holding["text"]["sha256"]:
        raise ValueError("holding text missing or changed; re-ingest it")
    raw = REPO / holding["file"]["raw_path"]
    if not raw.exists() or file_sha(raw) != holding["file"]["sha256"]:
        raise ValueError("holding file missing or changed; re-ingest it")
    folder = Path(folder)
    folder.mkdir(exist_ok=True)
    sid = holding["id"]
    shutil.copyfile(stored, folder / (sid + ".txt"))
    labels = None
    if holding["text"].get("page_labels"):
        import pymupdf

        with pymupdf.open(raw) as doc:
            labels = [page.get_label() for page in doc]
    meta = {
        "url": f"https://doi.org/{holding['doi']}" if holding.get("doi") else None,
        "raw_path": holding["file"]["raw_path"],
        "raw_sha256": holding["file"]["sha256"],
        "kind": holding["text"]["kind"],
        "parser": holding["text"]["parser"],
        "pages": holding["text"]["pages"],
        "coverage": holding["text"]["coverage"],
        "text_file": sid + ".txt",
        "text_sha256": holding["text"]["sha256"],
        "characters": holding["text"]["characters"],
        "source_id": sid,
        "holding": {
            "id": sid,
            "origin": holding["origin"],
            "citation": holding.get("citation"),
            "extent": holding["extent"],
            "zotero": holding.get("zotero"),
            "identity_hint": holding.get("identity_hint"),
        },
    }
    if labels:
        meta["page_labels"] = labels
    for key in ("ocr", "scan_text_layer"):
        if holding["text"].get(key):
            meta[key] = holding["text"][key]
    return meta


# Requests


def request_key(r):
    if r.get("doi"):
        return "doi:" + r["doi"].lower().removeprefix("https://doi.org/")
    if r.get("isbn"):
        return "isbn:" + isbns(r["isbn"])[0]
    title = re.sub(r"\W+", " ", (r.get("title") or "").lower()).strip()
    if not title:
        raise ValueError("a request needs a title, DOI or ISBN")
    # Without an identifier: first author's surname, year and title ("Sanskrit" alone
    # names many works).
    first = (r.get("authors") or [""])[0]
    surname = first.split(",")[0] if "," in first else first.split(" ")[-1]
    parts = [re.sub(r"\W+", "", surname.lower()), str(r.get("year") or ""), title[:100]]
    return "work:" + "|".join(parts)


def validate_request(r):
    if not isinstance(r, dict):
        raise ValueError("request must be an object")
    for field in ("title", "why", "needed"):
        if not isinstance(r.get(field), str) or not r[field].strip():
            raise ValueError("request needs " + field)
    if r["needed"] not in NEEDED:
        raise ValueError("needed must be one of " + ", ".join(NEEDED))
    routes = r.get("routes_tried", [])
    if not isinstance(routes, list) or not all(
        isinstance(x, dict) and x.get("route") and x.get("outcome") for x in routes
    ):
        raise ValueError("routes_tried: list of {route, url?, outcome}")
    if not isinstance(r.get("codes", []), list):
        raise ValueError("codes must be a list")
    return r


def set_status(r, status, **extra):
    if status not in REQUEST_STATUSES:
        raise ValueError("status must be one of " + ", ".join(REQUEST_STATUSES))
    r["status"] = status
    r.update(extra)
    r.setdefault("history", []).append({"status": status, "at": now(), **extra})


def merge_requests(run_dir):
    """Add a run's requests.jsonl to the queue. Repeated asks for one work accumulate;
    a supplied or unobtainable request keeps its status."""
    run_dir = Path(run_dir).resolve()
    path = run_dir / "requests.jsonl"
    if not path.exists():
        return {"merged": 0}
    run = str(run_dir.relative_to(REPO)) if run_dir.is_relative_to(REPO) else str(run_dir)
    merged = []
    with locked(REQUESTS):
        queue = load(REQUESTS, "requests")
        for line in path.read_text().splitlines():
            if not line.strip():
                continue
            r = validate_request(json.loads(line))
            key = request_key(r)
            ask = {
                "run": run,
                "codes": r.get("codes", []),
                "why": r["why"],
                "needed": r["needed"],
                "part": r.get("part"),
                "routes_tried": r.get("routes_tried", []),
                "requested_at": r.get("requested_at"),
            }
            entry = queue["requests"].get(key)
            if not entry:
                entry = {
                    "key": key,
                    **{
                        k: r.get(k)
                        for k in (
                            "title",
                            "authors",
                            "year",
                            "item_type",
                            "doi",
                            "isbn",
                            "container",
                            "publisher",
                            "edition",
                        )
                        if r.get(k)
                    },
                    "needed": r["needed"],
                    "priority": r.get("priority", "normal"),
                    "asks": [],
                }
                set_status(entry, "open")
                queue["requests"][key] = entry
            if ask in entry["asks"]:
                continue
            entry["asks"].append(ask)
            if r.get("priority") == "high":
                entry["priority"] = "high"
            merged.append(key)
        atomic_json(REQUESTS, queue)
    return {"merged": len(merged), "requests": merged}


ZOTERO_TYPES = {"book", "bookSection", "journalArticle", "thesis", "report", "dataset", "webpage"}


ITEM_TYPE_ALIASES = {
    "chapter": "bookSection",
    "article": "journalArticle",
    "paper": "journalArticle",
}


def zotero_item(r):
    """A new Zotero item from a request's bibliographic fields."""
    item_type = ITEM_TYPE_ALIASES.get(r.get("item_type"), r.get("item_type"))
    if item_type not in ZOTERO_TYPES:
        item_type = "journalArticle" if r.get("doi") else "book"
    creators = []
    for name in r.get("authors") or []:
        if "," in name:  # "Last, First"
            last, _, first = name.partition(",")
        else:  # "First Last"
            first, _, last = name.rpartition(" ")
        creators.append(
            {"creatorType": "author", "lastName": last.strip(), "firstName": first.strip()}
        )
    item = {
        "itemType": item_type,
        "title": r["title"],
        "creators": creators,
        "date": str(r.get("year") or ""),
    }
    if r.get("doi") and item_type in (
        "journalArticle",
        "bookSection",
        "book",
        "report",
        "thesis",
        "dataset",
    ):
        item["DOI"] = r["doi"]
    if r.get("isbn") and item_type in ("book", "bookSection"):
        item["ISBN"] = r["isbn"]
    if r.get("publisher") and item_type in ("book", "bookSection", "report"):
        item["publisher"] = r["publisher"]
    if r.get("container") and item_type in ("journalArticle", "bookSection"):
        item["publicationTitle" if item_type == "journalArticle" else "bookTitle"] = r["container"]
    return item


def request_note(r):
    from html import escape

    lines = [f"<h2>wety source request</h2><p><b>Needed:</b> {escape(r['needed'])}</p>"]
    for ask in r["asks"]:
        part = f" Part: {ask['part']}." if ask.get("part") else ""
        codes = ", ".join(ask["codes"]) or "research"
        lines.append(
            f"<p><b>{escape(codes)}</b> ({escape(ask['run'])}): {escape(ask['why'] + part)}</p>"
        )
        tried = "; ".join(f"{x['route']}: {x['outcome']}" for x in ask["routes_tried"])
        if tried:
            lines.append(f"<p>Tried: {escape(tried)}</p>")
    lines.append(
        f"<p>Request key: {escape(r['key'])}. Attach the file to this item, then run "
        "<code>dating holdings sync</code>.</p>"
    )
    return "".join(lines)


def same_work(r, d):
    """Whether a Zotero item's data is the requested work: DOI or ISBN, else exact title
    (and year, when both have one)."""
    if r.get("doi") and d.get("DOI"):
        return r["doi"].lower() == d["DOI"].lower()
    if r.get("isbn") and d.get("ISBN"):
        return bool(set(isbns(r["isbn"])) & set(isbns(d["ISBN"])))
    title = re.sub(r"\W+", " ", (r.get("title") or "").lower()).strip()
    same_title = title == re.sub(r"\W+", " ", (d.get("title") or "").lower()).strip()
    years = (str(r.get("year") or ""), (re.findall(r"\d{4}", d.get("date", "")) or [""])[0])
    return same_title and (not all(years) or years[0] == years[1])


def find_library_item(client, r):
    for probe in (r.get("doi"), r.get("title")):
        if probe:
            for item in client.search(probe):
                if item["data"]["itemType"] not in ("attachment", "note") and same_work(
                    r, item["data"]
                ):
                    return item["key"]
    return None


def push_requests(client, topic="periods"):
    """File open requests in Zotero: reuse a matching library item (DOI/ISBN) or create
    one, add it to wety/dating/<topic> and wety/needs-fulltext, tag it, and add a note."""
    topic_key = client.ensure_collection(["wety", "dating", topic])
    queue_key = client.ensure_collection(["wety", "needs-fulltext"])
    filed = []
    with locked(REQUESTS):
        queue = load(REQUESTS, "requests")
        for key, r in queue["requests"].items():
            if r["status"] != "open":
                continue
            existing = find_library_item(client, r)
            item_key = existing or client.create("items", [zotero_item(r)])[0]
            tag = NEEDS_PART if r["needed"] in ("supplement", "dataset") else NEEDS_FULLTEXT
            client.add_to_item(item_key, collections=[topic_key, queue_key], tags=[tag])
            client.create(
                "items",
                [
                    {
                        "itemType": "note",
                        "parentItem": item_key,
                        "note": request_note(r),
                        "tags": [{"tag": "wety:request"}],
                    }
                ],
            )
            set_status(
                r, "filed", zotero={"item_key": item_key, "reused_existing_item": bool(existing)}
            )
            filed.append({"request": key, "zotero_item": item_key, "reused": bool(existing)})
            atomic_json(REQUESTS, queue)  # each filing survives a later failure
    return {"filed": filed}


def untag_supplied(client):
    """After ingest: swap the needs tag for wety:ingested on supplied requests' items."""
    done = []
    for r in load(REQUESTS, "requests")["requests"].values():
        if r["status"] == "supplied" and r.get("zotero"):
            for tag in (NEEDS_FULLTEXT, NEEDS_PART):
                client.remove_tag(r["zotero"]["item_key"], tag)
            client.add_to_item(r["zotero"]["item_key"], tags=[SUPPLIED])
            done.append(r["key"])
    return done


def update_request(key, status, note=None, holding=None):
    with locked(REQUESTS):
        queue = load(REQUESTS, "requests")
        r = queue["requests"].get(key)
        if not r:
            raise ValueError("unknown request " + key)
        extra = {k: v for k, v in (("note", note), ("holding", holding)) if v}
        if holding and holding not in load(HOLDINGS, "holdings")["holdings"]:
            raise ValueError("unknown holding " + holding)
        set_status(r, status, **extra)
        atomic_json(REQUESTS, queue)
    return r


def main(argv=None):
    import argparse

    p = argparse.ArgumentParser(
        prog="dating holdings",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    sub = p.add_subparsers(dest="command", required=True)
    a = sub.add_parser("add", help="ingest a local PDF")
    a.add_argument("file", type=Path)
    a.add_argument("--title", required=True)
    a.add_argument("--author", action="append", default=[], help='"Last, First"; repeatable')
    a.add_argument("--year", type=int)
    a.add_argument("--doi")
    a.add_argument("--isbn")
    a.add_argument("--publisher")
    a.add_argument("--edition")
    for q in (a, z := sub.add_parser("zotero", help="ingest the PDFs of Zotero items")):
        q.add_argument("--extent", help='what the file covers, e.g. "chapter 3, pp. 41-88"')
        q.add_argument("--ocr", choices=("auto", "force", "off"), default="auto")
        q.add_argument("--languages", default="eng", help="tesseract languages, e.g. eng+deu+grc")
    z.add_argument("keys", nargs="+", help="Zotero item or attachment keys")
    sub.add_parser("sync", help="ingest new PDFs in the wety collections and filed requests")
    sub.add_parser("list")
    args = p.parse_args(argv)
    if args.command == "add":
        bib = {
            "item_type": "book",
            "title": args.title,
            "authors": args.author,
            "year": args.year,
            "doi": args.doi,
            "isbn": args.isbn,
            "publisher": args.publisher,
            "edition": args.edition,
        }
        bib = {k: v for k, v in bib.items() if v not in (None, [])}
        bib["citation"] = citation(bib)
        result = ingest(args.file, bib, extent=args.extent, ocr=args.ocr, languages=args.languages)
    elif args.command == "zotero":
        from .zotero import Zotero

        client = Zotero()
        result = [
            r
            for key in args.keys
            for r in ingest_zotero_item(
                client, key, extent=args.extent, ocr=args.ocr, languages=args.languages
            )
        ]
    elif args.command == "sync":
        from .zotero import Zotero

        client = Zotero()
        result = {"ingested": sync(client)}
        if any(
            r["status"] == "supplied" and r.get("zotero")
            for r in load(REQUESTS, "requests")["requests"].values()
        ):
            result["retagged"] = untag_supplied(client)
    else:
        result = search()
    print(json.dumps(result, indent=2, ensure_ascii=False))


def requests_main(argv=None):
    import argparse

    p = argparse.ArgumentParser(prog="dating requests", description="The source acquisition queue.")
    sub = p.add_subparsers(dest="command", required=True)
    sub.add_parser("list")
    m = sub.add_parser("merge", help="add a research run's requests.jsonl to the queue")
    m.add_argument("run", type=Path)
    s = sub.add_parser(
        "push", help="file open requests in Zotero (asks Zotero for write access once)"
    )
    s.add_argument("--topic", default="periods", help="collection under wety/dating")
    u = sub.add_parser("set", help="change a request's status")
    u.add_argument("key")
    u.add_argument("status", choices=REQUEST_STATUSES)
    u.add_argument("--note")
    u.add_argument("--holding")
    args = p.parse_args(argv)
    if args.command == "list":
        queue = load(REQUESTS, "requests")["requests"].values()
        result = [
            {
                "key": r["key"],
                "status": r["status"],
                "priority": r.get("priority"),
                "needed": r["needed"],
                "title": r.get("title"),
                "codes": sorted({c for ask in r["asks"] for c in ask["codes"]}),
                "zotero": (r.get("zotero") or {}).get("item_key"),
                "holding": r.get("holding"),
            }
            for r in sorted(
                queue, key=lambda r: (r["status"] != "open", r.get("priority") != "high", r["key"])
            )
        ]
    elif args.command == "merge":
        result = merge_requests(args.run)
    elif args.command == "push":
        from .zotero import Zotero

        result = push_requests(Zotero(), args.topic)
    else:
        result = update_request(args.key, args.status, args.note, args.holding)
    print(json.dumps(result, indent=2, ensure_ascii=False))
