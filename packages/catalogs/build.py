#!/usr/bin/env python3
"""
Build the language-code cross-reference between

    English Wiktionary language codes   (selected Wiktionary export artifact)
    ISO 639-1 / 639-2 / 639-3 / 639-5  (sources/iso-639-3.tab etc.)
    Glottolog glottocodes               (a local glottolog-cldf release)
    Wikidata items                      (sources/wikidata_*.json)

See README.md for the matching strategy, the output files and their fields.

Usage:
    python build.py [--glottolog-cldf DIR]
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import glob
import hashlib
import io
import json
import os
import re
import unicodedata
from collections import Counter, defaultdict

from workflow_tools.paths import (CATALOG_PRODUCT, CATALOG_WORKSPACE, CATALOG_SOURCES, selected)
HERE = str(CATALOG_PRODUCT)
SOURCES = str(CATALOG_SOURCES)
LANGUAGES_JSON = str(selected('wiktionary_languages'))
FAMILIES_JSON = str(selected('wiktionary_families'))
OVERRIDES_JSON = str(CATALOG_WORKSPACE / 'overrides.json')

WD_LANG_TYPES = {
    "Q436240": "ancient",
    "Q2315359": "historical",
    "Q45762": "dead",
    "Q38058796": "extinct",
    "Q206577": "proto",
    "Q137753162": "reconstructed",
    "Q33215": "constructed",
    "Q1288568": "modern",
    "Q33384": "dialect",
    "Q25295": "family",
    "Q34228": "sign",
    "Q33289": "creole",
    "Q33831": "pidgin",
    "Q33694": "mixed",
    "Q152559": "macrolanguage",
}

ISO_TYPE = {"L": "living", "E": "extinct", "H": "historical", "A": "ancient", "C": "constructed", "S": "special"}
ISO_SCOPE = {"I": "individual", "M": "macrolanguage", "S": "special"}

CODE2 = re.compile(r"[a-z]{2}")
CODE3 = re.compile(r"[a-z]{3}")


# --------------------------------------------------------------------------- #
# loading
# --------------------------------------------------------------------------- #

def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_tsv(path: str) -> list[dict]:
    with open(path, encoding="utf-8-sig", newline="") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def norm_name(s: str) -> str:
    s = unicodedata.normalize("NFKD", s)
    s = "".join(ch for ch in s if not unicodedata.combining(ch))
    s = s.casefold().strip()
    s = re.sub(r"\s+languages?$", "", s)
    s = re.sub(r"[\s\-‐-―_/]+", " ", s)
    s = re.sub(r"[^\w\s']", "", s)
    return s.strip()


def load_iso() -> dict:
    iso3 = {r["Id"]: r for r in read_tsv(os.path.join(SOURCES, "iso-639-3.tab"))}
    iso1_to_3 = {r["Part1"]: r["Id"] for r in iso3.values() if r["Part1"]}
    retired: dict[str, dict] = {}
    for r in read_tsv(os.path.join(SOURCES, "iso-639-3_Retirements.tab")):
        if r["Id"] not in retired or r["Effective"] > retired[r["Id"]]["Effective"]:
            retired[r["Id"]] = r
    macro_members: dict[str, list[str]] = defaultdict(list)
    member_of: dict[str, str] = {}
    for r in read_tsv(os.path.join(SOURCES, "iso-639-3-macrolanguages.tab")):
        if r["I_Status"] == "A":
            macro_members[r["M_Id"]].append(r["I_Id"])
            member_of[r["I_Id"]] = r["M_Id"]
    # 639-2 file: bib|term|639-1|English|French  (term empty when identical to bib)
    iso2: dict[str, dict] = {}
    with open(os.path.join(SOURCES, "ISO-639-2_utf-8.txt"), encoding="utf-8-sig") as f:
        for line in f:
            parts = line.rstrip("\n").split("|")
            if len(parts) < 5:
                continue
            bib, term, p1, en, fr = parts[:5]
            term = term or bib
            iso2[term] = {"bib": bib, "term": term, "iso639_1": p1, "name": en}
    iso5 = {r["code"]: r["Label (English)"] for r in read_tsv(os.path.join(SOURCES, "iso639-5.tsv"))}
    return {
        "iso3": iso3, "iso1_to_3": iso1_to_3, "retired": retired,
        "macro_members": dict(macro_members), "member_of": member_of,
        "iso2": iso2, "iso5": iso5,
    }


def find_glottolog_cldf() -> str:
    path = selected('glottolog_cldf')
    if not (path / 'cldf-metadata.json').is_file():
        raise ValueError('selected Glottolog release is missing; restore the pinned artifact')
    return str(path)


def validate_classification(langs: dict) -> None:
    """Reject missing nodes, cycles and incompatible ancestry-path prefixes."""
    for gid,r in langs.items():
        path=(r.get('classification') or '').split('/') if r.get('classification') else []
        if len(set(path+[gid]))!=len(path)+1 or any(x not in langs for x in path):
            raise ValueError('invalid Glottolog classification path '+gid)
        for i,ancestor in enumerate(path):
            if (langs[ancestor].get('classification') or '')!='/'.join(path[:i]):
                raise ValueError('inconsistent Glottolog classification prefix '+gid)


def load_glottolog(cldf_dir: str) -> dict:
    with open(os.path.join(cldf_dir, "languages.csv"), encoding="utf-8", newline="") as f:
        rows=list(csv.DictReader(f))
        langs = {r["ID"]: r for r in rows}
        if len(langs)!=len(rows):raise ValueError("duplicate Glottolog ID")
    with open(os.path.join(cldf_dir, "values.csv"), encoding="utf-8", newline="") as f:
        for r in csv.DictReader(f):
            p = r["Parameter_ID"]
            if p in ("category", "classification", "aes", "med") and r["Language_ID"] in langs:
                langs[r["Language_ID"]][p] = r["Value"]
    by_iso=defaultdict(list)
    for gid,r in langs.items():
        if r['ISO639P3code']:by_iso[r['ISO639P3code']].append(gid)
    validate_classification(langs)
    if any(len(v)>1 for v in by_iso.values()):raise ValueError('ambiguous Glottolog ISO assignment; needs multivalued source review')
    iso_to_glotto={k:v[0] for k,v in by_iso.items()}
    # name indexes (Glottolog name + alternative names), split by level
    names_fam: dict[str, set[str]] = defaultdict(set)
    names_lang: dict[str, set[str]] = defaultdict(set)
    def add(gid: str, name: str) -> None:
        n = norm_name(name)
        if not n or n == "not specified":
            return
        (names_fam if langs[gid]["Level"] == "family" else names_lang)[n].add(gid)
    for gid, r in langs.items():
        add(gid, r["Name"])
    # alternative names are only used for families: for languages they are too
    # loose (regional and cover names) and produce false unique matches
    with open(os.path.join(cldf_dir, "names.csv"), encoding="utf-8", newline="") as f:
        for r in csv.DictReader(f):
            gid = r["Language_ID"]
            if gid in langs and langs[gid]["Level"] == "family" and (not r["lang"] or r["lang"] == "en"):
                add(gid, r["Name"])
    version = None
    meta_path = os.path.join(cldf_dir, "..", "metadata.json")
    if os.path.exists(meta_path):
        with open(meta_path, encoding="utf-8") as f:
            version = json.load(f).get("title")
    return {
        "langs": langs, "iso_to_glotto": iso_to_glotto, "names_fam": names_fam, "names_lang": names_lang,
        "version": version, "dir": cldf_dir,
    }


def glotto_path(gl: dict, gid: str) -> list[str]:
    r = gl["langs"].get(gid)
    if not r:
        return []
    cls = r.get("classification") or ""
    return (cls.split("/") if cls else []) + [gid]


def glotto_relation(gl: dict, a: str, b: str) -> str:
    """How does languoid b relate to languoid a?"""
    if a == b:
        return "same"
    pa, pb = glotto_path(gl, a), glotto_path(gl, b)
    if b in pa[:-1]:
        return "ancestor"
    if a in pb[:-1]:
        return "descendant"
    if pa and pb and pa[:-1] and pa[:-1] == pb[:-1]:
        return "sibling"
    return "unrelated"


def load_wikidata() -> dict:
    with open(os.path.join(SOURCES, "wikidata_items.json"), encoding="utf-8") as f:
        items = json.load(f)
    with open(os.path.join(SOURCES, "wikidata_codes.json"), encoding="utf-8") as f:
        codes = json.load(f)
    glotto_to_q: dict[str, list[str]] = defaultdict(list)
    for q, d in codes.items():
        for g in d.get("glottocode", []):
            glotto_to_q[g].append(q)
    return {"items": items, "codes": codes, "glotto_to_q": dict(glotto_to_q)}


def load_wiktionary() -> tuple[dict, dict]:
    with open(LANGUAGES_JSON, encoding="utf-8") as f:
        raw = json.load(f)
    langs: dict[str, dict] = {}
    aliases: dict[str, list[str]] = defaultdict(list)
    for key, entry in raw.items():
        mc = entry["mainCode"]
        if key != mc:
            aliases[mc].append(key)
        if mc not in raw:raise ValueError("Wiktionary alias lacks main entry: "+key)
        if entry!=raw[mc]:raise ValueError("Wiktionary alias has conflicting payload: "+key)
        if mc not in langs or key == mc:
            langs[mc] = entry
    for mc, al in aliases.items():
        langs[mc]["_aliasCodes"] = sorted(al)
    with open(FAMILIES_JSON, encoding="utf-8") as f:
        fams = json.load(f)
    return langs, fams


# --------------------------------------------------------------------------- #
# helpers
# --------------------------------------------------------------------------- #

def wd_item(wd: dict, qid: str | None) -> tuple[str | None, dict]:
    """Resolve redirects; return (resolved qid, item data)."""
    if not qid:
        return None, {}
    seen=set()
    while True:
        if qid in seen:raise ValueError('Wikidata redirect cycle '+qid)
        seen.add(qid);d=wd['items'].get(qid,{})
        if 'redirects_to' not in d:return qid,d
        target=d['redirects_to']
        if target not in wd['items']:return target,{}
        qid=target



def wd_types(d: dict) -> list[str]:
    return sorted({WD_LANG_TYPES[q] for q in d.get("instance_of", []) if q in WD_LANG_TYPES})


def iso3_info(iso: dict, code: str | None) -> dict:
    if not code:
        return {}
    r = iso["iso3"].get(code)
    out: dict = {"iso639_3": code}
    if r:
        out.update({
            "iso639_3_name": r["Ref_Name"],
            "iso639_3_scope": ISO_SCOPE.get(r["Scope"], r["Scope"]),
            "iso639_3_type": ISO_TYPE.get(r["Language_Type"], r["Language_Type"]),
            "iso639_1": r["Part1"] or None,
            "iso639_2b": r["Part2b"] or None,
            "iso639_2t": r["Part2t"] or None,
            "iso639_3_status": "active",
        })
        if code in iso["macro_members"]:
            out["iso639_3_members"] = iso["macro_members"][code]
        if code in iso["member_of"]:
            out["iso639_3_macrolanguage"] = iso["member_of"][code]
    elif code in iso["retired"]:
        rt = iso["retired"][code]
        out.update({
            "iso639_3_name": rt["Ref_Name"],
            "iso639_3_status": "retired",
            "iso639_3_retirement": {
                "reason": rt["Ret_Reason"], "change_to": rt["Change_To"] or None,
                "remedy": rt["Ret_Remedy"] or None, "effective": rt["Effective"],
            },
        })
    else:
        out["iso639_3_status"] = "unknown"
    return out


def glottolog_info(gl: dict, gid: str | None) -> dict:
    if not gid:
        return {}
    r = gl["langs"].get(gid)
    if not r:
        return {"glottocode": gid, "glottolog_status": "unknown"}
    return {
        "glottocode": gid,
        "glottolog_name": r["Name"],
        "glottolog_level": r["Level"],
        "glottolog_category": r.get("category"),
        "glottolog_family_id": r["Family_ID"] or None,
        "glottolog_parent_language": r["Language_ID"] or None,
        "glottolog_classification": r.get("classification") or None,
        "glottolog_iso639_3": r["ISO639P3code"] or None,
        "glottolog_closest_iso639_3": r["Closest_ISO369P3code"] or None,
        "glottolog_first_year": int(r["First_Year_Of_Documentation"]) if r["First_Year_Of_Documentation"] else None,
        "glottolog_last_year": int(r["Last_Year_Of_Documentation"]) if r["Last_Year_Of_Documentation"] else None,
        "glottolog_aes": r.get("aes"),
    }


def name_candidates(index: dict, names: list[str]) -> set[str]:
    out: set[str] = set()
    for n in names:
        if n:
            out |= index.get(norm_name(n), set())
    return out


def iso_relation(iso: dict, a: str, b: str) -> str:
    if a == b:
        return "same"
    if iso["member_of"].get(b) == a:
        return "member"       # b is a member of macrolanguage a
    if iso["member_of"].get(a) == b:
        return "macrolanguage"  # b is the macrolanguage containing a
    ra, rb = iso["retired"].get(a), iso["retired"].get(b)
    if ra and ra["Change_To"] == b:
        return "successor"    # a was retired in favour of b
    if rb and rb["Change_To"] == a:
        return "predecessor"  # b was retired in favour of a
    return "unrelated"


# --------------------------------------------------------------------------- #
# families
# --------------------------------------------------------------------------- #

def match_family(code: str, fam: dict, iso: dict, gl: dict, wd: dict) -> dict:
    rec: dict = {
        "code": code,
        "name": fam["canonicalName"],
        "proto_language": fam.get("protoLanguage"),
        "wiktionary_superfamilies": fam.get("superfamilies", []),
        "wikidata": fam.get("wikidataItem"),
        "conflicts": [], "notes": [], "alternatives": {},
    }
    qid, wdd = wd_item(wd, fam.get("wikidataItem"))
    if qid and qid != fam.get("wikidataItem"):
        rec["wikidata_redirected_to"] = qid
    if wdd:
        rec["wikidata_label"] = wdd.get("label")
        rec["wikidata_types"] = wd_types(wdd)

    # ISO 639-5 (and the identical 639-2 collective code)
    iso5 = code if code in iso["iso5"] else None
    iso5_route = "code" if iso5 else None
    wd5 = wdd.get("iso639_5", [])
    if iso5 and wd5 and iso5 not in wd5:
        rec["conflicts"].append(f"iso639_5: code says {iso5}, wikidata says {','.join(wd5)}")
        rec["alternatives"]["iso639_5_wikidata"] = wd5
    elif iso5 and wd5:
        iso5_route = "code+wikidata"
    elif wd5:
        iso5, iso5_route = wd5[0], "wikidata"
    rec["iso639_5"] = iso5
    rec["iso639_5_name"] = iso["iso5"].get(iso5) if iso5 else None
    rec["iso639_5_route"] = iso5_route
    if iso5 and iso5 in iso["iso2"]:
        rec["iso639_2t"] = iso["iso2"][iso5]["term"]
        rec["iso639_2b"] = iso["iso2"][iso5]["bib"]

    # Glottolog family node: Wikidata P1394, else guarded name match
    wdg = [g for g in wdd.get("glottocode", []) if g in gl["langs"]]
    glotto, route = None, None
    if wdg:
        fam_level = [g for g in wdg if gl["langs"][g]["Level"] == "family"]
        glotto = (fam_level or wdg)[0]
        route = "wikidata"
        if len(wdg) > 1:
            rec["alternatives"]["glottocode_wikidata"] = wdg
            rec["conflicts"].append(f"glottocode: wikidata item has several: {','.join(wdg)}")
    else:
        cands = name_candidates(gl["names_fam"], [fam["canonicalName"], *fam.get("otherNames", [])])
        if len(cands) == 1:
            glotto, route = next(iter(cands)), "name"
        elif len(cands) > 1:
            rec["alternatives"]["glottocode_name_candidates"] = sorted(cands)
    rec.update(glottolog_info(gl, glotto))
    rec["glottocode_route"] = route
    if glotto and rec.get("glottolog_level") != "family":
        rec["notes"].append(f"glottocode {glotto} is {rec.get('glottolog_level')}-level, not a family")
    return rec


def proto_subclass_nodes(wlangs: dict, gl: dict, wd: dict) -> dict[str, list[str]]:
    """Proto-language code -> Glottolog family nodes reachable through its
    Wikidata item's subclass_of (Proto-Germanic subclass_of Germanic -> germ1287)."""
    out: dict[str, list[str]] = {}
    for code, e in wlangs.items():
        if e["kind"] != "reconstructed":
            continue
        _, wdd = wd_item(wd, e.get("wikidataItem"))
        nodes = set()
        for q in wdd.get("subclass_of", []):
            for g in wd["codes"].get(q, {}).get("glottocode", []):
                if g in gl["langs"] and gl["langs"][g]["Level"] == "family":
                    nodes.add(g)
        if nodes:
            out[code] = sorted(nodes)
    return out


