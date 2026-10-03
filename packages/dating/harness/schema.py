"""Pydantic models for the artifact, the JSON Schemas exported from them, and a
validator. Everything written under data/workspaces/dating/ conforms to these."""

from __future__ import annotations

import json
import os
from typing import Literal

from pydantic import BaseModel, Field, model_validator

from .config import SCHEMA_DIR

ClaimKind = Literal[
    "attestation_first",
    "attestation_last",
    "extinction",
    "conventional_period",
    "phylo_node_age",
    "archaeo_linguistic",
    "lexicostatistic",
    "expert_estimate",
    "derived",
]
Precision = Literal["year", "decade", "century", "millennium", "unknown"]


class TimeValue(BaseModel):
    """A dated claim's value. Exactly one of: a point (`value`), an interval
    (`earliest`/`latest`), or a distribution (`median`/`hpd95`). All years
    astronomical (0 = 1 BCE); `latest: null` with `open_end: true` = ongoing."""

    unknown: bool = False
    value: int | None = None
    earliest: int | None = None
    latest: int | None = None
    open_end: bool = False
    median: int | None = None
    mean: int | None = None
    hpd95: list[int] | None = Field(default=None, min_length=2, max_length=2)
    precision: Precision = "year"
    present: int | None = Field(
        default=None, description="the source's 'present' if it gave years BP"
    )
    conversion: str | None = Field(
        default=None, description="how the source's expression was converted"
    )

    @model_validator(mode="after")
    def _check(self):
        forms = [
            self.value is not None,
            self.earliest is not None or self.latest is not None,
            self.median is not None or self.mean is not None or self.hpd95 is not None,
        ]
        if sum(forms) > 1:
            raise ValueError("mixed temporal representations")
        if self.unknown and (any(forms) or self.open_end):
            raise ValueError("unknown time cannot assert a date")
        if sum(forms) == 0 and not self.open_end and not self.unknown:
            raise ValueError("time value has no value, interval, or distribution")
        if self.earliest is not None and self.latest is not None and self.earliest > self.latest:
            raise ValueError("earliest > latest")
        if self.hpd95 and self.hpd95[0] > self.hpd95[1]:
            raise ValueError("hpd95 out of order")
        return self


class MethodNotes(BaseModel):
    calibrations: int | None = None
    clock: str | None = None
    data: str | None = None
    taxa: int | None = None
    code_available: bool | None = None
    other: str | None = None


class Claim(BaseModel):
    id: str = Field(pattern=r"^clm_[a-z0-9]{4}[0-9]{4}_[0-9a-z_]+$")
    languoid: str | None = Field(
        default=None,
        description="glottocode; absent when the claim could not be placed on a Glottolog node",
    )
    languoid_name: str | None = None
    kind: ClaimKind
    node_role: Literal["crown", "stem", "interval"] | None = Field(
        default=None,
        description="for node-level claims: crown = first internal split, stem = split from the sister clade, interval = the proto-language was spoken during this window (formation / existence windows; never a crown point)",
    )
    time: TimeValue
    source: str = Field(description="id in sources.json")
    locator: str | None = Field(default=None, description="page, table, figure, row, or field")
    quote: str | None = Field(default=None, description="verbatim text supporting the claim")
    quote_verified: bool | None = Field(
        default=None, description="set by the harness: quote found in the retrieved text"
    )
    method_notes: MethodNotes | None = None
    extracted_by: dict = Field(default_factory=dict, description="model / program and run id")
    notes: str | None = None
    evidence: dict | None = None


class Source(BaseModel):
    id: str
    type: Literal["paper", "dataset", "database", "encyclopedia", "handbook", "thesis", "other"]
    title: str
    authors: list[str] = []
    year: int | None = None
    doi: str | None = None
    url: str | None = None
    version: str | None = None
    license: str | None = None
    retrieved: str | None = None
    sha256: str | None = Field(
        default=None, description="of the retrieved text, when full text was retrieved"
    )
    citation: str | None = None
    study_id: str | None = None
    analysis_id: str | None = None
    raw_sha256: str | None = None
    retrieval_quality: str | None = None


class Adopted(BaseModel):
    adopted: str | None = Field(description="claim id, or null when nothing could be adopted")
    alternatives: list[str] = []
    rationale: str | None = None


class Languoid(BaseModel):
    glottocode: str
    name: str
    level: Literal["language", "dialect", "family"]
    status: Literal[
        "living",
        "extinct",
        "historical",
        "ancient",
        "reconstructed",
        "contested",
        "constructed",
        "unknown",
    ]
    wiktionary: dict = Field(
        default_factory=dict, description="codes: language codes, family, proto_language"
    )
    spoken: dict | None = Field(default=None, description="{earliest, latest, open_end, basis}")
    node_age: dict[str, Adopted] | None = Field(default=None, description="crown / stem")
    attestation: dict[str, Adopted] | None = Field(default=None, description="first / last")
    extinction: Adopted | None = None
    conventional_period: Adopted | None = None
    rationale: str | None = None
    confidence: Literal["high", "medium", "low", "none"]
    open_questions: list[str] = []
    checks: list[str] = Field(default_factory=list, description="mechanical check violations")
    claims_considered: list[str] = []
    review: dict | None = Field(
        default=None,
        description="LLM synthesis review provenance; kept across rules re-runs while claims_considered is unchanged",
    )
    run: str


def export_schemas() -> None:
    os.makedirs(SCHEMA_DIR, exist_ok=True)
    from .evidence import (
        EvidenceClaim,
        Temporal,
        Decision,
        Subject,
        CatalogRecord,
        CodeView,
        RelativeConstraint,
    )
    from .research import ResearchGraph

    for model, name in [
        (Claim, "legacy-claim"),
        (Source, "source"),
        (EvidenceClaim, "claim"),
        (Temporal, "temporal"),
        (Decision, "decision"),
        (Subject, "subject"),
        (CatalogRecord, "languoid"),
        (CodeView, "code-view"),
        (RelativeConstraint, "relative-constraint"),
        (ResearchGraph, "research-graph"),
    ]:
        with open(os.path.join(SCHEMA_DIR, f"{name}.schema.json"), "w", encoding="utf-8") as f:
            json.dump(model.model_json_schema(), f, indent=2)
            f.write("\n")


if __name__ == "__main__":
    export_schemas()
