"""English Wikipedia language infoboxes: the `era` field ("c. 450 – 1150") and
the `extinct` field ("1st century AD"), for every Wiktionary language with a
Wikipedia article. Wikitext is fetched through the MediaWiki API (50 titles per
request) and cached; parsing is conservative (unparseable text is kept as a
raw note, not guessed)."""

from __future__ import annotations

import json
import os
import re
import time
import urllib.parse
import urllib.request

from .. import glotto
from ..config import CACHE_DIR, USER_AGENT, now
from workflow_tools.paths import selected
from ..schema import Claim, Source
from ..time_utils import parse_era
from . import claim_id, write_claims

API = "https://en.wikipedia.org/w/api.php"
CACHE = os.path.join(CACHE_DIR, "wikipedia_wikitext.json")
REDIRECTS = os.path.join(CACHE_DIR, "wikipedia_redirects.json")

SOURCE = Source(
    id="src_enwiki_infobox",
    type="encyclopedia",
    title="English Wikipedia, Infobox language `era` / `extinct` fields",
    url="https://en.wikipedia.org",
    license="CC-BY-SA-4.0",
)


def fetch_wikitext(
    titles: list[str], cache: dict[str, str | None], redirects_out: dict[str, str]
) -> None:
    todo = [t for t in titles if t not in cache or t not in redirects_out]
    for i in range(0, len(todo), 50):
        batch = todo[i : i + 50]
        params = {
            "action": "query",
            "prop": "revisions",
            "rvprop": "content",
            "rvslots": "main",
            "titles": "|".join(batch),
            "redirects": 1,
            "format": "json",
            "formatversion": 2,
            "maxlag": 5,
        }
        url = API + "?" + urllib.parse.urlencode(params)
        for attempt in range(5):
            try:
                req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
                with urllib.request.urlopen(req, timeout=120) as r:
                    data = json.load(r)
                break
            except Exception as e:  # noqa: BLE001
                time.sleep(5 * (attempt + 1))
                if attempt == 4:
                    raise e
        redirects = {x["from"]: x["to"] for x in data["query"].get("redirects", [])}
        normalized = {x["from"]: x["to"] for x in data["query"].get("normalized", [])}
        pages = {p["title"]: p for p in data["query"]["pages"]}
        for t in batch:
            key = normalized.get(t, t)
            key = redirects.get(key, key)
            redirects_out[t] = key
            p = pages.get(key)
            cache[t] = (
                p["revisions"][0]["slots"]["main"]["content"] if p and "revisions" in p else None
            )
        print(f"  wikipedia: {min(i + 50, len(todo))}/{len(todo)} fetched", flush=True)
        time.sleep(0.3)


def infobox_field(wikitext: str, field: str) -> str | None:
    m = re.search(r"\{\{\s*Infobox language\b", wikitext, re.I)
    if not m:
        return None
    body = wikitext[m.end() :]
    # take the field's value up to the next top-level '|' or the closing '}}'
    fm = re.search(rf"\n\s*\|\s*{field}\s*=\s*(.*?)(?=\n\s*\||\n\}}\}})", body, re.S)
    return fm.group(1).strip() if fm and fm.group(1).strip() else None


def run(limit: int | None = None) -> dict:
    g = glotto.load()
    with open(selected("wiktionary_languages"), encoding="utf-8") as f:
        wikt_raw = json.load(f)
    os.makedirs(CACHE_DIR, exist_ok=True)
    cache: dict[str, str | None] = {}
    if os.path.exists(CACHE):
        with open(CACHE, encoding="utf-8") as f:
            cache = json.load(f)
    redirects: dict[str, str] = {}
    if os.path.exists(REDIRECTS):
        with open(REDIRECTS, encoding="utf-8") as f:
            redirects = json.load(f)
    # Article subjects remain separate from linked catalog concepts. A shared
    # article with incompatible destinations is not resolved by regular-first.
    from collections import defaultdict

    contexts = defaultdict(set)
    for e in wikt_raw.values():
        rec = g.wikt.get(e["mainCode"], {})
        art = (e.get("wikipediaArticle") or "").strip()
        if art and "#" not in art and rec.get("glottocode"):
            contexts[art].add(rec["glottocode"])
    title_to_g = {t: next(iter(gs)) if len(gs) == 1 else None for t, gs in contexts.items()}
    titles = sorted(title_to_g)[:limit]
    try:
        fetch_wikitext(titles, cache, redirects)
    finally:
        with open(CACHE, "w", encoding="utf-8") as f:
            json.dump(cache, f, ensure_ascii=False)
        with open(REDIRECTS, "w", encoding="utf-8") as f:
            json.dump(redirects, f, ensure_ascii=False)
    SOURCE.retrieved = now()
    claims: list[Claim] = []
    unparsed = skipped_redirects = 0
    done: set[str] = set()
    for title in titles:
        wt = cache.get(title)
        if not wt:
            continue
        gid = title_to_g[title]
        target = redirects.get(title, title)
        if target != title and target in title_to_g and title_to_g[target] != gid:
            skipped_redirects += 1
            gid = None  # Preserve the article claim without choosing its catalog scope.
        if target in done:
            continue
        done.add(target)
        name = target
        evidence = {
            "subject_id": "enwiki:" + target,
            "subject_type": "unresolved",
            "catalog_mapping": "unresolved",
            "catalog_candidates": sorted(contexts.get(title, set()) | contexts.get(target, set())),
            "catalog_mapping_basis": "article link; no temporal scope equivalence",
        }
        extracted = {"program": "harness.bootstrap.wikipedia", "run": "bootstrap"}
        era = infobox_field(wt, "era")
        if era:
            iv = parse_era(era)
            if iv and (iv.earliest is not None or iv.latest is not None):
                t = {
                    "earliest": iv.earliest,
                    "latest": iv.latest,
                    "precision": iv.precision,
                    "open_end": iv.latest is None,
                    "conversion": "parsed from infobox era text",
                }
                claims.append(
                    Claim(
                        id=claim_id(gid, SOURCE.id, title + "_era"),
                        languoid=gid,
                        languoid_name=name,
                        kind="conventional_period",
                        time=t,
                        source=SOURCE.id,
                        evidence=evidence,
                        locator=f"{title}: |era=",
                        quote=era,
                        quote_verified=True,
                        extracted_by=extracted,
                        notes="infobox era; a periodization convention, usually uncited in the infobox itself",
                    )
                )
            else:
                unparsed += 1
        ext = infobox_field(wt, "extinct")
        if ext:
            iv = parse_era(ext)
            if iv and iv.earliest is not None:
                t = {
                    "earliest": iv.earliest,
                    "latest": iv.latest if iv.latest is not None else iv.earliest,
                    "precision": iv.precision,
                    "conversion": "parsed from infobox extinct text",
                }
                claims.append(
                    Claim(
                        id=claim_id(gid, SOURCE.id, title + "_extinct"),
                        languoid=gid,
                        languoid_name=name,
                        kind="extinction",
                        time=t,
                        source=SOURCE.id,
                        evidence=evidence,
                        locator=f"{title}: |extinct=",
                        quote=ext,
                        quote_verified=True,
                        extracted_by=extracted,
                    )
                )
            else:
                unparsed += 1
    summary = write_claims(g, SOURCE, claims)
    summary.update(
        {
            "titles": len(titles),
            "with_wikitext": sum(1 for t in titles if cache.get(t)),
            "unparsed_fields": unparsed,
            "skipped_redirects_to_other_language": skipped_redirects,
            "ambiguous_article_contexts": {
                t: sorted(gs) for t, gs in contexts.items() if len(gs) > 1
            },
        }
    )
    return summary