def resolve_iso3_from_code(code: str, iso: dict) -> tuple[str | None, str | None]:
    """Route A: the Wiktionary code itself. Returns (iso3, route)."""
    if CODE2.fullmatch(code):
        return (iso["iso1_to_3"].get(code), "iso1") if code in iso["iso1_to_3"] else (None, None)
    if CODE3.fullmatch(code) and (code in iso["iso3"] or code in iso["retired"]):
        return code, "code"
    return None, None


def match_language(code: str, entry: dict, wlangs: dict, iso: dict, gl: dict, wd: dict, shared_q: dict, families: dict) -> dict:
    kind = entry["kind"]
    rec: dict = {
        "code": code,
        "name": entry["canonicalName"],
        "kind": kind,
        "wiktionary_family": entry.get("family"),
        "wiktionary_parents": entry.get("parents", []),
        "wiktionary_non_etymology_only": entry["nonEtymologyOnly"],
        "wiktionary_alias_codes": entry.get("_aliasCodes", []),
        "wikidata": entry.get("wikidataItem"),
        "conflicts": [], "notes": [], "alternatives": {},
    }
    qid, wdd = wd_item(wd, entry.get("wikidataItem"))
    if qid and qid != entry.get("wikidataItem"):
        rec["wikidata_redirected_to"] = qid
    if wdd:
        rec["wikidata_label"] = wdd.get("label")
        rec["wikidata_types"] = wd_types(wdd)
    conflicts, notes, alternatives = rec["conflicts"], rec["notes"], rec["alternatives"]

    # A Wiktionary language may share its Wikidata item with another one. If it
    # shares with its own non-etymology-only parent (or an ancestor), the item
    # describes the parent, and its codes must not be taken as this variety's.
    wd_is_parents = False
    if qid and len(shared_q.get(qid, [])) > 1:
        others = [c for c in shared_q[qid] if c != code]
        rec["wikidata_shared_with"] = others
        related = set(entry.get("ancestors", [])) | {entry["nonEtymologyOnly"]}
        my_name = entry["canonicalName"]
        wd_is_parents = any(
            o in related
            for o in others)
        if wd_is_parents:
            notes.append("wikidata item is the parent language's; its codes were not used")
        else:
            # the item is mine if every other sharer is my descendant: either by
            # Wiktionary's ancestry, or (Wiktionary often omits that for
            # etymology-only sub-varieties) by name: "Jinjiang Hokkien" / "Hokkien"
            def is_descendant(o: str) -> bool:
                oe = wlangs[o]
                return code in (set(oe.get("ancestors", [])) | {oe["nonEtymologyOnly"]})
            if not all(is_descendant(o) for o in others):
                rec["_wd_shared_unrelated"] = True

    # ---- ISO 639-3 -----------------------------------------------------------
    iso3_code, iso3_route = resolve_iso3_from_code(code, iso)
    wd_iso3 = [] if wd_is_parents else wdd.get("iso639_3", [])
    if iso3_code:
        if wd_iso3 and iso3_code in wd_iso3:
            iso3_route += "+wikidata"
        elif wd_iso3:
            rels = {c: iso_relation(iso, iso3_code, c) for c in wd_iso3}
            alternatives["iso639_3_wikidata"] = wd_iso3
            if all(r != "unrelated" for r in rels.values()):
                notes.append("iso639_3: wikidata has " + ", ".join(f"{c} ({r})" for c, r in rels.items()))
            else:
                conflicts.append(f"iso639_3: code says {iso3_code}, wikidata says {','.join(wd_iso3)}")
    elif wd_iso3:
        iso3_code, iso3_route = wd_iso3[0], "wikidata"
        if len(wd_iso3) > 1:
            alternatives["iso639_3_wikidata"] = wd_iso3
            conflicts.append(f"iso639_3: wikidata item has several: {','.join(wd_iso3)}")
    rec.update(iso3_info(iso, iso3_code))
    rec["iso639_3_route"] = iso3_route
    if CODE2.fullmatch(code) and not rec.get("iso639_1"):
        if code in iso["iso1_to_3"]:rec["iso639_1"] = code
        if not iso3_code:
            notes.append("2-letter code not in ISO 639-1")
    if not wd_is_parents:
        if not rec.get("iso639_1") and wdd.get("iso639_1"):
            rec["iso639_1"] = wdd["iso639_1"][0]
        if not rec.get("iso639_2t") and wdd.get("iso639_2"):
            rec["iso639_2t"] = wdd["iso639_2"][0]
        if wdd.get("iso639_5"):
            rec["iso639_5"] = wdd["iso639_5"][0]

    # ---- Glottocode ----------------------------------------------------------
    glotto_iso = None
    lookup = iso3_code
    if lookup:
        glotto_iso = gl["iso_to_glotto"].get(lookup)
    # Glottolog parks retired ISO codes in the "Bookkeeping" pseudo-family; such a
    # languoid is a placeholder, not a match, whenever anything better exists
    bookkeeping_iso = None
    if glotto_iso and gl["langs"][glotto_iso].get("category") == "Bookkeeping":
        bookkeeping_iso, glotto_iso = glotto_iso, None
    wd_glotto_all = [] if wd_is_parents else wdd.get("glottocode", [])
    wd_glotto = [g for g in wd_glotto_all if g in gl["langs"]]
    if len(wd_glotto) < len(wd_glotto_all):
        alternatives["glottocode_wikidata_not_in_glottolog"] = [g for g in wd_glotto_all if g not in gl["langs"]]

    glotto, route = None, None
    iso_from_code = iso3_route is not None and not iso3_route.startswith("wikidata")
    if glotto_iso and wd_glotto:
        if glotto_iso in wd_glotto:
            # Keep route dependencies explicit; agreement is not independence.
            glotto, route = glotto_iso, "iso+wikidata" if iso_from_code else "wikidata"
        else:
            glotto, route = glotto_iso, "iso"
            alternatives["glottocode_wikidata"] = wd_glotto
            rels = {g: glotto_relation(gl, glotto_iso, g) for g in wd_glotto}
            if all(r != "unrelated" for r in rels.values()):
                route = "iso~wikidata"
                notes.append("glottocode: wikidata has " + ", ".join(
                    f"{g} ({gl['langs'][g]['Level']}, {r} of {glotto_iso})" for g, r in rels.items()))
            else:
                conflicts.append(f"glottocode: iso says {glotto_iso}, wikidata says {','.join(wd_glotto)}")
        if len(wd_glotto) > 1:
            alternatives["glottocode_wikidata"] = wd_glotto
    elif glotto_iso:
        glotto, route = glotto_iso, "iso"
    elif wd_glotto:
        glotto, route = wd_glotto[0], "wikidata"
        if len(wd_glotto) > 1:
            alternatives["glottocode_wikidata"] = wd_glotto
            conflicts.append(f"glottocode: wikidata item has several: {','.join(wd_glotto)}")
    elif bookkeeping_iso:
        glotto, route = bookkeeping_iso, "iso"
        notes.append("iso639_3 maps only to a Glottolog bookkeeping (retired-code) languoid")
        rec["_weak"] = True
    else:
        # Name equality proposes candidates; catalog classification cannot veto them.
        if kind in ("regular", "etymology-only"):
            cands = name_candidates(gl["names_lang"], [entry["canonicalName"], *entry.get("aliases", [])])
            alternatives["glottocode_name_candidates"] = sorted(cands)
            if len(cands) == 1:
                glotto, route = next(iter(cands)), "name"
    if bookkeeping_iso and glotto and glotto != bookkeeping_iso:
        notes.append(f"iso639_3 maps to Glottolog bookkeeping languoid {bookkeeping_iso}; wikidata's node used instead")
        alternatives["glottocode_iso_bookkeeping"] = bookkeeping_iso
    if route == "wikidata" and rec.get("_wd_shared_unrelated"):
        notes.append("glottocode from a wikidata item shared with unrelated " + ",".join(rec["wikidata_shared_with"]) + "; may belong to them")
        rec["_weak"] = True
    rec.update(glottolog_info(gl, glotto))
    rec["glottocode_route"] = route
    if glotto and not iso3_code and rec.get("glottolog_iso639_3"):
        # Glottolog carries an ISO code we did not have: record it, but it is
        # Glottolog's claim about the languoid, not Wiktionary's about the code
        notes.append(f"glottolog languoid carries iso639_3 {rec['glottolog_iso639_3']}")

    rec["_wd_is_parents"] = wd_is_parents
    return rec


