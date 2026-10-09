"""Append-only editorial ledger: decisions, withdrawals and registered sources.

Every write adds one immutable batch file under `_ledger/` (named by UTC time and
digest); no file is ever edited. An event puts or retracts the record under a key,
for example ("decision", "wikt:eya|first_attestation") or ("source", "src_x"), and
names the heads it replaces: the events current for that key when it was written.
The current state is every head not replaced by a later event.

Sessions in different checkouts merge by copying batch files. Two sessions that
write one key without seeing each other leave two heads with different records: a
conflict, which `current` reports and the registry withholds until a later event
replaces both. Identical records written twice are not a conflict.
"""

from __future__ import annotations

import json
import os
import socket
from datetime import datetime, timezone
from pathlib import Path

from .config import DATA_DIR
from .io_utils import atomic_json, digest

LEDGER_DIR = Path(DATA_DIR) / "_ledger"
SCHEMA_VERSION = 1
_cache: dict[str, list[dict]] = {}


def root(path=None) -> Path:
    return Path(path or LEDGER_DIR)


def session() -> str:
    """Who writes: WETY_SESSION if set, else host and checkout directory."""
    if os.environ.get("WETY_SESSION"):
        return os.environ["WETY_SESSION"]
    from workflow_tools.paths import REPO

    return f"{socket.gethostname()}:{Path(REPO).name}"


def batches(path=None) -> list[dict]:
    """Every batch, oldest first. Files are immutable, so each is parsed once."""
    out = []
    for p in sorted(root(path).glob("*.json")):
        key = str(p)
        if key not in _cache:
            _cache[key] = json.loads(p.read_text())
        out.append(_cache[key])
    return sorted(out, key=lambda b: (b["at"], b["id"]))


def events(path=None) -> list[dict]:
    return [e for b in batches(path) for e in b["events"]]


def heads(kind, path=None) -> dict[str, list[dict]]:
    """{key: [current events]}: more than one only after concurrent writes."""
    out: dict[str, list[dict]] = {}
    for e in events(path):
        if e["kind"] != kind:
            continue
        replaced = set(e["replaces"])
        kept = [h for h in out.get(e["key"], []) if h["id"] not in replaced]
        out[e["key"]] = kept + [e]
    return out


def current(kind, path=None):
    """(records, conflicts): records maps each key to its current record (retracted
    keys omitted); conflicts maps a key to its competing records."""
    records, conflicts = {}, {}
    for key, hs in heads(kind, path).items():
        distinct = {digest(h["record"]): h for h in hs}.values()
        if len(distinct) > 1:
            conflicts[key] = [h["record"] for h in distinct]
        elif (h := next(iter(distinct))) and h["op"] == "put":
            records[key] = h["record"]
    return records, conflicts


def append(changes, reason=None, path=None, actor=None) -> dict | None:
    """Write one batch. Each change is {kind, key, op: put|retract, record?, note?};
    it replaces the key's current heads (including earlier changes in this batch).
    Returns the batch, or None when nothing changed."""
    at = datetime.now(timezone.utc).isoformat(timespec="microseconds")
    live = {
        (k, key): list(hs)
        for k in {c["kind"] for c in changes}
        for key, hs in heads(k, path).items()
    }
    out = []
    for c in changes:
        if c["op"] not in ("put", "retract"):
            raise ValueError("ledger op must be put or retract")
        hs = live.get((c["kind"], c["key"]), [])
        if c["op"] == "retract" and not any(h["op"] == "put" for h in hs):
            raise ValueError(f"nothing to retract for {c['kind']} {c['key']}")
        record = c.get("record") if c["op"] == "put" else None
        if c["op"] == "put" and len(hs) == 1 and hs[0]["op"] == "put" and hs[0]["record"] == record:
            continue  # unchanged
        body = {
            "kind": c["kind"],
            "key": c["key"],
            "op": c["op"],
            "record": record,
            "note": c.get("note"),
            "replaces": sorted(h["id"] for h in hs),
            "at": at,
        }
        e = {"id": digest(body), **body}
        live[(c["kind"], c["key"])] = [e]
        out.append(e)
    if not out:
        return None
    batch = {
        "schema_version": SCHEMA_VERSION,
        "at": at,
        "session": session(),
        "actor": actor,
        "reason": reason,
        "events": out,
    }
    batch["id"] = digest(batch)
    stamp = at.replace("-", "").replace(":", "").replace("+0000", "Z").replace("+00:00", "Z")
    d = root(path)
    d.mkdir(parents=True, exist_ok=True)
    atomic_json(d / f"{stamp}-{batch['id'][:12]}.json", batch)
    return batch


def withdrawals(path=None) -> list[dict]:
    """Retracted decisions with what they retracted, oldest first."""
    by_id = {e["id"]: e for e in events(path)}
    out = []
    for e in events(path):
        if e["kind"] == "decision" and e["op"] == "retract":
            gone = [by_id[i]["record"] for i in e["replaces"] if by_id[i]["op"] == "put"]
            out.append(
                {
                    "decision": gone[0] if gone else None,
                    **(e["note"] or {}),
                    "withdrawn_at": e["at"],
                }
            )
    return out


def decision_key(d) -> str:
    return f"{d['subject_id']}|{d['event']}"


def migrate(data_dir=None, path=None) -> dict:
    """Move the legacy files (decisions.json, withdrawals.json, sources.json) into a
    genesis batch, unchanged, and rename them `*.migrated`. Refuses a non-empty ledger."""
    data = Path(data_dir or DATA_DIR)
    if events(path):
        raise ValueError("ledger already has events; nothing to migrate")
    files = {
        "decisions": data / "_decisions/decisions.json",
        "withdrawals": data / "_decisions/withdrawals.json",
        "sources": data / "sources/sources.json",
    }
    load = lambda p, empty: json.loads(p.read_text()) if p.exists() else empty
    decisions, withdrawn, sources = (
        load(files["decisions"], []),
        load(files["withdrawals"], []),
        load(files["sources"], {}),
    )
    changes = [
        {"kind": "source", "key": k, "op": "put", "record": v} for k, v in sorted(sources.items())
    ]
    # A withdrawn decision was adopted, then retracted with its reason, before the
    # current decision for its key (if any) was recorded.
    for w in sorted(withdrawn, key=lambda w: w["withdrawn_at"]):
        key = decision_key(w["decision"])
        note = {
            "reviewer": w["reviewer"],
            "rationale": w["rationale"],
            "legacy_withdrawn_at": w["withdrawn_at"],
        }
        changes += [
            {"kind": "decision", "key": key, "op": "put", "record": w["decision"]},
            {"kind": "decision", "key": key, "op": "retract", "note": note},
        ]
    changes += [
        {"kind": "decision", "key": decision_key(d), "op": "put", "record": d} for d in decisions
    ]
    batch = append(
        changes, reason="migration of decisions.json, withdrawals.json and sources.json", path=path
    )
    for p in files.values():
        if p.exists():
            p.rename(p.with_name(p.name + ".migrated"))
    return {
        "events": len(batch["events"]) if batch else 0,
        "sources": len(sources),
        "decisions": len(decisions),
    }
