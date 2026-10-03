"""Read/write the on-disk artifact: sources.json, per-family claims.jsonl,
languoids.json, search_log.jsonl. Idempotent writes keyed by id."""

from __future__ import annotations
from workflow_tools.paths import DATING_PRODUCT, DATING_GENERATIONS, DATING_HISTORY

import json
import os
from collections import defaultdict

from .config import DATA_DIR, family_dir, now
from .schema import Claim, Languoid, Source
from .io_utils import atomic_json, atomic_text, locked, digest

SOURCES_PATH = os.path.join(DATA_DIR, "sources", "sources.json")


def _read_jsonl(path: str) -> list[dict]:
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def _write_jsonl(path: str, rows: list[dict]) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    atomic_text(
        path, "".join(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n" for r in rows)
    )


def load_sources() -> dict[str, dict]:
    if not os.path.exists(SOURCES_PATH):
        return {}
    with open(SOURCES_PATH, encoding="utf-8") as f:
        return json.load(f)


def save_sources(sources: dict[str, dict]) -> None:
    os.makedirs(os.path.dirname(SOURCES_PATH), exist_ok=True)
    atomic_json(SOURCES_PATH, sources)


def upsert_source(src: Source) -> None:
    with locked(SOURCES_PATH):
        sources = load_sources()
        sources[src.id] = {**sources.get(src.id, {}), **src.model_dump(exclude_none=True)}
        save_sources(sources)


def load_claims(family: str) -> list[dict]:
    return _read_jsonl(os.path.join(family_dir(family), "claims.jsonl"))


def upsert_claims(family: str, claims: list[Claim]) -> tuple[int, int]:
    with locked(os.path.join(family_dir(family), "claims.jsonl")):
        return _upsert_claims(family, claims)


def _upsert_claims(family: str, claims: list[Claim]) -> tuple[int, int]:
    """Insert or replace claims by id. Returns (added, replaced)."""
    existing = {c["id"]: c for c in load_claims(family)}
    added = replaced = 0
    for c in claims:
        d = c.model_dump(exclude_none=True)
        if c.id in existing:
            replaced += 1
        else:
            added += 1
        existing[c.id] = d
    _write_jsonl(
        os.path.join(family_dir(family), "claims.jsonl"), [existing[k] for k in sorted(existing)]
    )
    return added, replaced


def replace_extraction(family: str, source: str, claims: list[Claim], provenance: dict) -> None:
    """Publish one complete extraction; immutable snapshots retain superseded claims.

    Empty extractions also supersede old output. Tree/bootstrap evidence is independent.
    A crash after snapshot but before publication leaves the previous complete set active.
    """
    path = os.path.join(family_dir(family), "claims.jsonl")
    with locked(path):
        old = load_claims(family)
        previous = [
            c for c in old if c["source"] == source and c.get("extracted_by", {}).get("model")
        ]
        rows = [c.model_dump(exclude_none=True) for c in claims]
        snapshot = {
            "source": source,
            "previous": previous,
            "claims": rows,
            "provenance": provenance,
        }
        sid = digest(snapshot)
        atomic_json(os.path.join(DATING_HISTORY, "extractions", sid + ".json"), snapshot)
        keep = [
            c for c in old if not (c["source"] == source and c.get("extracted_by", {}).get("model"))
        ]
        _write_jsonl(path, sorted(keep + rows, key=lambda c: c["id"]))


def delete_claims(family: str, source_prefix: str) -> int:
    """Remove claims whose source id starts with the prefix (used to re-run a bootstrap)."""
    path = os.path.join(family_dir(family), "claims.jsonl")
    with locked(path):
        rows = load_claims(family)
        keep = [r for r in rows if not r["source"].startswith(source_prefix)]
        atomic_json(os.path.join(DATING_HISTORY, "bootstrap", digest(rows) + ".json"), rows)
        _write_jsonl(path, keep)
    return len(rows) - len(keep)


def load_languoids(family: str) -> dict[str, dict]:
    path = os.path.join(family_dir(family), "languoids.json")
    if not os.path.exists(path):
        return {}
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def save_languoids(family: str, records: dict[str, Languoid | dict]) -> None:
    path = os.path.join(family_dir(family), "languoids.json")
    out = {
        k: (v.model_dump(exclude_none=True) if isinstance(v, Languoid) else v)
        for k, v in sorted(records.items())
    }
    atomic_json(path, out)


def append_log(family: str, name: str, record: dict) -> None:
    record = {"at": now(), **record}
    path = os.path.join(family_dir(family), name)
    with locked(path):
        with open(path, "a", encoding="utf-8") as f:
            f.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")
            f.flush()
            os.fsync(f.fileno())


def families_on_disk() -> list[str]:
    return sorted(
        d
        for d in os.listdir(DATA_DIR)
        if os.path.isdir(os.path.join(DATA_DIR, d))
        and (
            __import__("re").fullmatch(r"[a-z]{4}[0-9]{4}", d)
            or d in ("isolates", "unclassified", "curated")
        )
    )


def all_claims() -> dict[str, list[dict]]:
    out = defaultdict(list)
    for fam in families_on_disk():
        for c in load_claims(fam):
            out[fam].append(c)
    return dict(out)


def replace_source_claims(family: str, source: str, claims: list[Claim]) -> None:
    """Crash-safe complete replacement of one bootstrap source in one family."""
    path = os.path.join(family_dir(family), "claims.jsonl")
    with locked(path):
        old = load_claims(family)
        rows = [c.model_dump(exclude_none=True) for c in claims]
        snapshot = {
            "source": source,
            "previous": [c for c in old if c["source"] == source],
            "claims": rows,
        }
        atomic_json(os.path.join(DATING_HISTORY, "bootstrap", digest(snapshot) + ".json"), snapshot)
        _write_jsonl(
            path, sorted([c for c in old if c["source"] != source] + rows, key=lambda c: c["id"])
        )
