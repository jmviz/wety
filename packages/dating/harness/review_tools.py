"""Reviewer tooling for research runs: judgments -> review, publication, run README,
assertion summaries and tree-clade adoptions.

The researcher's identity comes from the run's frozen brief (profile model, effort,
transport); the reviewer's from `WETY_REVIEWER` (for example "Claude Opus 5.5 (Claude
Code)"). Nothing here is hard-coded to one model or date.
"""

from __future__ import annotations

import json
import os
import re
from datetime import datetime, timezone
from pathlib import Path

from workflow_tools.paths import DATING_PRODUCT

from .config import DATA_DIR

TRANSPORT = {
    "codex": "Codex CLI on ChatGPT subscription",
    "claude-agent": "Claude Agent SDK on Claude subscription",
    "openrouter": "OpenRouter API",
}
MAPPING_NOTE = {
    "exact": "Exact for the code.",
    "subset": "Subset; context.",
    "superset": "Superset; context.",
    "overlap": "Overlap; context.",
    "unresolved": "Mapping unresolved; context.",
}


def reviewer_name(name=None) -> str:
    name = name or os.environ.get("WETY_REVIEWER")
    if not name or not name.strip():
        raise ValueError("name the reviewer: --reviewer or WETY_REVIEWER")
    return name.strip()


def researcher(run) -> dict:
    """Who researched: the profile frozen in the run's brief."""
    brief = json.loads((Path(run) / "brief.json").read_text())
    p = brief["profile"]
    usage = Path(run) / "usage.json"
    cli = json.loads(usage.read_text()).get("cli") if usage.exists() else None
    return {
        "model": p["model"],
        "effort": p["effort"],
        "profile": brief.get("profile_name"),
        "transport": TRANSPORT.get(p["backend"], p["backend"]),
        "cli": cli,
        "label": f"{p['model']} ({p['effort']} effort, {TRANSPORT.get(p['backend'], p['backend'])})",
        "research_seconds": brief.get("research_seconds"),
        "delivery_seconds": brief.get("delivery_seconds"),
    }


def run_started(run) -> str:
    r = json.loads((Path(run) / "run.json").read_text())
    started = r.get("attempt_started") or (r.get("attempts") or [{}])[0].get("started")
    return datetime.fromtimestamp(started, timezone.utc).isoformat(timespec="seconds")


def rows(run) -> list[dict]:
    return [
        json.loads(x) for x in (Path(run) / "assertions.jsonl").read_text().split("\n") if x.strip()
    ]


def _known_sources() -> dict:
    """Source records named in earlier reviews (reused so one work keeps one id)."""
    known = {}
    for f in sorted((Path(DATA_DIR) / "_research").glob("*/coordinator-review/*.json")):
        try:
            known.update(json.loads(f.read_text()).get("sources", {}))
        except (ValueError, OSError):
            pass
    return known


def _slug(s):
    return re.sub(r"[^a-z0-9]+", "_", s.lower()).strip("_")


def parse_citation(cit):
    """(authors, year) from a free-text citation: "Surname, Given & Given Surname. 2020."
    Retrieval dates ("accessed 2026-10-06") and "n.d." are not years."""
    head = re.split(r"accessed|retrieved", cit, flags=re.I)[0]
    undated = re.search(r"(?<!\w)(n\.\s?d\.|undated)(?!\w)", head, re.I)
    yr = None if undated else re.search(r"\b(1[5-9]|20)\d\d\b", head)
    names = head[: yr.start()] if yr else head.split(".")[0]
    authors = [a.strip(" ,.") for a in re.split(r"\s*(?:&|\band\b|;)\s*", names) if a.strip(" ,.")]
    # "Rau, Felix" -> "Felix Rau"; a list "Auderset, Sandra, Simon J. Greenhill" -> two names.
    out = []
    for a in authors:
        parts = [p.strip() for p in a.split(",") if p.strip()]
        if len(parts) == 2 and " " not in parts[0]:
            out.append(f"{parts[1]} {parts[0]}")
        elif len(parts) > 2 and " " not in parts[0]:
            out += [f"{parts[1]} {parts[0]}", *parts[2:]]
        else:
            out += parts
    return out, int(yr.group(0)) if yr else None


