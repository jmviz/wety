import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import tempfile
import threading
import time
import unittest
from urllib.parse import parse_qs, urlparse
from unittest.mock import patch

import pymupdf

from harness import holdings
from harness.frontier_tools import ResearchTools, check
from harness.research_packet import build
from harness.zotero import Zotero, ZoteroError

PASSAGE = "Old High German is conventionally dated from c. 750 to 1050."


def text_pdf(path, pages=3, label_start=11):
    doc = pymupdf.open()
    for i in range(pages):
        page = doc.new_page()
        page.insert_text((72, 100), "A Test Handbook of Germanic" if i == 0 else f"Body page {i}")
        if i == 1:
            page.insert_text((72, 140), PASSAGE)
    doc.set_page_labels([{"startpage": 0, "prefix": "", "style": "D", "firstpagenum": label_start}])
    doc.save(path)


def scanned_pdf(path):
    src = pymupdf.open()
    page = src.new_page()
    page.insert_text((72, 100), "Gothic is attested from 350 to 380", fontsize=18)
    pix = page.get_pixmap(dpi=200)
    doc = pymupdf.open()
    scan = doc.new_page(width=page.rect.width, height=page.rect.height)
    scan.insert_image(scan.rect, pixmap=pix)
    doc.save(path)


class Sandbox(unittest.TestCase):
    """A repository root in a temporary directory: holdings, requests, receipts,
    artifacts and texts all land there."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name).resolve()
        data = self.repo / "data"
        for name, value in {
            "REPO": self.repo,
            "DATA": data,
            "HOLDINGS": data / "registry/holdings.json",
            "REQUESTS": data / "registry/source-requests.json",
            "RECEIPTS": data / "registry/acquisitions",
            "CACHE_DIR": str(data / "retrieved"),
        }.items():
            p = patch.object(holdings, name, value)
            p.start()
            self.addCleanup(p.stop)
        self.files = self.repo / "supplied"
        self.files.mkdir()


class ExtractTests(Sandbox):
    def test_text_pdf_keeps_acquire_format_and_labels(self):
        path = self.files / "book.pdf"
        text_pdf(path)
        text, meta = holdings.extract_pdf(path)
        with pymupdf.open(path) as doc:
            acquire_format = "\n".join(
                f"\n[PDF page {i + 1}]\n" + page.get_text() for i, page in enumerate(doc)
            )
        self.assertEqual(text, acquire_format)
        self.assertEqual(meta["kind"], "pdf_text")
        self.assertEqual(meta["page_labels"], ["11", "12", "13"])
        self.assertEqual(holdings.page_at(text, text.index(PASSAGE)), 2)

    def test_scanned_pages_are_ocred_or_skipped_when_too_many(self):
        path = self.files / "scan.pdf"
        scanned_pdf(path)
        text, meta = holdings.extract_pdf(path, languages="eng")
        self.assertEqual(meta["kind"], "pdf_ocr_text")
        self.assertEqual(meta["ocr"]["page_list"], "1")
        self.assertIn("350 to 380", text)
        self.assertIn("page image", meta["coverage"])
        text, meta = holdings.extract_pdf(path, max_ocr_pages=0)
        self.assertEqual(meta["kind"], "pdf_text")
        self.assertIn("ocr_skipped", meta)
        self.assertEqual(meta["pages_without_text"], "1")

    def test_scan_with_text_layer_is_flagged(self):
        path = self.files / "layered.pdf"
        scanned_pdf(path)
        with pymupdf.open(path) as doc:
            doc[0].insert_text((72, 300), "an embedded OCR layer of unknown quality")
            doc.saveIncr()
        text, meta = holdings.extract_pdf(path)
        self.assertEqual((meta["kind"], meta["scan_text_layer"]["page_list"]), ("pdf_text", "1"))
        self.assertIn("page image", meta["coverage"])

    def test_zotero_item_names(self):
        item = holdings.zotero_item({"title": "T", "authors": ["Albert C. Baugh", "Cable, Thomas"]})
        self.assertEqual(
            [(c["lastName"], c["firstName"]) for c in item["creators"]],
            [("Baugh", "Albert C."), ("Cable", "Thomas")],
        )

    def test_compact_pages(self):
        self.assertEqual(holdings.compact_pages([1, 2, 3, 7, 9, 10]), "1-3,7,9-10")
        self.assertEqual(holdings.expand_pages("1-3,7"), [1, 2, 3, 7])


class HoldingTests(Sandbox):
    def ingest(self, **kw):
        path = self.files / "book.pdf"
        if not path.exists():
            text_pdf(path)
        bib = {
            "title": "A Test Handbook of Germanic",
            "authors": ["Doe, Jane"],
            "year": 2001,
            "isbn": "978-0-00-000000-2",
        }
        bib["citation"] = holdings.citation(bib)
        return holdings.ingest(path, bib, **kw)

    def test_ingest_retains_bytes_receipt_and_text(self):
        h = self.ingest(zotero={"item_key": "ITEM0001", "attachment_key": "ATT00001"})
        raw = self.repo / h["file"]["raw_path"]
        self.assertEqual(holdings.file_sha(raw), h["file"]["sha256"])
        self.assertEqual(h["id"], h["file"]["sha256"][:16])
        receipt = json.loads((self.repo / h["file"]["receipt"]).read_text())
        self.assertEqual(receipt["origin"], "maintainer_supplied")
        self.assertEqual(receipt["zotero"]["item_key"], "ITEM0001")
        self.assertNotIn(str(self.repo), json.dumps(receipt))
        stored = holdings.texts_dir() / (h["text"]["sha256"] + ".txt")
        self.assertEqual(holdings.file_sha(stored), h["text"]["sha256"])
        self.assertEqual(h["identity_hint"]["title_words_found"], "3/3")
        # The tracked index holds metadata, never the text.
        index = holdings.HOLDINGS.read_text()
        self.assertNotIn(PASSAGE, index)
        self.assertNotIn(str(self.repo), index)
        # Same bytes and settings: nothing is re-ingested.
        self.assertEqual(self.ingest(zotero=h["zotero"])["ingested_at"], h["ingested_at"])
        # Different OCR settings: a new text version, the old one kept in history,
        # and no second receipt for the same retained bytes.
        again = self.ingest(ocr="off")
        self.assertEqual(len(again["previous_texts"]), 1)
        self.assertEqual(again["file"]["receipt"], h["file"]["receipt"])
        self.assertEqual(len(list(holdings.RECEIPTS.iterdir())), 1)

    def test_non_pdf_is_refused(self):
        p = self.files / "book.epub"
        p.write_bytes(b"PK\x03\x04")
        with self.assertRaisesRegex(ValueError, "PDF required"):
            holdings.ingest(p, {"title": "x"})

    def run_dir(self):
        root = self.repo / "data/workspaces/dating/_research/run"
        root.mkdir(parents=True)
        brief = {
            "deadline_unix": time.time() + 120,
            "research_deadline_unix": time.time() + 60,
            "limits": {"searches": None, "acquisitions": 0, "tool_calls": None},
        }
        (root / "brief.json").write_text(json.dumps(brief))
        return root

    def test_open_find_page_check_and_packet(self):
        h = self.ingest(zotero={"item_key": "ITEM0001", "attachment_key": "ATT00001"})
        root = self.run_dir()
        tools = ResearchTools(root)
        found = tools.call("library", {"query": "germanic doe"})
        self.assertEqual(found["holdings"][0]["holding_id"], h["id"])
        self.assertEqual(tools.call("library", {"query": "nothing such"})["holdings"], [])
        opened = tools.call("open", {"holding_id": h["id"]})
        sid = opened["source_id"]
        self.assertEqual(opened["holding"]["origin"], "maintainer_supplied")
        self.assertIsInstance(opened["page_labels"], str)  # not the whole list
        # Opening is local: it does not use the (zero) acquisition allowance.
        self.assertEqual(tools.state()["acquisitions"], 0)
        with self.assertRaises(ValueError):
            tools.call("open", {"holding_id": "../../etc/passwd"})
        match = tools.call("find", {"source_id": sid, "query": "c. 750 to 1050"})["matches"][0]
        self.assertEqual((match["pdf_page"], match["page_label"]), (2, "12"))
        with patch("harness.frontier_tools.REPO", self.repo):
            page = tools.call("page", {"source_id": sid, "page": 2})
            self.assertEqual(page["page_label"], "12")
            # OCR-derived text is still a PDF: its pages render.
            state = json.loads((root / "tools-state.json").read_text())
            state["sources"][sid]["kind"] = "pdf_ocr_text"
            (root / "tools-state.json").write_text(json.dumps(state))
            self.assertEqual(tools.call("page", {"source_id": sid, "page": 1})["page"], 1)
        row = {
            "id": "a1",
            "code": "goh",
            "source_id": sid,
            "quote": PASSAGE,
            "locator": {"page": "12"},
            "subject": {"code": "goh", "source_label": "Old High German", "mapping": "exact"},
            "event": "conventional_period",
            "role": "convention",
            "method": "conventional_period",
            "native_time": {
                "form": "duration",
                "unit": "CE",
                "lower": 750,
                "upper": 1050,
                "approximate": True,
            },
            "qualifications": [],
        }
        for name in ("report.md", "sources.json", "self-check.json"):
            (root / name).write_text("{}")
        (root / "assertions.jsonl").write_text(json.dumps(row) + "\n")
        self.assertEqual(check(root)["errors"], [])
        review = {
            "packet_id": "test",
            "claim_prefix": "ev_test_",
            "reviewer": "Reviewer",
            "scope": "test",
            "review_independence": "same session",
            "retrieved": "2026-10-03",
            "sources": {
                sid: {"id": "src_test_handbook", "type": "handbook", "title": "A Test Handbook"}
            },
            "claims": [{"assertion": "a1", "status": "candidate"}],
        }
        languages = {"goh": {"kind": "regular", "canonicalName": "Old High German"}}
        with patch("harness.research_packet.REPO", self.repo):
            result = build(root, review, languages=languages, texts_dir=holdings.texts_dir())
        reg = result["registrations"]["src_test_handbook"]
        self.assertIsNone(reg["url"])
        self.assertEqual(reg["acquisition"]["origin"], "maintainer_supplied")
        self.assertEqual(reg["acquisition"]["zotero"]["item_key"], "ITEM0001")
        self.assertEqual(reg["sha256"], h["text"]["sha256"])

    def test_requests_merge_and_close_on_supply(self):
        root = self.run_dir()
        tools = ResearchTools(root)
        ask = {
            "title": "A Test Handbook of Germanic",
            "isbn": "9780000000002",
            "needed": "work",
            "why": "Defines the Old High German period.",
            "codes": ["goh"],
            "routes_tried": [{"route": "publisher", "outcome": "403"}],
        }
        self.assertEqual(tools.call("request", ask)["recorded"], "isbn:9780000000002")
        tools.call(
            "request",
            {
                **ask,
                "title": "Supplementary data",
                "isbn": None,
                "doi": "10.1/x",
                "needed": "supplement",
            },
        )
        with self.assertRaisesRegex(ValueError, "needed"):
            tools.call("request", {**ask, "needed": "everything"})
        self.assertEqual(holdings.merge_requests(root)["merged"], 2)
        self.assertEqual(holdings.merge_requests(root)["merged"], 0)  # idempotent
        h = self.ingest()  # same ISBN, hyphenated: closes the work request only
        queue = holdings.load(holdings.REQUESTS, "requests")["requests"]
        self.assertEqual(queue["isbn:9780000000002"]["status"], "supplied")
        self.assertEqual(queue["isbn:9780000000002"]["holding"], h["id"])
        self.assertEqual(queue["doi:10.1/x"]["status"], "open")
        self.assertEqual(
            tools.call("library", {"query": "supplementary"})["requests"][0]["status"], "open"
        )
        holdings.update_request("doi:10.1/x", "unobtainable", note="not published")
        queue = holdings.load(holdings.REQUESTS, "requests")["requests"]
        self.assertEqual(
            [x["status"] for x in queue["doi:10.1/x"]["history"]], ["open", "unobtainable"]
        )


class FakeZotero(BaseHTTPRequestHandler):
    """Just enough of Zotero's local API: items, children, collections, files, writes."""

    state = None

    def log_message(self, *args):
        pass

    def reply(self, status, body=None, headers=None):
        raw = (
            b""
            if body is None
            else (body if isinstance(body, bytes) else json.dumps(body).encode())
        )
        self.send_response(status)
        self.send_header("Zotero-Server-ID", "TESTSERVER")
        for k, v in (headers or {}).items():
            self.send_header(k, v)
        self.end_headers()
        self.wfile.write(raw)

    def body(self):
        return json.loads(self.rfile.read(int(self.headers["Content-Length"])))

    def authorized(self):
        s = self.state
        if (
            self.headers.get("Zotero-API-Key") != "KEY"
            or self.headers.get("Zotero-Server-ID") != "TESTSERVER"
        ):
            self.reply(403)
            return False
        s["writes"] += 1
        return True

    def do_GET(self):
        s = self.state
        path = self.path.split("?")[0].removeprefix("/api")
        if path == "/":
            return self.reply(200, b"")
        parts = path.removeprefix("/users/0/").split("/")
        if parts == ["collections"]:
            return self.reply(
                200, list(s["collections"].values()) if "start=0" in self.path else []
            )
        if parts[0] == "collections" and len(parts) == 2:
            return self.reply(200, s["collections"][parts[1]])
        if parts[0] == "collections" and parts[2:] == ["items", "top"]:
            items = [
                i
                for i in s["items"].values()
                if parts[1] in i["data"].get("collections", []) and "parentItem" not in i["data"]
            ]
            return self.reply(200, items if "start=0" in self.path else [])
        if parts == ["items", "top"]:
            q = parse_qs(urlparse(self.path).query).get("q", [""])[0]
            hits = [i for i in s["items"].values() if q and q in json.dumps(i["data"])]
            return self.reply(200, hits if "start=0" in self.path else [])
        if parts[0] == "items" and len(parts) == 2:
            return (
                self.reply(200, s["items"][parts[1]]) if parts[1] in s["items"] else self.reply(404)
            )
        if parts[0] == "items" and parts[2] == "children":
            kids = [i for i in s["items"].values() if i["data"].get("parentItem") == parts[1]]
            return self.reply(200, kids if "start=0" in self.path else [])
        if parts[0] == "items" and parts[2:] == ["file", "view", "url"]:
            f = s["files"].get(parts[1])
            return self.reply(200, ("file://" + str(f)).encode()) if f else self.reply(404)
        self.reply(404)

    def do_POST(self):
        s = self.state
        path = self.path.removeprefix("/api")
        if path == "/local/authorize":
            return self.reply(200, {"key": "KEY", "remember": True})
        if not self.authorized():
            return
        kind = path.removeprefix("/users/0/")
        out = {}
        for i, obj in enumerate(self.body()):
            key = f"{kind[0].upper()}{len(s['items']) + len(s['collections']):07d}"
            record = {"key": key, "version": 1, "data": {**obj, "key": key}}
            (s["collections"] if kind == "collections" else s["items"])[key] = record
            out[str(i)] = {"key": key}
        self.reply(200, {"successful": out, "failed": {}})

    def do_PATCH(self):
        s = self.state
        if not self.authorized():
            return
        key = self.path.removeprefix("/api/users/0/items/")
        item = s["items"][key]
        if self.headers.get("If-Unmodified-Since-Version") != str(item["version"]):
            return self.reply(412)
        item["data"].update(self.body())
        item["version"] += 1
        self.reply(204)


