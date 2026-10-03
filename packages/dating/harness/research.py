"""Typed analytical dependencies; bibliographic multiplicity never measures support.

An absent edge means unassessed dependence, not independence. Critiques/replies
are scholarly relations, not derivations or additional chronological estimates.
"""

import json
from pathlib import Path
from typing import Literal
from pydantic import Field
from .evidence import StrictModel
from .config import DATA_DIR


class ResearchNode(StrictModel):
    id: str
    kind: Literal["study", "analysis", "dataset", "calibration_set", "result"]
    label: str
    scope: Literal["substantive", "methodological_demonstration", "sensitivity", "unassessed"] = (
        "unassessed"
    )
    details: dict = Field(default_factory=dict)


class ResearchRelation(StrictModel):
    source: str
    target: str
    relation: Literal[
        "analysis_of",
        "uses_dataset",
        "uses_calibration_set",
        "reports_result_of",
        "uses_prior_from",
        "derived_from",
        "critiques",
        "replies_to",
    ]
    basis: str = Field(min_length=1)
    references: list[str] = Field(min_length=1)
    status: Literal["proposed", "reviewed"] = "proposed"


class ResearchGraph(StrictModel):
    nodes: list[ResearchNode] = Field(default_factory=list)
    relations: list[ResearchRelation] = Field(default_factory=list)


def load_graph(claims, manifests):
    path = Path(DATA_DIR) / "_research/graph.json"
    curated = ResearchGraph.model_validate(json.loads(path.read_text()) if path.exists() else {})
    nodes = {n.id: n.model_dump() for n in curated.nodes}
    if len(nodes) != len(curated.nodes):
        raise ValueError("duplicate research node")
    relations = [r.model_dump() for r in curated.relations]
    for c in claims:
        nodes.setdefault(
            c["study_id"],
            ResearchNode(id=c["study_id"], kind="study", label=c["study_id"]).model_dump(),
        )
    for m in manifests.values():
        aid = "analysis:" + m["id"]
        nodes.setdefault(
            aid,
            ResearchNode(
                id=aid, kind="analysis", label=m["id"], details=m["methodology"]
            ).model_dump(),
        )
        nodes[aid]["details"] = {**m["methodology"], **nodes[aid].get("details", {})}
        nodes.setdefault(
            m["study_id"],
            ResearchNode(id=m["study_id"], kind="study", label=m["study_id"]).model_dump(),
        )
        edge = ResearchRelation(
            source=aid,
            target=m["study_id"],
            relation="analysis_of",
            basis="Explicit deposited-analysis manifest",
            references=[m["publication_url"]],
            status="reviewed",
        ).model_dump()
        if not any(
            (r["source"], r["target"], r["relation"]) == (aid, m["study_id"], "analysis_of")
            for r in relations
        ):
            relations.append(edge)
    for c in claims:
        aid = "analysis:" + str(c.get("analysis_id"))
        if not c.get("result_id") or aid not in nodes:
            continue
        rid = "result:" + c["result_id"]
        nodes.setdefault(
            rid, ResearchNode(id=rid, kind="result", label=c["subject"]["name"]).model_dump()
        )
        edge = ResearchRelation(
            source=rid,
            target=aid,
            relation="derived_from",
            basis="Explicit result/analysis identity in evidence record",
            references=[c["source_id"]],
            status="reviewed",
        ).model_dump()
        if edge not in relations:
            relations.append(edge)
    return {
        "nodes": sorted(nodes.values(), key=lambda n: n["id"]),
        "relations": sorted(relations, key=lambda r: (r["source"], r["relation"], r["target"])),
    }


def validate_graph(raw):
    try:
        graph = ResearchGraph.model_validate(raw)
    except ValueError as e:
        return ["invalid research graph: " + str(e)]
    errors = []
    nodes = {n.id: n for n in graph.nodes}
    dependencies = {}
    seen = set()
    if len(nodes) != len(graph.nodes):
        errors.append("duplicate research node")
    target_kinds = {
        "analysis_of": {"study"},
        "uses_dataset": {"dataset"},
        "uses_calibration_set": {"calibration_set"},
        "uses_prior_from": {"analysis", "result", "calibration_set"},
        "reports_result_of": {"analysis", "result"},
    }
    for r in graph.relations:
        key = (r.source, r.relation, r.target)
        if key in seen:
            errors.append("duplicate research relation")
        seen.add(key)
        if r.source not in nodes or r.target not in nodes:
            errors.append("research relation references missing node")
            continue
        if r.source == r.target:
            errors.append("self-referential research relation")
        if r.relation in target_kinds and nodes[r.target].kind not in target_kinds[r.relation]:
            errors.append("wrong research target kind: " + r.relation)
        if r.relation == "analysis_of" and nodes[r.source].kind != "analysis":
            errors.append("analysis_of requires analysis source")
        if r.relation in (
            "uses_dataset",
            "uses_calibration_set",
            "uses_prior_from",
            "derived_from",
            "reports_result_of",
        ):
            dependencies.setdefault(r.source, []).append(r.target)
    active = set()
    visited = set()

    def visit(node):
        if node in active:
            errors.append("research dependency cycle")
            return
        if node in visited:
            return
        active.add(node)
        for target in dependencies.get(node, []):
            visit(target)
        active.remove(node)
        visited.add(node)

    for node in nodes:
        visit(node)
    return errors
