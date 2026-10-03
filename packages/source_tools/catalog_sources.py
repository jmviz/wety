#!/usr/bin/env python3
"""
Fetch the external sources needed by build.py into sources/ and record their
provenance in sources/manifest.json.

Sources fetched:

  ISO 639-3 (SIL, the registration authority)
    iso-639-3.tab                 all 639-3 codes with 639-1 / 639-2B / 639-2T
                                  equivalents, scope (I/M/S) and type (L/E/H/A/C/S)
    iso-639-3-macrolanguages.tab  macrolanguage -> member mappings
    iso-639-3_Retirements.tab     retired codes and what replaced them
  ISO 639-2 (Library of Congress, the registration authority)
    ISO-639-2_utf-8.txt           needed for the collective (family) codes and
                                  special codes, which are not in 639-3
  ISO 639-5 (Library of Congress)
    iso639-5.tsv                  language family / group codes
  Wikidata (query.wikidata.org)
    wikidata_codes.json           every item carrying an ISO 639-1/2/3/5 or
                                  Glottolog code (global reverse index)
    wikidata_items.json           per-item snapshot for every Wikidata item
                                  referenced from languages.json / families.json:
                                  code properties, instance-of, label, redirect
                                  target, and the temporal properties that are
                                  useful for the historical-calibration project

Glottolog itself is NOT fetched here: build.py reads a local unpacked
glottolog-cldf release (https://zenodo.org/communities/glottolog or
https://github.com/glottolog/glottolog-cldf/releases).

Usage:
    python fetch_sources.py [--skip-iso] [--skip-wikidata]
"""

from __future__ import annotations

import argparse
from pathlib import Path
import tempfile
import shutil
import fcntl
from catalogs.registry import atomic, json_bytes
import datetime as dt
import hashlib
import json
import os
import sys
import time
import urllib.parse
import urllib.request

from workflow_tools.paths import DATA, CATALOG_WORKSPACE, CATALOG_SOURCES, selected
HERE = str(CATALOG_WORKSPACE)
SOURCES = str(CATALOG_SOURCES)
LANGUAGES_JSON = str(selected('wiktionary_languages'))
FAMILIES_JSON = str(selected('wiktionary_families'))

USER_AGENT = "wety/1.0 (https://github.com/jmviz/wety)"
SPARQL = "https://query.wikidata.org/sparql"

ISO_FILES = {
    "iso-639-3.tab": "https://iso639-3.sil.org/sites/iso639-3/files/downloads/iso-639-3.tab",
    "iso-639-3-macrolanguages.tab": "https://iso639-3.sil.org/sites/iso639-3/files/downloads/iso-639-3-macrolanguages.tab",
    "iso-639-3_Retirements.tab": "https://iso639-3.sil.org/sites/iso639-3/files/downloads/iso-639-3_Retirements.tab",
    "ISO-639-2_utf-8.txt": "https://www.loc.gov/standards/iso639-2/ISO-639-2_utf-8.txt",
    "iso639-5.tsv": "https://id.loc.gov/vocabulary/iso639-5.tsv",
}

# Wikidata properties. Codes first, then classification, then temporal.
WD_PROPS = {
    "P218": "iso639_1",
    "P219": "iso639_2",
    "P220": "iso639_3",
    "P1798": "iso639_5",
    "P1394": "glottocode",
    "P1627": "ethnologue",
    "P1232": "linguist_list",
    "P305": "ietf",
    "P424": "wikimedia_code",
    "P31": "instance_of",
    "P279": "subclass_of",
    "P571": "inception",
    "P576": "dissolved",
    "P580": "start_time",
    "P582": "end_time",
    "P1249": "earliest_written_record",
}
# Properties for the global reverse index.
WD_GLOBAL_PROPS = ["P218", "P219", "P220", "P1798", "P1394"]

BATCH = 300


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def http_get(url: str, accept: str | None = None, retries: int = 5) -> bytes:
    headers = {"User-Agent": USER_AGENT}
    if accept:
        headers["Accept"] = accept
    for attempt in range(retries):
        try:
            from .acquire import fetch_bytes
            return fetch_bytes(url,'catalog-http',headers=headers)
        except Exception as e:  # noqa: BLE001
            wait = 5 * (attempt + 1)
            print(f"  request failed ({e}); retrying in {wait}s", file=sys.stderr)
            time.sleep(wait)
    raise RuntimeError(f"giving up on {url}")


def sparql(query: str) -> list[dict]:
    url = SPARQL + "?" + urllib.parse.urlencode({"query": query, "format": "json"})
    data = http_get(url, accept="application/sparql-results+json")
    return json.loads(data)["results"]["bindings"]