def review_from_judgments(run, judgments, reviewer=None) -> dict:
    """Write coordinator-review/review.json from a judgments file.

    judgments: {"scope", "adopt": {code: assertion | [assertion]}, "status": {a: status},
    "rationale": {a: text}, "correct": {a: {...}}, "corrections": {a: [text]},
    "mapping": {a: [mapping, basis]}, "taxa": {a: [taxa, basis]}, "adopt_rationale":
    {a: text}, "proto_mapping": {a: text}, "proto_alternatives": {a: [...]},
    "sources": {source_id: record}}. Statuses default to the researcher's proposal."""
    run, J = Path(run), judgments
    reviewer = reviewer_name(reviewer)
    who = researcher(run)
    name = run.name
    rs = rows(run)
    state = json.loads((run / "tools-state.json").read_text())["sources"]
    from workflow_tools.paths import REPO

    holdings = REPO / "data/registry/holdings.json"
    held = set(json.loads(holdings.read_text()).get("holdings", {})) if holdings.exists() else set()
    from .store import load_sources

    used = set(load_sources())
    known = _known_sources()
    sources = {}
    for r in rs:
        sid = r["source_id"]
        if sid in sources:
            continue
        if sid in J.get("sources", {}):
            sources[sid] = J["sources"][sid]
            continue
        if sid in known:
            sources[sid] = known[sid]
            continue
        cit = r.get("citation") or state.get(sid, {}).get("url", sid)
        authors, year = parse_citation(cit)
        lead = (re.match(r"([^,.(]+)", cit) or re.match(r"(.+)", cit)).group(1).strip()[:40]
        rid = f"src_{_slug(lead)[:20]}_{year or 'nd'}_{_slug(cit[len(lead) :])[:24]}".rstrip("_")
        while rid in used:
            rid += "_b"
        used.add(rid)
        sources[sid] = {
            "id": rid,
            "type": "handbook" if sid in held else "paper",
            "title": cit[:150],
            "authors": authors[:6] or [lead],
            **({"year": year} if year else {}),
            "citation": cit,
            "review_scope": "Passages cited by the claims only.",
        }
    adopted = {a for v in J.get("adopt", {}).values() for a in (v if isinstance(v, list) else [v])}
    claims = []
    for r in rs:
        a = r["id"]
        proposed = (
            r.get("proposed_status") or (r.get("assessment") or {}).get("status") or "candidate"
        )
        note = MAPPING_NOTE.get(r["subject"].get("mapping"), "Context.")
        c = {
            "assertion": a,
            "status": J.get("status", {}).get(a, proposed),
            "rationale": J.get("rationale", {}).get(
                a, note + (" Adopted." if a in adopted else "")
            ),
        }
        for k in ("correct", "corrections"):
            if a in J.get(k, {}):
                c[k] = J[k][a]
        # A quote found more than once in its text is located by the run's offset.
        txt = state.get(r["source_id"], {}).get("text_file")
        if txt and (run / "evidence" / txt).exists():
            body = (run / "evidence" / txt).read_text()
            if body.count(r["quote"]) > 1:
                loc = r.get("locator") if isinstance(r.get("locator"), dict) else {}
                starts = [loc.get(k) for k in ("start", "text_start", "character_start")]
                s = next(
                    (
                        x
                        for x in starts
                        if isinstance(x, int) and body[x : x + len(r["quote"])] == r["quote"]
                    ),
                    None,
                )
                c["start"] = body.index(r["quote"]) if s is None else s
        if a in J.get("taxa", {}):
            c["taxa"], c["taxa_basis"] = J["taxa"][a]
        if a in J.get("mapping", {}):
            c["mapping"], c["mapping_basis"] = J["mapping"][a]
        if "code" not in r and "code" in r["subject"] and "code" not in c.get("correct", {}):
            c.setdefault("correct", {})["code"] = r["subject"]["code"]
            c.setdefault("corrections", []).append(
                "representation: code copied from subject.code (reviewer)."
            )
        if c["status"] != proposed and not any("status" in x for x in c.get("corrections", [])):
            c.setdefault("corrections", []).append(
                f"status {proposed} -> {c['status']} (reviewer)."
            )
        claims.append(c)
    by = {r["id"]: r for r in rs}

    def choice(a):
        why = (
            J.get("adopt_rationale", {}).get(a)
            or (by[a].get("proposed_decision") or {}).get("rationale")
            or "Quote exact; checked."
        )
        out = {"assertion": a, "rationale": why}
        for k in ("proto_mapping", "proto_alternatives"):
            if a in J.get(k, {}):
                out[k] = J[k][a]
        return out

    review = {
        "packet_id": name,
        "claim_prefix": "ev_period_",
        "reviewer": f"{who['label']}, run {name}; reviewed by {reviewer}: quotes exact in retained texts. Model review only.",
        "scope": J.get("scope", name),
        "review_independence": f"Two model sessions ({who['model']}, {reviewer}): process separation, not independent corroboration.",
        "decision_reviewer": f"{reviewer}, reviewer of {who['model']}'s batch {name}",
        "retrieved": run_started(run),
        "sources": sources,
        "claims": claims,
        "adopt": {
            k: ([choice(x) for x in v] if isinstance(v, list) else choice(v))
            for k, v in J.get("adopt", {}).items()
        },
    }
    out = run / "coordinator-review/review.json"
    out.parent.mkdir(exist_ok=True)
    out.write_text(json.dumps(review, ensure_ascii=False, indent=1))
    return {"review": str(out), "claims": len(claims), "adopted_codes": len(review["adopt"])}


