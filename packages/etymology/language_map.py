"""A source's language map: each of its languages against Wiktionary's codes.

The relation runs from the source's language to the Wiktionary code:

- exact: the same variety; only this lets a record join an item of that code;
- subset: the source's variety is narrower (a dialect of the code's language);
- superset: the source's variety is broader;
- overlap: some shared scope, neither containing the other;
- unresolved: no code, or several candidates with nothing to choose between them.

The map is derived from the current catalog bundle (`packages/catalogs`), whose
Glottolog and ISO links are catalog lookups, not reviewed equivalences: an exact row
says that one code answers the lookup, and its `basis` says which. Rows a curator has
reviewed replace the derived ones and say so.
"""

import argparse
import csv
import json
from pathlib import Path

from catalogs import registry
from workflow_tools.paths import CATALOG_PRODUCT

RELATIONS = ("exact", "subset", "superset", "overlap", "unresolved")
LOOKUP = {"catalog_correspondence", "equivalent"}
SCOPE = {"source_narrower": "superset", "source_broader": "subset", "overlap": "overlap"}


class Catalog:
    """The lookups a language map needs from one catalog bundle."""

    def __init__(self, bundle):
        self.generation = registry.digest(bundle)
        self.by_target = bundle["index_by_target"]
        self.languages = bundle["languages"]
        self.glottolog = {g["glottocode"]: g for g in bundle["glottolog"]}
        self.proto_by_glottocode = {}
        for family in bundle["families"].values():
            proto = family.get("proto_language")
            if family.get("glottocode") and proto in self.languages:
                self.proto_by_glottocode.setdefault(family["glottocode"], set()).add(proto)

    def codes(self, target):
        """Wiktionary language codes that a target answers, by relation."""
        found = {}
        for link in self.by_target.get(target, []):
            source = link["source"]
            if not source.startswith("wikt:lang:") or link["status"] == "rejected":
                continue
            code = source.removeprefix("wikt:lang:")
            if link["relation"] in LOOKUP and link["status"] in (
                "catalog_link",
                "reviewed",
            ):
                found.setdefault("exact", set()).add(code)
            elif link["relation"] in SCOPE and link["status"] == "reviewed":
                found.setdefault(SCOPE[link["relation"]], set()).add(code)
            elif link["relation"] == "unresolved" and link["status"] == "reviewed":
                found.setdefault("unresolved", set()).add(code)
        return found


def row(language, relation, code=None, candidates=(), basis=""):
    return {
        "id": language["ID"],
        "name": language["Name"],
        "glottocode": language.get("Glottocode") or None,
        "iso639_3": language.get("ISO639P3code") or None,
        "is_proto": language.get("Is_Proto") == "true",
        "relation": relation,
        "code": code,
        "candidates": sorted(candidates),
        "basis": basis,
    }


def from_lookup(language, found, route):
    """A row from one route's lookup, or None when the route found nothing."""
    if found.get("unresolved"):
        return row(language, "unresolved", candidates=found["unresolved"], basis=route)
    exact = found.get("exact", set())
    if len(exact) == 1:
        return row(language, "exact", next(iter(exact)), basis=route)
    if len(exact) > 1:
        return row(language, "unresolved", candidates=exact, basis=route)
    for relation in ("subset", "superset", "overlap"):
        codes = found.get(relation, set())
        if len(codes) == 1:
            return row(language, relation, next(iter(codes)), basis=route)
        if codes:
            return row(language, "unresolved", candidates=codes, basis=route)
    return None


