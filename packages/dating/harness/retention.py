"""Retention for published dating generations (`data/artifacts/dating-generations`).

Keep the current generation, the most recently published others (default two), and
any generation a tracked file names by its digest (a packet's baseline, a run
README). Migration inventories do not count as references. Everything else is
reported for removal. Nothing is deleted without `apply`; a removal log is kept in the
dating history.
"""

import json
import re
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from workflow_tools.paths import DATING_GENERATIONS, DATING_HISTORY, DATING_PRODUCT, REPO

DIGEST = re.compile(rb"[0-9a-f]{64}")
# Migration inventories list every file that moved, not what depends on it.
NOT_REFERENCES = ("data/registry/migrations/",)


def size(path):
    return sum(p.stat().st_size for p in Path(path).rglob("*") if p.is_file())


def tracked_references(generations, repo=REPO, ignore=NOT_REFERENCES):
    """{generation: [tracked files naming it]}; reads every tracked file once."""
    files = subprocess.run(
        ["git", "ls-files", "-z"], cwd=repo, capture_output=True, check=True
    ).stdout.split(b"\0")
    wanted = set(generations)
    refs = {}
    for name in filter(None, files):
        if name.decode().startswith(ignore):
            continue
        path = Path(repo) / name.decode()
        if not path.is_file():
            continue
        for match in set(DIGEST.findall(path.read_bytes())):
            digest = match.decode()
            if digest in wanted:
                refs.setdefault(digest, []).append(name.decode())
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
    refs = tracked_references(names, repo)
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