def fetch_iso(manifest: dict) -> None:
    for name, url in ISO_FILES.items():
        print(f"fetching {name}")
        data = http_get(url)
        path = os.path.join(SOURCES, name)
        atomic(path,data)
        manifest["files"][name] = {
            "url": url,
            "retrieved": now(),
            "sha256": sha256(path),
            "bytes": len(data),
        }


def qid_of(uri: str) -> str:
    return uri.rsplit("/", 1)[-1]


def fetch_wikidata_global(manifest: dict) -> None:
    """All items with any language code property -> wikidata_codes.json."""
    items: dict[str, dict[str, list[str]]] = {}
    queries = []
    for p in WD_GLOBAL_PROPS:
        q = f"SELECT ?item ?v WHERE {{ ?item wdt:{p} ?v . }}"
        print(f"wikidata global query {p} ({WD_PROPS[p]})")
        rows = sparql(q)
        queries.append({"property": p, "query": q, "rows": len(rows), "at": now()})
        key = WD_PROPS[p]
        for r in rows:
            items.setdefault(qid_of(r["item"]["value"]), {}).setdefault(key, []).append(
                r["v"]["value"]
            )
    for props in items.values():
        for k in props:
            props[k] = sorted(set(props[k]))
    path = os.path.join(SOURCES, "wikidata_codes.json")
    atomic(path,json_bytes(items))
    manifest["files"]["wikidata_codes.json"] = {
        "url": SPARQL,
        "queries": queries,
        "retrieved": now(),
        "sha256": sha256(path),
        "items": len(items),
    }


def wiktionary_qids() -> list[str]:
    qids: set[str] = set()
    for path in (LANGUAGES_JSON, FAMILIES_JSON):
        with open(path, encoding="utf-8") as f:
            for entry in json.load(f).values():
                q = entry.get("wikidataItem")
                if q:
                    qids.add(q)
    return sorted(qids, key=lambda q: int(q[1:]))


def fetch_wikidata_items(manifest: dict) -> None:
    """Per-item snapshot of every QID referenced from Wiktionary data."""
    qids = wiktionary_qids()
    print(f"wikidata targeted lookup for {len(qids)} items in batches of {BATCH}")
    props = " ".join(f"wdt:{p}" for p in WD_PROPS)
    items: dict[str, dict] = {q: {} for q in qids}
    n_batches = 0
    for i in range(0, len(qids), BATCH):
        batch = qids[i : i + BATCH]
        values = " ".join(f"wd:{q}" for q in batch)
        q = f"""
SELECT ?item ?p ?v ?vLabel WHERE {{
  VALUES ?item {{ {values} }}
  {{
    VALUES ?p {{ {props} }}
    ?item ?p ?v .
    OPTIONAL {{ ?v rdfs:label ?vLabel FILTER(LANG(?vLabel) = "en") }}
  }} UNION {{
    BIND(rdfs:label AS ?p)
    ?item rdfs:label ?v FILTER(LANG(?v) = "en")
  }} UNION {{
    BIND(owl:sameAs AS ?p)
    ?item owl:sameAs ?v .
  }}
}}"""
        rows = sparql(q)
        n_batches += 1
        for r in rows:
            item = qid_of(r["item"]["value"])
            p = r["p"]["value"].rsplit("/", 1)[-1].rsplit("#", 1)[-1]
            v = r["v"]["value"]
            if p == "label":
                items[item]["label"] = v
            elif p == "sameAs":
                items[item]["redirects_to"] = qid_of(v)
            else:
                key = WD_PROPS[p]
                if r["v"]["type"] == "uri":
                    v = qid_of(v)
                    if "vLabel" in r:
                        items[item].setdefault(key + "_labels", {})[v] = r["vLabel"]["value"]
                items[item].setdefault(key, []).append(v)
        print(f"  batch {n_batches}: {len(batch)} items, {len(rows)} rows")
        time.sleep(0.5)
    # Second pass: resolve redirects so the snapshot carries the target's data.
    redirected = {q: d["redirects_to"] for q, d in items.items() if "redirects_to" in d}
    if redirected:
        print(f"  resolving {len(redirected)} redirected items")
        targets = sorted(set(redirected.values()) - set(items))
        # (targets are few; reuse the same batch machinery by recursion-free loop)
        for i in range(0, len(targets), BATCH):
            batch = targets[i : i + BATCH]
            values = " ".join(f"wd:{q}" for q in batch)
            q = f"""
SELECT ?item ?p ?v WHERE {{
  VALUES ?item {{ {values} }}
  {{ VALUES ?p {{ {props} }} ?item ?p ?v . }}
  UNION {{ BIND(rdfs:label AS ?p) ?item rdfs:label ?v FILTER(LANG(?v) = "en") }}
}}"""
            for r in sparql(q):
                item = qid_of(r["item"]["value"])
                p = r["p"]["value"].rsplit("/", 1)[-1].rsplit("#", 1)[-1]
                v = r["v"]["value"]
                d = items.setdefault(item, {})
                if p == "label":
                    d["label"] = v
                else:
                    if r["v"]["type"] == "uri":
                        v = qid_of(v)
                    d.setdefault(WD_PROPS[p], []).append(v)
    for d in items.values():
        for k, v in list(d.items()):
            if isinstance(v, list):
                d[k] = sorted(set(v))
    path = os.path.join(SOURCES, "wikidata_items.json")
    atomic(path,json_bytes(items))
    manifest["files"]["wikidata_items.json"] = {
        "url": SPARQL,
        "properties": WD_PROPS,
        "queried_items": len(qids),
        "batches": n_batches,
        "redirected_items": len(redirected),
        "retrieved": now(),
        "sha256": sha256(path),
        "inputs": {
            "languages.json": sha256(LANGUAGES_JSON),
            "families.json": sha256(FAMILIES_JSON),
        },
    }