def map_language(language, catalog, by_id):
    """Map one CLDF LanguageTable row."""
    glottocode = language.get("Glottocode")
    if language.get("Is_Proto") == "true":
        protos = catalog.proto_by_glottocode.get(glottocode, set())
        if len(protos) == 1:
            return row(
                language,
                "exact",
                next(iter(protos)),
                basis="proto-language of the Wiktionary family with this glottocode",
            )
        return row(
            language,
            "unresolved",
            candidates=protos,
            basis="no single Wiktionary family has this glottocode",
        )
    if glottocode:
        mapped = from_lookup(language, catalog.codes("glottolog:" + glottocode), "glottocode")
        if mapped:
            return mapped
    if language.get("ISO639P3code"):
        mapped = from_lookup(
            language,
            catalog.codes("iso639-3:" + language["ISO639P3code"]),
            "ISO 639-3 code",
        )
        if mapped:
            return mapped
    parent = None
    if language.get("Dialect_Of") in by_id:
        parent = by_id[language["Dialect_Of"]].get("Glottocode")
    elif glottocode in catalog.glottolog:
        parent = catalog.glottolog[glottocode].get("parent_language")
    if parent and parent != glottocode:
        found = catalog.codes("glottolog:" + parent).get("exact", set())
        if len(found) == 1:
            return row(
                language,
                "subset",
                next(iter(found)),
                basis="dialect of " + parent + ", which the glottocode lookup maps",
            )
    return row(language, "unresolved", basis="no Wiktionary code found")


def build(languages, catalog, reviewed=()):
    """The language map: a row per language, reviewed rows replacing derived ones."""
    by_id = {language["ID"]: language for language in languages}
    decided = {decision["id"]: decision for decision in reviewed}
    unknown = set(decided) - set(by_id)
    if unknown:
        raise ValueError("reviewed rows name unknown languages: " + ", ".join(unknown))
    rows = []
    for language in languages:
        derived = map_language(language, catalog, by_id)
        decision = decided.get(language["ID"])
        if decision:
            if decision["relation"] not in RELATIONS:
                raise ValueError("unknown relation " + decision["relation"])
            if (decision["relation"] == "unresolved") != (decision.get("code") is None):
                raise ValueError("only an unresolved row has no code: " + language["ID"])
            if decision.get("code") and decision["code"] not in catalog.languages:
                raise ValueError("not a Wiktionary language code: " + decision["code"])
            derived = {
                **derived,
                "relation": decision["relation"],
                "code": decision.get("code"),
                "candidates": sorted(decision.get("candidates", [])),
                "basis": decision["rationale"],
                "reviewer": decision["reviewer"],
                "derived": {k: derived[k] for k in ("relation", "code", "candidates", "basis")},
            }
        rows.append(derived)
    return rows


def summary(rows):
    """Counts by relation, varieties and proto-languages apart."""
    counts = {"varieties": {}, "proto-languages": {}}
    for r in rows:
        group = counts["proto-languages" if r["is_proto"] else "varieties"]
        group[r["relation"]] = group.get(r["relation"], 0) + 1
    for group in counts.values():
        total = sum(group.values())
        group["total"] = total
        group["exact_share"] = round(group.get("exact", 0) / total, 4) if total else 0
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("languages", type=Path, help="a CLDF LanguageTable (CSV)")
    parser.add_argument("output", type=Path, help="the language map (JSON)")
    parser.add_argument("--reviewed", type=Path, help="reviewed rows (JSON list)")
    parser.add_argument("--catalog", type=Path, default=CATALOG_PRODUCT)
    args = parser.parse_args()
    with args.languages.open(newline="") as f:
        languages = list(csv.DictReader(f))
    reviewed = json.loads(args.reviewed.read_text()) if args.reviewed else []
    catalog = Catalog(registry.current_bundle(args.catalog))
    rows = build(languages, catalog, reviewed)
    result = {
        "catalog_generation": catalog.generation,
        "languages": {"path": str(args.languages), "sha256": registry.sha(args.languages)},
        "reviewed": (
            {"path": str(args.reviewed), "sha256": registry.sha(args.reviewed)}
            if args.reviewed
            else None
        ),
        "summary": summary(rows),
        "rows": rows,
    }
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n")
    print(json.dumps(result["summary"]))


if __name__ == "__main__":
    main()
