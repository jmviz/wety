"""Parsoid HTML of the pages a pages-articles dump holds in a namespace Wikimedia
Enterprise does not serve (Reconstruction, 118), at the dump's own revisions.

    uv run --frozen python -m source_tools.wiktionary_html DUMP.xml.bz2 OUT_DIR [--namespace 118]

Writes OUT_DIR/pages.ndjson.gz, one line per non-redirect page in Enterprise's record
shape (`name`, `identifier`, `namespace`, `version`, `article_body.html` from
/w/rest.php/v1/revision/{rev}/html, `article_body.wikitext` from the dump), so the
entries reader takes both alike; and OUT_DIR/fetch.json (counts, failures, times). A
rerun skips pages already written, so an interrupted fetch resumes. Wikimedia limits
bots identified by User-Agent to 200 requests a minute and asks for at most 3 at a
time (mediawiki.org/wiki/Wikimedia_APIs/Rate_limits): requests go out --workers at a
time (default 3) and at most --per-minute (default 180); HTTP 429 and 5xx are retried
after Retry-After or a backoff. Adopt OUT_DIR with `wety-data adopt` once fetch.json
shows no failures.
"""

import argparse
import gzip
import html
import json
import os
import re
import subprocess
import threading
import time
import urllib.error
import urllib.request
import zlib
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from source_tools.acquire import USER_AGENT
from workflow_tools.artifacts import now

REST = "https://en.wiktionary.org/w/rest.php/v1/revision/{}/html"
TAG = re.compile(r"^\s*<(title|ns|id|redirect)\b(?: title=\"([^\"]*)\")?\s*/?>(?:([^<]*)</\1>)?")


def dump_pages(dump, namespace):
    """(title, page id, revision, wikitext) of each non-redirect page in namespace."""
    bz = subprocess.Popen(["bzcat", str(dump)], stdout=subprocess.PIPE, text=True, encoding="utf-8")
    page, text = None, None
    for line in bz.stdout:
        if "<page>" in line:
            page, text = {}, None
        elif page is None:
            continue
        elif text is not None:
            end = line.find("</text>")
            text.append(line if end < 0 else line[:end])
            if end >= 0:
                page["text"] = html.unescape("".join(text))
                text = None
        elif "<text" in line:
            start = line.index(">", line.index("<text")) + 1
            if line.rstrip().endswith("/>"):
                page["text"] = ""
            else:
                end = line.find("</text>", start)
                if end >= 0:
                    page["text"] = html.unescape(line[start:end])
                else:
                    text = [line[start:]]
        elif "</page>" in line:
            if page.get("ns") == str(namespace) and "redirect" not in page:
                yield page["title"], int(page["id"]), int(page["rev"]), page.get("text", "")
            page = None
        else:
            m = TAG.match(line)
            if not m:
                continue
            tag = m.group(1)
            if tag == "redirect":
                page["redirect"] = True
            elif tag == "id":
                # The page's id, then its revision's; a contributor's id comes later.
                key = "id" if "id" not in page else "rev" if "rev" not in page else None
                if key:
                    page[key] = m.group(3)
            else:
                page[tag] = html.unescape(m.group(3))
    bz.wait()
    if bz.returncode:
        raise SystemExit(f"bzcat failed with {bz.returncode}")


class Pace:
    """At most `per_minute` starts a minute, across threads."""

    def __init__(self, per_minute):
        self.gap = 60 / per_minute
        self.next = time.monotonic()
        self.lock = threading.Lock()

    def wait(self):
        with self.lock:
            now = time.monotonic()
            start = max(now, self.next)
            self.next = start + self.gap
        time.sleep(max(0, start - now))


def fetch(rev, pace, tries=8):
    request = urllib.request.Request(REST.format(rev), headers={"User-Agent": USER_AGENT})
    for attempt in range(tries):
        pace.wait()
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                return response.read().decode("utf-8"), None
        except urllib.error.HTTPError as e:
            if e.code not in (429, 500, 502, 503, 504) or attempt + 1 == tries:
                return None, f"HTTP {e.code}"
            time.sleep(max(5, int(e.headers.get("Retry-After") or 2**attempt)))
        except OSError as e:
            if attempt + 1 == tries:
                return None, str(e)
            time.sleep(max(5, 2**attempt))


def salvage(path):
    """Ids of the pages `path` holds whole. An interrupted fetch leaves a cut-off line
    and gzip member, after which nothing appended could be read: the whole lines are
    then copied into a fresh file."""
    done, lines, clean = set(), [], True
    try:
        with gzip.open(path, "rt", encoding="utf-8") as f:
            for line in f:
                try:
                    done.add(json.loads(line)["identifier"])
                except ValueError:
                    clean = False
                    break
                lines.append(line)
    except (EOFError, OSError, zlib.error):
        clean = False
    if not clean:
        tmp = path.with_suffix(".tmp")
        with gzip.open(tmp, "wt", encoding="utf-8") as f:
            f.writelines(lines)
        os.replace(tmp, path)
    return done


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("dump", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--namespace", type=int, default=118)
    parser.add_argument("--workers", type=int, default=3)
    parser.add_argument("--per-minute", type=int, default=180)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.out / "pages.ndjson.gz"
    done = salvage(path) if path.exists() else set()
    report = {"dump": str(args.dump), "namespace": args.namespace, "started_at": now(), "already_written": len(done)}
    pages = [p for p in dump_pages(args.dump, args.namespace) if p[1] not in done]
    report["to_fetch"] = len(pages)
    failures, written = [], 0
    pace = Pace(args.per_minute)
    with ThreadPoolExecutor(args.workers) as pool, gzip.open(path, "at", encoding="utf-8") as out:
        for (title, page_id, rev, text), (body, error) in zip(pages, pool.map(lambda p: fetch(p[2], pace), pages)):
            if error:
                failures.append({"name": title, "identifier": page_id, "revision": rev, "error": error})
                continue
            record = {
                "name": title,
                "identifier": page_id,
                "namespace": {"identifier": args.namespace},
                "version": {"identifier": rev},
                "url": REST.format(rev),
                "article_body": {"html": body, "wikitext": text},
            }
            out.write(json.dumps(record, ensure_ascii=False) + "\n")
            written += 1
            if written % 1000 == 0:
                out.flush()
                print(written, "of", len(pages), flush=True)
    report.update(written=written, failures=failures, completed_at=now(), user_agent=USER_AGENT)
    (args.out / "fetch.json").write_text(json.dumps(report, indent=1, ensure_ascii=False) + "\n")
    print(f"{written} written, {len(failures)} failed")


if __name__ == "__main__":
    main()
