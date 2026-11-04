"""Converters: each source's own format to assertions (`etymology.assertions`).

Each converter reads the source's retained files and its curation files in
`data/workspaces/etymology/sources/<source>/`, and converts the records a caller selects.
"""

import json

from etymology.assertions import REPO

SOURCES = REPO / "data/workspaces/etymology/sources"


def profile(name):
    return json.loads((SOURCES / name / "etc/source.json").read_text())


def source(name):
    """The source object for a source profile, `<name>@<release>`."""
    p = profile(name)
    s = {
        "type": "source",
        "id": f"{name}@{p['release']}",
        "title": p["title"],
        "licence": p["licence"],
        "may_display": p["may_display"],
        "may_export": p["may_export"],
        "profile": f"data/workspaces/etymology/sources/{name}/etc/source.json",
    }
    for key in ("doi", "url", "published", "uniform"):
        if p.get(key):
            s[key] = p[key]
    return s


class Languages:
    """A source's language map: its labels to Wiktionary codes, with the relation."""

    def __init__(self, name):
        data = json.loads((SOURCES / name / "etc/languages.json").read_text())
        self.rows = {r["id"]: r for r in data["rows"]}

    def __call__(self, key, label=None):
        r = self.rows.get(key)
        if r is None:
            return {"label": label or key, "code": None, "relation": "unresolved"}
        code = r["code"] if r["relation"] != "unresolved" else None
        return {"label": label or r["name"], "code": code, "relation": r["relation"]}


def agent(id, kind, name, **fields):
    return {"type": "agent", "id": id, "kind": kind, "name": name, **fields}


def work(id, citation, **fields):
    return {"type": "work", "id": id, "citation": citation, **fields}