def publish(run, review_path=None) -> dict:
    """Packet, import, publish, adopt, publish, validate, export: one reviewed run."""
    from .periods import write_periods
    from .registry import publish as synthesize
    from .research_packet import adopt, build, write
    from .review import import_evidence
    from .validate import validate_all

    run = Path(run)
    review = json.loads(Path(review_path or run / "coordinator-review/review.json").read_text())
    packet = write(build(run, review))
    imported = import_evidence(Path(packet["packet"]) / "claims.json")
    synthesize()
    adopted = adopt(review)
    synthesize()
    errors = validate_all()
    if errors:
        raise ValueError("validation failed after adoption: " + "; ".join(errors[:5]))
    write_periods()
    product = Path(DATING_PRODUCT)
    rejected = json.loads((product / "rejected_decisions.json").read_text())
    periods = json.loads((product / "language_periods.json").read_text())["languages"]
    return {
        "packet": packet["packet"],
        "imported": imported["imported"],
        "adopted": adopted,
        "rejected": [r["decision"]["subject_id"] for r in rejected],
        "codes": len(periods),
    }


def run_readme(run, title, notes, reviewer=None) -> str:
    """README.md for a reviewed run: researcher, budget, outcome, then the notes."""
    from .frontier_tools import check

    run = Path(run)
    who = researcher(run)
    reviewer = reviewer_name(reviewer)
    r = json.loads((run / "run.json").read_text())
    used = r.get("active_seconds_used") or r.get("elapsed_seconds") or 0
    status = (r.get("attempts") or [{}])[-1].get("status") or r.get("status")
    turns = []
    if (run / "usage.json").exists():
        turns = json.loads((run / "usage.json").read_text()).get("turns") or []
    t = turns[0] if turns else None
    actions = (
        sum(1 for _ in open(run / "tool-events.jsonl"))
        if (run / "tool-events.jsonl").exists()
        else 0
    )
    rs = rows(run)
    errors = check(run).get("errors", [])
    review = json.loads((run / "coordinator-review/review.json").read_text())
    supported = sum(c["status"] == "supported" for c in review["claims"])
    requests = []
    if (run / "requests.jsonl").exists():
        requests = [
            json.loads(x).get("title")
            for x in (run / "requests.jsonl").read_text().split("\n")
            if x.strip()
        ]
    usage = (
        f"Usage {t['input_tokens']:,} input tokens ({t.get('cached_input_tokens', 0):,} cached), {t['output_tokens']:,} output."
        if t
        else "Usage not recorded."
    )
    cli = f", {who['cli']}" if who["cli"] else ""
    text = f"""# {title}

[Batch brief](../briefs/{run.name}.md).

## Run

- Profile `{who["profile"]}` ({who["model"]}, {who["effort"]} effort, {who["transport"]}{cli}).
  {who["research_seconds"]} s + {who["delivery_seconds"]} s; used {round(used)} s ({status}).
  {actions} tool actions. {usage}
  `check`: {len(rs)} assertions, {len(errors)} errors.{(" Requests: " + "; ".join(filter(None, requests)) + ".") if requests else ""}

## Review ({reviewer})

Quotes exact in the retained texts. {supported} of {len(rs)} supported; {len(review["adopt"])} codes adopted.

{notes.strip()}
"""
    (run / "README.md").write_text(text)
    return text