def fetch_statements(manifest: dict) -> None:
    """Retain entire entity claims: rank, snaks, precision, qualifiers, references.

    This optional snapshot supplements legacy truthy observations; it never
    silently upgrades their semantic assessment or replaces old assertions.
    """
    entities={};responses=[];ids=wiktionary_qids()
    for i in range(0,len(ids),50):
        params={"action":"wbgetentities","ids":"|".join(ids[i:i+50]),"props":"claims|info","redirects":"yes","format":"json"}
        url="https://www.wikidata.org/w/api.php?"+urllib.parse.urlencode(params)
        response=json.loads(http_get(url))
        if "error" in response or "entities" not in response:raise ValueError("incomplete entity response")
        responses.append(response)  # retain redirects and missing entities too
        entities.update(response["entities"])
    path=Path(SOURCES)/"wikidata_statements.json"
    atomic(path,json_bytes({"responses":responses,"entities":entities}))
    manifest["files"][path.name]={"url":"https://www.wikidata.org/w/api.php", "retrieved":now(),"sha256":sha256(path),"requested_items":len(ids),"inputs":{"languages.json":sha256(LANGUAGES_JSON),"families.json":sha256(FAMILIES_JSON)},"role":"statement provenance only; no automatic adjudication"}


def main() -> None:
    global SOURCES
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--skip-iso",action="store_true")
    ap.add_argument("--skip-wikidata",action="store_true")
    ap.add_argument("--statement-details",action="store_true",help="Also retrieve complete Wikidata statements, including ranks, qualifiers and references")
    args=ap.parse_args()
    destination=Path(SOURCES);destination.mkdir(parents=True,exist_ok=True)
    with (Path(HERE)/".source.lock").open("a") as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        manifest_path=destination/"manifest.json"
        manifest=json.loads(manifest_path.read_text()) if manifest_path.exists() else {"files":{}}
        # Keep the complete previous source set. Network failure never destroys it.
        if manifest_path.exists():
            archive=DATA/"archive/catalog-source-history"/sha256(manifest_path)
            if not archive.exists():shutil.copytree(destination,archive)
        with tempfile.TemporaryDirectory(dir=HERE,prefix=".fetch-") as tmp:
            shutil.copytree(destination,tmp,dirs_exist_ok=True);SOURCES=tmp
            if not args.skip_iso:fetch_iso(manifest)
            if not args.skip_wikidata:fetch_wikidata_global(manifest);fetch_wikidata_items(manifest)
            if args.statement_details:fetch_statements(manifest)
            manifest["last_run"]=now()
            # Publish data first, manifest last. Builders verify under the same lock.
            for name in manifest["files"]:atomic(destination/name,(Path(tmp)/name).read_bytes())
            atomic(manifest_path,json_bytes(manifest))
        SOURCES=str(destination)
        from workflow_tools.artifacts import adopt
        adopt("catalog-source-snapshot", destination, "source", {"original_receipt":"files/manifest.json", "acquisition":"individual original responses retained with acquisition receipts"})
    print("wrote "+str(manifest_path)+"; rebuild catalog registry before consuming changed sources")


if __name__ == "__main__":main()
