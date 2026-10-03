"""Bounded source windows with complete primary-text accounting and exact context."""

import hashlib
import re
from .io_utils import digest
from .extraction_recovery import source_segment


def definition_context(text):
    """Exact sentence/line spans ranked for temporal interpretation, never facts.

    Sentence splitting is deliberately conservative and may miss context. The
    caller still receives omitted spans and must request missing methods/tables.
    A match is not permission to transfer an epoch to a cited study.
    """
    patterns = [
        (
            "epoch_definition",
            r"\bpresent\b.{0,100}\b(?:is|refers|defined|\d{3,4})\b|\b\d{3,4}\b.{0,100}\bpresent\b|\bBP\s+(?:means|refers)\b|B\.P\..{0,50}\b\d{3,4}\b",
        ),
        (
            "prior_definition",
            r"\b(?:uniform|normal|truncated|root|tip.date|calibration)\b.{0,65}\b(?:prior|distribution|range|deviation|bound)\b|\bcalibration ranges\b",
        ),
        (
            "branch_interval_definition",
            r"\b(?:interval|bound|range)\b.{0,100}\b(?:branch|node|transition)\b|\b(?:branch|node)\b.{0,100}\b(?:interval|bound|range)\b",
        ),
    ]
    hints = []
    for reason, pattern in patterns:
        # Newlines and conservative sentence boundaries keep a definition in a
        # long prose paragraph from consuming the entire context allowance.
        for match in re.finditer(r"[^\n]+", text):
            boundaries = [
                0,
                *[m.end() for m in re.finditer(r"(?<=[.!?])\s+(?=[A-Z])", match[0])],
                len(match[0]),
            ]
            for a, b in zip(boundaries, boundaries[1:]):
                sentence = match[0][a:b]
                if re.search(pattern, sentence, re.I):
                    hints.append(
                        {"start": match.start() + a, "end": match.start() + b, "selector": reason}
                    )
    return hints