def summarize(run, pages=None) -> str:
    """One block per assertion for review; with `pages`, render the PDF pages that the
    proposed defaults cite into that directory."""
    from .evidence import normalize_time
    from .research_packet import registry_raw

    run = Path(run)
    state = json.loads((run / "tools-state.json").read_text())["sources"]
    out, wanted = [], set()
    for a in rows(run):
        s = a["subject"]
        try:
            normalize_time(registry_raw(a["native_time"]))
            note = ""
        except Exception as e:  # noqa: BLE001 - shown to the reviewer
            note = " TIME-ERR " + str(e)[:80]
        default = "DEFAULT" if a.get("proposed_decision") else ""
        out.append(
            f"## {a['id']} [{a.get('code') or s.get('code')}] {a.get('proposed_status')} map={s.get('mapping')} "
            f"{a['event']}/{a['role']} {default} loc={json.dumps(a.get('locator'))}{note}"
        )
        out.append(
            f"   label: {s.get('source_label')} | expr: {a['native_time'].get('expression', '')!r}"[
                :200
            ]
        )
        out.append(f"   Q: {a['quote'][:300]}")
        if default:
            out.append(f"   why: {a['proposed_decision'].get('rationale', '')}")
            loc = a.get("locator") if isinstance(a.get("locator"), dict) else {}
            if loc.get("pdf_page"):
                wanted.add((a["source_id"], int(loc["pdf_page"])))
    if pages and wanted:
        import pymupdf

        Path(pages).mkdir(parents=True, exist_ok=True)
        from workflow_tools.paths import REPO as repo

        for sid, page in sorted(wanted):
            raw = state.get(sid, {}).get("raw_path")
            if not raw or not (repo / raw).exists():
                continue
            try:
                doc = pymupdf.open(repo / raw)
            except Exception:  # noqa: BLE001 - not a PDF
                continue
            if 0 < page <= len(doc):
                dest = Path(pages) / f"{sid}-{page}.png"
                doc[page - 1].get_pixmap(dpi=110).save(dest)
                out.append(f"rendered {dest}")
    return "\n".join(out)


def adopt_clades(spec) -> list[dict]:
    """Adopt crown ages of deposited trees' clades for proto codes.

    spec: {"reviewer", "decisions": [{"code", "analysis", "taxa": [..] | "root",
    "proto_mapping", "rationale", "alternatives": [{"analysis", "taxa", "proto_mapping",
    "note"}]}]}. Each clade is found by its exact sampled-taxon set in the analysis's
    evidence; the registry checks eligibility as for any decision."""
    from .registry import build_bundle, bundle_decision_digest
    from .review import decide_record

    def clade(analysis, taxa):
        cl = json.loads((Path(DATA_DIR) / "_evidence" / f"{analysis}.json").read_text())
        if taxa == "root":
            return max(cl, key=lambda c: len(c["subject"]["taxa"]))
        m = [c for c in cl if set(c["subject"]["taxa"]) == set(taxa)]
        if len(m) != 1:
            raise ValueError(f"{analysis}: {len(m)} clades with exactly these taxa")
        return m[0]

    reviewer = reviewer_name(spec.get("reviewer"))
    bundle = build_bundle()
    results = []
    for d in spec["decisions"]:
        c = clade(d["analysis"], d["taxa"])
        sid = "wikt:" + d["code"]
        raw = {
            "subject_id": sid,
            "event": "crown_divergence",
            "claim_id": c["id"],
            "kind": "editorial_default",
            "evidence_digest": bundle_decision_digest(bundle, sid),
            "reviewer": reviewer,
            "rationale": d["rationale"],
            "proto_mapping": d["proto_mapping"],
            "dependencies": [],
            "proto_alternatives": [
                {
                    "claim_id": clade(alt["analysis"], alt["taxa"])["id"],
                    "proto_mapping": alt["proto_mapping"],
                    **({"note": alt["note"]} if alt.get("note") else {}),
                }
                for alt in d.get("alternatives", [])
            ],
        }
        results.append({"code": d["code"], "claim_id": c["id"], **decide_record(raw, bundle)})
    return results