class ZoteroTests(Sandbox):
    def setUp(self):
        super().setUp()
        pdf = self.files / "handbook.pdf"
        text_pdf(pdf)
        FakeZotero.state = {
            "writes": 0,
            "collections": {},
            "items": {
                "BOOK0001": {
                    "key": "BOOK0001",
                    "version": 3,
                    "meta": {"parsedDate": "2001"},
                    "data": {
                        "key": "BOOK0001",
                        "itemType": "book",
                        "title": "A Test Handbook of Germanic",
                        "creators": [
                            {"creatorType": "author", "lastName": "Doe", "firstName": "Jane"}
                        ],
                        "ISBN": "978-0-00-000000-2",
                        "publisher": "Test Press",
                        "collections": [],
                        "tags": [{"tag": "mine"}],
                    },
                },
                "PDF00001": {
                    "key": "PDF00001",
                    "version": 3,
                    "data": {
                        "key": "PDF00001",
                        "itemType": "attachment",
                        "parentItem": "BOOK0001",
                        "contentType": "application/pdf",
                        "linkMode": "imported_file",
                        "title": "PDF",
                    },
                },
            },
            "files": {"PDF00001": pdf},
        }
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), FakeZotero)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.addCleanup(self.server.shutdown)
        self.client = Zotero(f"http://127.0.0.1:{self.server.server_port}/api", key="KEY")

    def test_ingest_item_by_key(self):
        [result] = holdings.ingest_zotero_item(self.client, "BOOK0001")
        h = holdings.load(holdings.HOLDINGS, "holdings")["holdings"][result["holding"]]
        self.assertEqual(h["authors"], ["Doe, Jane"])
        self.assertEqual(h["zotero"]["attachment_key"], "PDF00001")
        self.assertEqual(FakeZotero.state["writes"], 0)  # reading needs no write access

    def test_push_reuses_item_and_sync_closes_request(self):
        root = self.repo / "data/workspaces/dating/_research/run"
        root.mkdir(parents=True)
        (root / "requests.jsonl").write_text(
            json.dumps(
                {
                    "title": "A Test Handbook of Germanic",
                    "isbn": "978-0-00-000000-2",
                    "needed": "work",
                    "why": "goh period",
                    "codes": ["goh"],
                }
            )
            + "\n"
            + json.dumps(
                {
                    "title": "Missing Article",
                    "authors": ["Roe, Ann"],
                    "year": 2020,
                    "doi": "10.9/miss",
                    "needed": "work",
                    "why": "sla-pro date",
                    "codes": ["sla-pro"],
                }
            )
            + "\n"
        )
        holdings.merge_requests(root)
        filed = {f["request"]: f for f in holdings.push_requests(self.client)["filed"]}
        self.assertTrue(filed["isbn:9780000000002"]["reused"])
        self.assertFalse(filed["doi:10.9/miss"]["reused"])
        items = FakeZotero.state["items"]
        book = items["BOOK0001"]["data"]
        self.assertEqual({t["tag"] for t in book["tags"]}, {"mine", holdings.NEEDS_FULLTEXT})
        self.assertEqual(book["title"], "A Test Handbook of Germanic")  # otherwise untouched
        new = items[filed["doi:10.9/miss"]["zotero_item"]]["data"]
        self.assertEqual((new["itemType"], new["DOI"]), ("journalArticle", "10.9/miss"))
        names = {c["data"]["name"] for c in FakeZotero.state["collections"].values()}
        self.assertEqual(names, {"wety", "dating", "periods", "needs-fulltext"})
        notes = [i for i in items.values() if i["data"]["itemType"] == "note"]
        self.assertEqual(len(notes), 2)
        self.assertTrue(any("goh period" in n["data"]["note"] for n in notes))
        # Pushing again files nothing new.
        self.assertEqual(holdings.push_requests(self.client)["filed"], [])
        report = holdings.sync(self.client)
        self.assertEqual(sorted(r["status"] for r in report), ["ingested", "no_pdf_attachment"])
        queue = holdings.load(holdings.REQUESTS, "requests")["requests"]
        self.assertEqual(queue["isbn:9780000000002"]["status"], "supplied")
        self.assertEqual(queue["doi:10.9/miss"]["status"], "filed")
        holdings.untag_supplied(self.client)
        self.assertEqual(
            {t["tag"] for t in items["BOOK0001"]["data"]["tags"]}, {"mine", holdings.SUPPLIED}
        )

    def test_writes_are_confined(self):
        with self.assertRaises(ValueError):
            self.client.ensure_collection(["Other"])
        with self.assertRaises(ValueError):
            self.client.add_to_item("BOOK0001", tags=["mine2"])
        with self.assertRaises(ValueError):
            self.client.remove_tag("BOOK0001", "mine")

    def test_disabled_local_api_is_explained(self):
        class Off(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                self.send_response(403)
                self.end_headers()
                self.wfile.write(b"Local API is not enabled")

        server = ThreadingHTTPServer(("127.0.0.1", 0), Off)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        self.addCleanup(server.shutdown)
        with self.assertRaisesRegex(ZoteroError, "Settings > Advanced"):
            Zotero(f"http://127.0.0.1:{server.server_port}/api").item("X")


if __name__ == "__main__":
    unittest.main()
