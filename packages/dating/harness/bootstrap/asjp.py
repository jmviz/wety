"""ASJP dates for language groups (Holman et al. 2011, Current Anthropology
52(6): 841–875, tables 3–7), a calibrated lexicostatistic date for ~400
families and subgroups, in years BP.

The tables use Ethnologue's (2009) classification names, and the PDF text loses
the indentation that shows nesting, so a row is placed on a Glottolog node
only when its name matches exactly one family-level languoid (primary or
alternative Glottolog name) and is not a generic relative name ("Eastern",
"Northern", ...). Everything else is recorded with `languoid: null` so the
table is preserved and can be mapped by hand."""

from __future__ import annotations

import csv
import glob
import os
import re
import unicodedata
from collections import defaultdict

from .. import glotto
from ..config import CACHE_DIR, USER_AGENT, now
from workflow_tools.paths import selected
from ..schema import Claim, Source, TimeValue
from ..time_utils import from_bp
from . import claim_id, write_claims

PDF_URL = "https://pure.mpg.de/rest/items/item_2395214_2/component/file_2432001/content"
PDF = os.path.join(CACHE_DIR, "holman2011.pdf")
PRESENT = 2000
GENERIC = {"eastern", "western", "northern", "southern", "central", "east", "west", "north", "south", "south central",
           "north central", "northwest", "northeast", "southwest", "southeast", "nuclear", "proper", "common", "insular",
           "plains", "coast", "interior", "highland", "lowland", "northwestern", "southwestern", "northeastern", "southeastern"}

SOURCE = Source(
    id="src_holman2011_asjp",
    type="paper", title="Automated Dating of the World's Language Families Based on Lexical Similarity",
    authors=["Holman, Eric W.", "Brown, Cecil H.", "Wichmann, Søren", "et al."], year=2011,
    doi="10.1086/662127", url=PDF_URL,
    citation="Holman, E. W., Brown, C. H., Wichmann, S., et al. 2011. Automated Dating of the World's Language Families Based on Lexical Similarity. Current Anthropology 52(6): 841–875.",
)


def norm(s: str) -> str:
    s = unicodedata.normalize("NFKD", s)
    s = "".join(ch for ch in s if not unicodedata.combining(ch)).casefold()
    s = re.sub(r"\s+languages?$", "", s)
    return re.sub(r"[^a-z0-9]+", " ", s).strip()


def family_name_index(g: glotto.Glottolog) -> dict[str, set[str]]:
    idx: dict[str, set[str]] = defaultdict(set)
    for gid, r in g.rows.items():
        if r["level"] == "family":
            idx[norm(r["name"])].add(gid)
    cldf = [str(selected('glottolog_cldf')/'names.csv')]
    if cldf:
        with open(cldf[-1], encoding="utf-8", newline="") as f:
            for r in csv.DictReader(f):
                gid = r["Language_ID"]
                if gid in g.rows and g.rows[gid]["level"] == "family" and (not r["lang"] or r["lang"] == "en"):
                    idx[norm(r["Name"])].add(gid)
    return idx


def parse_tables(pdf_path: str) -> list[dict]:
    import pymupdf
    doc = pymupdf.open(pdf_path)
    rows = []
    for pno in range(doc.page_count):
        text = doc[pno].get_text()
        if "dates for language groups" not in text:
            continue
        lines = [ln.strip() for ln in text.split("\n")]
        # a row is: name, pairs, subgroups "n (m)", similarity, date — one per line
        i = 0
        while i < len(lines) - 4:
            name, pairs, sub, sim, date = lines[i : i + 5]
            if (re.fullmatch(r"[\d,]+|\. \. \.", pairs) and re.fullmatch(r"\d+ \(\d+\)( \d+)?|\d+|\. \. \.", sub)
                    and re.fullmatch(r"-?\.?\d*\.?\d+a?|\. \. \.", sim) and re.fullmatch(r"[\d,]+|\. \. \.", date)
                    and re.fullmatch(r"[A-Z][A-Za-z’'\-–. ()]+", name)):
                if date != ". . .":
                    rows.append({"page": pno + 1, "group": name, "pairs": pairs, "subgroups": sub, "similarity": sim,
                                 "date_bp": int(date.replace(",", ""))})
                i += 5
            else:
                i += 1
    return rows


def run() -> dict:
    g = glotto.load()
    os.makedirs(CACHE_DIR, exist_ok=True)
    if not os.path.exists(PDF):
        import urllib.request
        req = urllib.request.Request(PDF_URL, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(req, timeout=120) as r, open(PDF, "wb") as f:
            f.write(r.read())
    rows = parse_tables(PDF)
    idx = family_name_index(g)
    claims: list[Claim] = []
    mapped = 0
    used: dict[str, int] = defaultdict(int)
    for n, row in enumerate(rows):
        name = row["group"]
        key = norm(name)
        gid = None
        if key not in GENERIC:
            cands = idx.get(key, set())
            if len(cands) == 1:
                gid = next(iter(cands))
        if gid:
            used[gid] += 1
            if used[gid] > 1:
                gid = None  # same Glottolog name matched twice (Ethnologue nesting we cannot see): leave unplaced
        if gid:
            mapped += 1
        claims.append(Claim(
            id=claim_id(gid, SOURCE.id, n, bucket="unpl0000"), languoid=gid, languoid_name=g.rows[gid]["name"] if gid else None,
            kind="lexicostatistic",
            evidence={"source_values":{"form":"point","value":row["date_bp"],"unit":"BP","present":None}, "role":"hypothesis", "event":"crown_divergence", "subject_type":"sampled_clade", "subject_id":"asjp:group:"+name},
            time=TimeValue(unknown=True, conversion=f"Source-native estimate {row['date_bp']} years BP; epoch not independently verified"),
            source=SOURCE.id, locator=f"p. {row['page']}, table row '{name}'",
            quote=f"{name}  {row['pairs']}  {row['subgroups']}  {row['similarity']}  {row['date_bp']}", quote_verified=True,
            extracted_by={"program": "harness.bootstrap.asjp", "run": "bootstrap"},
            notes=f"ASJP date for Ethnologue group '{name}'" + ("" if gid else "; not placed on a Glottolog node (ambiguous or generic name)"),
        ))
    SOURCE.retrieved = now()
    s = write_claims(g, SOURCE, claims)
    s.update({"rows": len(rows), "placed": mapped})
    return s