def build_packets(
    text,
    source_id,
    *,
    headings=(),
    context_spans=(),
    omissions=(),
    max_chars=12000,
    context_chars=2500,
    overlap_chars=500,
):
    if not 0 <= overlap_chars < context_chars < max_chars:
        raise ValueError("require 0 <= overlap_chars < context_chars < max_chars")
    sha = hashlib.sha256(text.encode()).hexdigest()
    for span in [*headings, *context_spans]:
        if not (0 <= span["start"] < span["end"] <= len(text)):
            raise ValueError("context span outside source")
        if span.get("text", text[span["start"] : span["end"]]) != text[span["start"] : span["end"]]:
            raise ValueError("context span does not match source")

    def segment(start, end, purpose):
        return source_segment(text[start:end], source_id, purpose, start, sha)

    # Span selectors are heuristics, not assertions that all global definitions
    # have been located. Explicit caller-selected context takes precedence.
    hints = definition_context(text)
    body_limit = max_chars - context_chars
    packets = []
    start = 0
    while start < len(text):
        cap = min(start + body_limit, len(text))
        end = cap
        if cap < len(text):
            boundary = text.rfind("\n", start + body_limit // 2, cap)
            if boundary >= 0:
                end = boundary + 1
        primary = segment(start, end, "primary")
        contexts = []
        omitted = []
        selected = []
        remaining = context_chars
        ancestors = {}
        for h in sorted(headings, key=lambda h: h["start"]):
            if h["start"] > start:
                break
            level = h.get("level", 2)
            ancestors = {k: v for k, v in ancestors.items() if k < level}
            ancestors[level] = h
        candidates = [*context_spans, *ancestors.values(), *hints]
        if start:
            candidates.append({"start": max(0, start - overlap_chars), "end": start})
        seen = set()
        for span in candidates:
            a, b = span["start"], span["end"]
            if (a, b) in seen or start <= a and b <= end:
                continue
            seen.add((a, b))
            if b - a <= remaining:
                contexts.append(segment(a, b, "context"))
                remaining -= b - a
                selected.append(
                    {"start": a, "end": b, "selector": span.get("selector", "declared_or_overlap")}
                )
            else:
                omitted.append(
                    {
                        "start": a,
                        "end": b,
                        "reason": "context_budget",
                        "selector": span.get("selector", "declared_or_overlap"),
                    }
                )
        packet = {
            "id": f"{source_id}:{start}-{end}",
            "source_id": source_id,
            "text_sha256": sha,
            "primary": primary,
            "context": contexts,
            "context_omissions": omitted,
            "source_omissions": list(omissions),
            "boundary_split": end < len(text) and text[end - 1] != "\n",
            "context_selection": selected,
            "context_policy": "Caller spans, ancestor headings, then heuristic epoch/prior/branch definition sentences and overlap; not a guarantee of complete context. Study epochs never automatically apply to secondary reports.",
        }
        packet["sha256"] = digest(packet)
        packets.append(packet)
        start = end
    return {
        "version": 1,
        "source_id": source_id,
        "text_sha256": sha,
        "source_chars": len(text),
        "limits": {
            "max_chars": max_chars,
            "context_chars": context_chars,
            "overlap_chars": overlap_chars,
        },
        "packets": packets,
        "source_omissions": list(omissions),
        "coverage_scope": "Primary spans partition supplied text exactly. Acquisition completeness is separate.",
    }


def verify_packet(packet):
    if digest({k: v for k, v in packet.items() if k != "sha256"}) != packet["sha256"]:
        raise ValueError("section packet hash mismatch")
    for s in [packet["primary"], *packet["context"]]:
        if (
            s["source_id"] != packet["source_id"]
            or s["text_sha256"] != packet["text_sha256"]
            or s["end"] - s["start"] != len(s["text"])
        ):
            raise ValueError("section segment identity/offset mismatch")


def packet_prompt(packet):
    verify_packet(packet)
    metadata = {k: v for k, v in packet.items() if k not in ("primary", "context")}
    import json

    evidence = "\n\n".join(
        f"SOURCE SEGMENT {s['purpose']} [{s['start']}:{s['end']}]\n{s['text']}"
        for s in [packet["primary"], *packet["context"]]
    )
    return (
        "Extract chronological assertions from the PRIMARY segment. CONTEXT segments provide definitions/headings; "
        "do not repeat assertions solely from context. Quote within a single segment, never join noncontiguous excerpts. "
        "Return explicit gaps for missing definitions or truncated tables; a section with zero claims is still a reviewed section. "
        "Bibliographic publication years and dates in reference titles are not assertions established by this source. "
        "Metadata and segment markers are not evidence. No catalog mapping or historical adoption.\n"
        + json.dumps(metadata, ensure_ascii=False)
        + "\n\n"
        + evidence
    )


def reconcile(plan, results):
    """Keep every version; group only identical claims on the same source span."""
    rows = []
    groups = {}
    missing = []
    covered = 0
    scheduled = set()
    for cell in plan["cells"]:
        task = plan["tasks"][cell["task"]]
        if task.get("kind") not in ("section_extract", "compact_extract"):
            continue
        scheduled.add(cell["task"])
        packet = task["packet"]
        verify_packet(packet)
        result = results.get(cell["id"], {})
        recovery = result.get("recovery")
        complete = (
            result.get("status") == "completed"
            and recovery
            and recovery["status"] == "schema_valid"
        )
        rows.append(
            {
                "cell": cell["id"],
                "source_id": packet["source_id"],
                "primary_start": packet["primary"]["start"],
                "primary_end": packet["primary"]["end"],
                "status": result.get("status", "not_attempted"),
                "response_complete": bool(complete),
                "recovery_status": recovery.get("status") if recovery else None,
            }
        )
        if complete:
            covered += packet["primary"]["end"] - packet["primary"]["start"]
        else:
            missing.append(cell["id"])
        if not recovery:
            continue
        for claim in recovery["claims"]:
            if not claim["schema_valid"]:
                continue
            payload = claim["proposal"]
            spans = [{k: v for k, v in s.items() if k != "purpose"} for s in claim.get("spans", [])]
            # Exact structural identity only; different scenarios/quotes/values
            # remain separate even when a model repeats a catalog ID or label.
            key = digest(
                {
                    "source": packet["source_id"],
                    "proposal": payload,
                    "spans": sorted(spans, key=lambda s: (s["start"], s["end"])),
                }
            )
            group = groups.setdefault(
                key,
                {
                    "proposal": payload,
                    "spans": spans,
                    "occurrences": [],
                    "mechanically_ready": claim["mechanically_ready"],
                    "semantic_status": "pending",
                },
            )
            group["occurrences"].append(
                {
                    "cell": cell["id"],
                    "index": claim["index"],
                    "primary_evidence": claim["primary_evidence"],
                }
            )
    unscheduled = sorted(
        k
        for k, t in plan["tasks"].items()
        if t.get("kind") in ("section_extract", "compact_extract") and k not in scheduled
    )
    return {
        "version": 1,
        "sections": rows,
        "incomplete_cells": missing,
        "unscheduled_sections": unscheduled,
        "completed_primary_chars_across_cells": covered,
        "proposals": list(groups.values()),
        "document_complete": bool(rows) and not missing and not unscheduled,
        "scope": "Execution completeness only, not assertion recall or source completeness. Exact duplicate grouping is conservative; semantic duplicates/conflicts need review. Never imports claims.",
    }
