"""Batch briefs for research runs, generated from the prompt templates.

spec: {"name": run name, "family": batch title, "template": "period" | "proto",
"sources": the holdings or works to read first (period), "codes": [[code, hint or
leads], ...]}. The brief is written to `_research/briefs/<name>.md`, the path the run's
README links to; `frontier_research prepare --brief` reads it.
"""

from __future__ import annotations

import json
from pathlib import Path

from workflow_tools.paths import DATING_PRODUCT, REPO

from .config import DATA_DIR

PROMPTS = Path(__file__).resolve().parents[1] / "prompts"
ASJP = ("Holman", "Rama")


def _wrap(text, width=86):
    out, line = [], ""
    for w in text.split():
        if len(line) + len(w) + 1 > width:
            out.append(line)
            line = "  " + w
        else:
            line = (line + " " + w) if line else w
    out.append(line)
    return "\n".join(out)


def _body(template):
    text = (PROMPTS / template).read_text()
    return text[text.index("```text") + len("```text\n") : text.rindex("```")]


def _context():
    inputs = json.loads((REPO / "data/registry/inputs.json").read_text())
    langs = json.loads((REPO / inputs["wiktionary_languages"]).read_text())
    product = Path(DATING_PRODUCT)
    targets = {
        t["code"]: t for t in json.loads((product / "period_targets.json").read_text())["targets"]
    }
    periods = {
        r["code"]: r
        for r in json.loads((product / "language_periods.json").read_text())["languages"]
    }
    return langs, targets, periods


def period_note(code, hint, langs, targets, periods):
    x = langs[code]
    head = f"{code} — {x['canonicalName']} ({x['kind']}"
    if x["kind"] == "etymology-only":
        p = x["nonEtymologyOnly"]
        head += f", a variety filed under {p} {langs[p]['canonicalName']}"
    parts = [head + ")."]
    varieties = [k for k, y in langs.items() if y.get("nonEtymologyOnly") == code and k != code]
    if varieties:
        parts.append(
            "Varieties with their own codes: "
            + ", ".join(f"{k} {langs[k]['canonicalName']}" for k in varieties)
            + "."
        )
    if items := targets.get(code, {}).get("items"):
        parts.append(f"Tree items: {items}.")
    p = periods.get(code)
    parts.append(
        f"Now: {p['label']} ({p['basis']}, {p['evidence'][0]['source']})." if p else "Now: none."
    )
    parts.append(hint)
    return _wrap(" ".join(parts))


def proto_line(code, leads, langs, targets, periods):
    p = periods.get(code)
    asjp = p and any(k in p["evidence"][0]["source"] for k in ASJP)
    line = f"{code} {langs[code]['canonicalName']} ({targets.get(code, {}).get('items', 0)})"
    line += "; ASJP only" if asjp else ""
    return line + (f". Leads: {leads}" if leads else "")


def recorded_works():
    """Short names of the works behind every recorded crown age: the exclusion list."""
    from .periods import short_source
    from .registry import load_evidence
    from .store import load_sources

    sources = load_sources()
    names = {
        short_source(sources[c["source_id"]])
        for c in load_evidence()
        if c["event"] == "crown_divergence" and c["source_id"] in sources
    }
    return "; ".join(sorted(n for n in names if n))


def make_brief(spec) -> Path:
    langs, targets, periods = _context()
    rank = lambda c: -(targets.get(c[0], {}).get("items") or 0)
    codes = sorted(spec["codes"], key=rank)
    if spec.get("template", "period") == "proto":
        template = "proto-estimates.md"
        text = (
            _body(template)
            .replace("{{IMPORTED}}", recorded_works())
            .replace(
                "{{CODES}}", "\n".join(proto_line(c, h, langs, targets, periods) for c, h in codes)
            )
        )
    else:
        template = "period-verification.md"
        text = (
            _body(template)
            .replace("{{SOURCES}}", spec["sources"])
            .replace(
                "{{CODES}}", "\n".join(period_note(c, h, langs, targets, periods) for c, h in codes)
            )
        )
    head = (
        f"# Batch brief: {spec['family']}\n\nGenerated from `packages/dating/prompts/{template}` "
        f"for run `{spec['name']}`;\ncodes ranked by `dating period-targets`.\n\n```text\n"
    )
    out = Path(DATA_DIR) / "_research/briefs" / f"{spec['name']}.md"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(head + text + "```\n")
    return out
