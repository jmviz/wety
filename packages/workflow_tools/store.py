"""The private research store: versioned snapshots of data/ and merging between checkouts.

Git holds code, docs and the few files the app is built from; everything else under
data/ lives here. Snapshots use restic (deduplicated, encrypted, any backend: Cloudflare
R2, another S3 service, a local disk). A commit names its data with the pointer file
data/SNAPSHOT.json, written by `snapshot`.

    uv run --frozen wety-data store status
    uv run --frozen wety-data store snapshot [--data DIR] [--note TEXT]
    uv run --frozen wety-data store restore --target DIR [--snapshot ID]
    uv run --frozen wety-data store sync SRC DST [--apply]
    uv run --frozen wety-data store check

Configuration is restic's own environment: RESTIC_REPOSITORY, RESTIC_PASSWORD_FILE and,
for S3, AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY (docs/architecture/provenance.md).
When RESTIC_REPOSITORY is unset, they are read from ~/.config/wety/restic.env (or the
file WETY_RESTIC_ENV names), so non-interactive shells need no profile.
Snapshots are taken of the main checkout's data/ (worktrees link parts of it); bring a
worktree's new records there first with `sync WORKTREE/data MAIN/data --apply`.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

from .paths import REPO

POINTER = "data/SNAPSHOT.json"
SETTINGS = Path("~/.config/wety/restic.env")
# What a settings file may set: restic's own variables and its S3 credentials.
SETTING_PREFIXES = ("RESTIC_", "AWS_")
EXCLUDES = ["*.lock", "__pycache__", ".DS_Store"]
# Paths whose files are written once and never edited: a difference is corruption or a
# collision, not a newer version.
IMMUTABLE = (
    "artifacts/",
    "workspaces/dating/_ledger/",
    "workspaces/dating/_evidence/review_",
    "workspaces/dating/_research/packets/",
    "registry/acquisitions/",
)


def main_checkout(repo=REPO) -> Path:
    """The main checkout (worktrees share its git directory)."""
    common = subprocess.run(
        ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    return Path(common).parent


def load_settings():
    """Source the settings file into this process unless RESTIC_REPOSITORY is set.
    Returns the file read, or None. A shell reads it, so `export A=1 B=$HOME/x` works."""
    if os.environ.get("RESTIC_REPOSITORY"):
        return None
    path = Path(os.environ.get("WETY_RESTIC_ENV") or SETTINGS).expanduser()
    if not path.is_file():
        return None
    out = subprocess.run(
        ["sh", "-c", 'set -a; . "$1" >/dev/null; env -0', "sh", str(path)],
        capture_output=True,
        text=True,
        check=False,
    )
    if out.returncode:
        raise SystemExit(f"could not read {path}: {out.stderr.strip()[-300:]}")
    for entry in out.stdout.split("\0"):
        key, sep, value = entry.partition("=")
        if sep and key.startswith(SETTING_PREFIXES):
            os.environ[key] = value
    return str(path)


def restic(*args, capture=True, cwd=None):
    if not shutil.which("restic"):
        raise SystemExit("restic is not installed (macOS: brew install restic)")
    load_settings()
    if not os.environ.get("RESTIC_REPOSITORY"):
        raise SystemExit(
            f"set RESTIC_REPOSITORY or write {SETTINGS} (see docs/architecture/provenance.md)"
        )
    out = subprocess.run(["restic", *args], capture_output=capture, text=True, cwd=cwd)
    if out.returncode:
        raise SystemExit(f"restic {args[0]} failed: {(out.stderr or '').strip()[-500:]}")
    return out.stdout


def git_head(repo=REPO):
    out = subprocess.run(["git", "rev-parse", "HEAD"], cwd=repo, capture_output=True, text=True)
    return out.stdout.strip() or None


def snapshot(data=None, note=None, repo=REPO) -> dict:
    data = Path(data or main_checkout(repo) / "data").resolve()
    links = [p.name for p in data.iterdir() if p.is_symlink()]
    if links:
        raise SystemExit(
            f"{data} links {', '.join(links)} elsewhere; snapshot the main checkout's data/ instead"
        )
    # Backed up by its relative name from its parent, so the snapshot holds /data/...
    # rather than this machine's directories, and restores anywhere. The host label
    # is fixed for the same reason.
    args = ["backup", "--json", "--tag", "wety", "--host", "wety", data.name]
    for pattern in EXCLUDES:
        args += ["--exclude", pattern]
    if note:
        args += ["--tag", "note:" + note[:60]]
    summary = next(
        json.loads(line)
        for line in reversed(restic(*args, cwd=data.parent).splitlines())
        if line.strip().startswith("{") and json.loads(line).get("message_type") == "summary"
    )
    pointer = {
        "schema_version": 1,
        "snapshot": summary["snapshot_id"],
        "taken_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "root": "/" + data.name,
        "files": summary.get("total_files_processed"),
        "bytes": summary.get("total_bytes_processed"),
        "git_commit": git_head(repo),
        "note": note,
    }
    (Path(repo) / POINTER).write_text(json.dumps(pointer, indent=2) + "\n")
    return pointer


def read_pointer(repo=REPO) -> dict:
    p = Path(repo) / POINTER
    if not p.exists():
        raise SystemExit(f"no {POINTER}; take a snapshot first")
    return json.loads(p.read_text())


def restore(target, snapshot_id=None, repo=REPO) -> dict:
    target = Path(target)
    if target.exists() and any(target.iterdir()):
        raise SystemExit(f"{target} is not empty; restore into an empty directory, then sync")
    pointer = read_pointer(repo)
    sid = snapshot_id or pointer["snapshot"]
    restic("restore", f"{sid}:{pointer['root']}", "--target", str(target), capture=False)
    return {"restored": sid, "target": str(target)}


def check(repo=REPO) -> dict:
    pointer = read_pointer(repo)
    found = json.loads(restic("snapshots", "--json", pointer["snapshot"]))
    if not found:
        raise SystemExit(
            f"snapshot {pointer['snapshot']} is not in {os.environ['RESTIC_REPOSITORY']}"
        )
    return {"snapshot": pointer["snapshot"], "time": found[0]["time"], "host": found[0]["hostname"]}


def status(repo=REPO) -> dict:
    p = Path(repo) / POINTER
    settings = load_settings()
    return {
        "restic": shutil.which("restic"),
        "settings": settings or ("environment" if os.environ.get("RESTIC_REPOSITORY") else None),
        "repository": os.environ.get("RESTIC_REPOSITORY"),
        "password": bool(
            os.environ.get("RESTIC_PASSWORD_FILE") or os.environ.get("RESTIC_PASSWORD")
        ),
        "pointer": json.loads(p.read_text()) if p.exists() else None,
        "main_data": str(main_checkout(repo) / "data"),
    }


def _sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _tracked(checkout) -> set[str]:
    """Paths under data/ that git tracks in that checkout (git merges those)."""
    out = subprocess.run(
        ["git", "ls-files", "-z", "data"], cwd=checkout, capture_output=True, text=True
    ).stdout
    return {p.removeprefix("data/") for p in out.split("\0") if p}


def _copy(src, dst):
    dst.parent.mkdir(parents=True, exist_ok=True)
    # A copy-on-write clone where the filesystem supports it (APFS), else a copy.
    if sys.platform == "darwin" and subprocess.run(["cp", "-c", "-p", src, dst]).returncode == 0:
        return
    shutil.copy2(src, dst)


def sync(src, dst, apply=False) -> dict:
    """Bring src's files into dst: missing files are copied; a file present in both with
    different content is a conflict and is never overwritten. Files git tracks in dst's
    checkout, and directories src links from elsewhere, are skipped."""
    src, dst = Path(src).resolve(), Path(dst).resolve()
    tracked = _tracked(dst.parent) if (dst.parent / ".git").exists() else set()
    copied, conflicts, same = [], [], 0
    for root, dirs, files in os.walk(src):
        here = Path(root)
        dirs[:] = [d for d in dirs if not (here / d).is_symlink()]
        for name in files:
            s = here / name
            if s.is_symlink() or name in (".DS_Store",) or name.endswith(".lock"):
                continue
            rel = s.relative_to(src).as_posix()
            if rel in tracked:
                continue
            d = dst / rel
            if not d.exists():
                copied.append(rel)
                if apply:
                    _copy(s, d)
            elif s.stat().st_size == d.stat().st_size and _sha(s) == _sha(d):
                same += 1
            else:
                conflicts.append(
                    {
                        "path": rel,
                        "kind": "immutable" if rel.startswith(IMMUTABLE) else "mutable",
                        "newer": "src" if s.stat().st_mtime > d.stat().st_mtime else "dst",
                    }
                )
    return {
        "applied": apply,
        "copied" if apply else "to_copy": len(copied),
        "identical": same,
        "conflicts": conflicts,
        "sample": copied[:20],
    }
