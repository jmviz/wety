"""Retention for published dating generations (`data/artifacts/dating-generations`).

Keep the current generation, the most recently published others (default two), and
any generation a record names by its digest (a packet's baseline, a run README).
Records are the files git tracks and the research store's dating workspace
(`data/workspaces/dating`: packets, run directories, briefs, evidence, ledger, family
workspaces), which git no longer tracks. Each file is read once. Retrieved and retained
source text, caches and binary files are not scanned; migration inventories do not
count as references. Everything else is reported for removal. Nothing is deleted
without `apply`; a removal log is kept in the dating history.
"""

import json
import os
import re
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from workflow_tools.paths import DATING_GENERATIONS, DATING_HISTORY, DATING_PRODUCT, REPO

DIGEST = re.compile(rb"[0-9a-f]{64}")
# Hex bytes become x, others a space: a C-speed find then locates the long hex runs.
HEX_MASK = bytes(ord("x" if chr(c) in "0123456789abcdef" else " ") for c in range(256))
# Migration inventories list every file that moved, not what depends on it.
NOT_REFERENCES = ("data/registry/migrations/",)
# Research-store records outside git that can name a generation.
STORE = ("data/workspaces/dating",)
# Source text (the retrieved store, runs' retained `evidence/`), caches and binaries
# only hold what was read or computed; skipping them keeps the scan fast.
NOT_SCANNED_DIRS = {"retrieved", "evidence", "__pycache__", "cache"}
NOT_SCANNED_SUFFIXES = {".pdf", ".png", ".jpg", ".jpeg", ".xlsx", ".pyc", ".gz", ".zip"}


def size(path):
    return sum(p.stat().st_size for p in Path(path).rglob("*") if p.is_file())


def record_files(repo=REPO, store=STORE):
    """Repository-relative paths of tracked files, then research-store files; each once."""
    tracked = subprocess.run(
        ["git", "ls-files", "-z"], cwd=repo, capture_output=True, check=True
    ).stdout.split(b"\0")
    names = dict.fromkeys(n.decode() for n in tracked if n)
    for top in store:
        for folder, dirs, files in os.walk(Path(repo) / top):
            dirs[:] = sorted(d for d in dirs if d not in NOT_SCANNED_DIRS)
            for f in sorted(files):
                if Path(f).suffix.lower() not in NOT_SCANNED_SUFFIXES:
                    names.setdefault(Path(folder, f).relative_to(repo).as_posix(), None)
    return list(names)


def digests(data):
    """The digests DIGEST.findall finds in data; the regex only sees long hex runs."""
    mask, found = data.translate(HEX_MASK), set()
    start = mask.find(b"x" * 64)
    while start != -1:
        end = mask.find(b" ", start)
        end = len(data) if end == -1 else end
        found.update(m.decode() for m in DIGEST.findall(data, start, end))
        start = mask.find(b"x" * 64, end)
    return found


def references(generations, repo=REPO, ignore=NOT_REFERENCES, store=STORE):
    """{generation: [records naming it]}; reads every record file once."""
    wanted = set(generations)
    refs = {}
    for name in record_files(repo, store):
        if name.startswith(ignore):
            continue
        path = Path(repo) / name
        if not path.is_file():
            continue
        for digest in digests(path.read_bytes()) & wanted:
            refs.setdefault(digest, []).append(name)
    return refs


def plan(keep_recent=2, root=DATING_GENERATIONS, product=DATING_PRODUCT, repo=REPO):
    root = Path(root)
    current = json.loads((Path(product) / "CURRENT.json").read_text())["generation"]
    # Publication order: publish rewrites manifest.json, so its mtime is the latest
    # publication of that generation.
    published = sorted(
        (p.parent for p in root.glob("*/manifest.json")),
        key=lambda d: (d / "manifest.json").stat().st_mtime,
        reverse=True,
    )
    names = [d.name for d in published]
    if current not in names:
        raise ValueError("current generation is not in the generation store")
    recent = [n for n in names if n != current][:keep_recent]
    refs = references(names, repo)
    keep = {}
    for n in names:
        reasons = (
            (["current"] if n == current else [])
            + (["recent"] if n in recent else [])
            + [f"referenced by {f}" for f in refs.get(n, [])]
        )
        if reasons:
            keep[n] = reasons
    remove = [n for n in names if n not in keep]
    # Directories without a manifest are not generations; report, never touch.
    unknown = sorted(p.name for p in root.iterdir() if p.is_dir() and p.name not in names)
    return {
        "keep": keep,
        "remove": remove,
        "remove_bytes": sum(size(root / n) for n in remove),
        "keep_bytes": sum(size(root / n) for n in keep),
        "unknown_directories": unknown,
    }


def apply(result, root=DATING_GENERATIONS, history=DATING_HISTORY):
    from .io_utils import atomic_json

    root = Path(root)
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    removed = []
    for n in result["remove"]:
        manifest = json.loads((root / n / "manifest.json").read_text())
        shutil.rmtree(root / n)
        removed.append(manifest)
    atomic_json(Path(history) / "gc" / (stamp + ".json"), {"removed": removed, "plan": result})
    return {"removed": len(removed), "freed_bytes": result["remove_bytes"]}
