"""Paths and constants shared by the harness."""

from __future__ import annotations

import datetime as dt
import os

from workflow_tools.paths import (REPO, DATING_WORKSPACE, DATING_PRODUCT, DATING_GENERATIONS,
    DATING_HISTORY, RETRIEVED, CATALOG_PRODUCT, CATALOG_SOURCES, IECOR_PRODUCT, selected, historical)
DATING_DIR = str(REPO / 'packages/dating')
REPO_DIR = str(REPO)
DATA_DIR = str(DATING_WORKSPACE)
SCHEMA_DIR = os.path.join(DATING_DIR, 'schemas')
# Historical name retained in the domain API; these are durable research inputs, not disposable.
CACHE_DIR = str(RETRIEVED)
LANGCODES_DIR = str(CATALOG_PRODUCT)
IECOR_DIR = str(IECOR_PRODUCT)

USER_AGENT = "wety-dating/0.2 (https://github.com/jmviz/wety)"

# Calendar convention: astronomical year numbering. Year 0 exists and is 1 BCE;
# 500 BCE is -499. Every year in the data is an int in this convention.
YEAR_CONVENTION = "astronomical"

# BP epochs must be source-specific; there is no default.

MODEL = "claude-opus-5"
SCREEN_MODEL = "claude-haiku-4-5"


def load_env() -> None:
    """Load KEY=VALUE lines from the repo-root .env into os.environ (without
    overriding variables already set). Values are never logged or printed."""
    path = os.path.join(REPO_DIR, ".env")
    if not os.path.exists(path):
        return
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            k, v = line.split("=", 1)
            k, v = k.strip(), v.strip().strip('"').strip("'")
            if k and v and k not in os.environ:
                os.environ[k] = v


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def run_id() -> str:
    return "run_" + dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")


def family_dir(glottocode: str) -> str:
    d = os.path.join(DATA_DIR, glottocode)
    os.makedirs(d, exist_ok=True)
    return d