def _main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--glottolog-cldf", default=None, help="path to an unpacked glottolog-cldf 'cldf' directory")
    args = ap.parse_args()

    iso = load_iso()
    gl = load_glottolog(args.glottolog_cldf or find_glottolog_cldf())
    wd = load_wikidata()
    wlangs, wfams = load_wiktionary()
    overrides = {}
    if os.path.exists(OVERRIDES_JSON):
        with open(OVERRIDES_JSON, encoding="utf-8") as f:
            overrides = json.load(f)

    shared_q: dict[str, list[str]] = defaultdict(list)
    for code, e in wlangs.items():
        q, _ = wd_item(wd, e.get("wikidataItem"))
        if q:
            shared_q[q].append(code)

    # Source families and languages remain separate concept schemes.
    families = {c: match_family(c, f, iso, gl, wd) for c, f in wfams.items()}
    # Heuristics generate proposals. P279 never supplies a family's identity,
    # and inheritance never copies a broader entity's codes or dates to a stage.
    proto_nodes=proto_subclass_nodes(wlangs,gl,wd)
    proto_to_families=defaultdict(list)
    for c,f in wfams.items():
        if f.get('protoLanguage'):proto_to_families[f['protoLanguage']].append(c)
    langs={c:match_language(c,e,wlangs,iso,gl,wd,shared_q,families) for c,e in wlangs.items()}
    for c,r in langs.items():
        r['_proto_families']=sorted(proto_to_families.get(c,[]))
        r['_proto_subclass_nodes']=proto_nodes.get(c,[])
    from .registry import source_snapshot,convert_record,apply_decisions,project,publish
    snapshot=source_snapshot(gl['dir'])
    languages={c:convert_record(r,wlangs[c],False,iso,gl,wd,snapshot['digest']) for c,r in sorted(langs.items())}
    families={c:convert_record(r,wfams[c],True,iso,gl,wd,snapshot['digest']) for c,r in sorted(families.items())}
    decisions=overrides.get('decisions',{})
    if set(overrides)-{'schema_version','decisions','notes'}:raise ValueError('legacy/unrestricted overrides must be migrated')
    apply_decisions({r['id']:r for r in [*languages.values(),*families.values()]},decisions,snapshot['digest'],gl,iso)
    langs={c:project(r) for c,r in languages.items()}
    families={c:project(r) for c,r in families.items()}
    # Incompatible source classifications remain explicit warnings, not grounds
    # to force one catalog into the other's tree.
    for r in families.values():
        for sf in r['wiktionary_superfamilies']:
            a,b=r.get('glottocode'),families.get(sf,{}).get('glottocode')
            if a and b and glotto_relation(gl,a,b) not in ('ancestor','same'):
                r['issues'].append('source classification disagreement with '+sf)
                r['review_state']='needs_review'

    # ---- glottolog-keyed table ------------------------------------------------
    glotto_to_wikt: dict[str, list[str]] = defaultdict(list)
    for c, rec in langs.items():
        if rec.get("glottocode"):
            glotto_to_wikt[rec["glottocode"]].append(c)
    glotto_to_wfam: dict[str, list[str]] = defaultdict(list)
    for c, rec in families.items():
        if rec.get("glottocode"):
            glotto_to_wfam[rec["glottocode"]].append(c)
    glotto_rows = []
    for gid, r in gl["langs"].items():
        glotto_rows.append({
            "glottocode": gid, "name": r["Name"], "level": r["Level"], "category": r.get("category"),
            "iso639_3": r["ISO639P3code"] or None, "closest_iso639_3": r["Closest_ISO369P3code"] or None,
            "parent_language": r["Language_ID"] or None, "family_id": r["Family_ID"] or None,
            "classification": r.get("classification") or None,
            "first_year": r["First_Year_Of_Documentation"] or None, "last_year": r["Last_Year_Of_Documentation"] or None,
            "aes": r.get("aes"), "med": r.get("med"),
            "wikidata": wd["glotto_to_q"].get(gid, []),
            "wiktionary_codes": sorted(glotto_to_wikt.get(gid, [])),
            "wiktionary_family_codes": sorted(glotto_to_wfam.get(gid, [])),
            "macroarea": r["Macroarea"] or None, "latitude": r["Latitude"] or None, "longitude": r["Longitude"] or None,
        })
    glotto_rows.sort(key=lambda r: r["glottocode"])

    bundle=publish(langs,families,glotto_rows,snapshot,decisions)
    print(f"Published {len(langs)} language codes, {len(families)} family codes; {len(bundle['review_queue'])} records require scope review.")


def main() -> None:
    import fcntl
    with open(CATALOG_WORKSPACE / '.source.lock', 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        _main()


if __name__ == '__main__':
    main()
