"""Version 2 scientific contract: observations, constraints and analysis-specific results.

Catalog membership is not chronological identity. No model permits an inherited
language birth date, an untyped interval, or a confidence score based on file count.
"""

from __future__ import annotations
from typing import Literal
from pydantic import BaseModel, ConfigDict, Field, model_validator
from .io_utils import digest


class StrictModel(BaseModel):
    model_config = ConfigDict(extra="forbid")


class Subject(StrictModel):
    id: str
    type: Literal[
        "catalog",
        "wiktionary_stage",
        "sampled_clade",
        "documentary_sample",
        "reconstructed_stage",
        "population",
        "unresolved",
    ]
    name: str
    catalog_id: str | None = None
    mapping: Literal["exact", "subset", "superset", "overlap", "unresolved"] = "unresolved"
    mapping_basis: str | None = None
    taxa: list[str] = Field(default_factory=list)
    source_label: str | None = None
    catalog_mapping: Literal["exact", "subset", "superset", "overlap", "unresolved"] = "unresolved"
    mapping_certainty: Literal["checked", "proposed", "unresolved"] = "unresolved"
    geographic_scope: str | None = None
    identification_alternatives: list[str] = Field(default_factory=list)


class Temporal(StrictModel):
    form: Literal["point", "event_range", "duration", "bound", "distribution", "unknown"]
    value: int | None = None
    lower: int | None = None
    upper: int | None = None
    relation: Literal["before", "after", "by", "since"] | None = None
    mean: int | None = None
    median: int | None = None
    interval_type: Literal["hpd", "equal_tailed", "reported_range", "unknown"] | None = None
    probability: float | None = Field(default=None, gt=0, lt=1)
    ongoing: bool = False
    precision: Literal["year", "decade", "century", "millennium", "unknown"] = "unknown"
    original: dict = Field(default_factory=dict)
    normalization: Literal["program_verified", "legacy_unverified", "unknown"] = "unknown"

    @model_validator(mode="after")
    def valid(self):
        if self.lower is not None and self.upper is not None and self.lower > self.upper:
            raise ValueError("reversed temporal bounds")
        if self.form == "point":
            if self.value is None or any(
                x is not None
                for x in (
                    self.lower,
                    self.upper,
                    self.mean,
                    self.median,
                    self.relation,
                    self.interval_type,
                    self.probability,
                )
            ):
                raise ValueError("point requires only value")
        elif self.form == "distribution":
            if self.value is not None or self.relation is not None or self.ongoing:
                raise ValueError("distribution cannot contain point/duration fields")
            if self.mean is None and self.median is None and self.lower is None:
                raise ValueError("empty distribution")
            if (self.lower is None) != (self.upper is None):
                raise ValueError("distribution interval needs both bounds")
            if self.lower is not None and self.interval_type is None:
                raise ValueError("distribution interval needs type")
            if (
                self.lower is not None
                and self.interval_type in ("hpd", "equal_tailed")
                and self.probability is None
            ):
                raise ValueError("statistical interval needs probability")
            if self.lower is None and (
                self.interval_type is not None or self.probability is not None
            ):
                raise ValueError("interval metadata requires bounds")
        elif self.form in ("event_range", "duration"):
            if any(
                x is not None
                for x in (
                    self.value,
                    self.mean,
                    self.median,
                    self.relation,
                    self.interval_type,
                    self.probability,
                )
            ):
                raise ValueError("range cannot contain point/distribution fields")
            if self.lower is None and self.upper is None:
                raise ValueError("empty range")
            if self.form == "event_range" and (
                self.lower is None or self.upper is None or self.ongoing
            ):
                raise ValueError("uncertain event requires two bounds")
        elif self.form == "bound":
            if (
                self.value is None
                or self.relation is None
                or any(
                    x is not None
                    for x in (
                        self.lower,
                        self.upper,
                        self.mean,
                        self.median,
                        self.interval_type,
                        self.probability,
                    )
                )
            ):
                raise ValueError("bound requires value and relation only")
        elif any(
            x is not None
            for x in (
                self.value,
                self.lower,
                self.upper,
                self.mean,
                self.median,
                self.relation,
                self.interval_type,
                self.probability,
            )
        ):
            raise ValueError("unknown cannot contain numerical assertions")
        if self.ongoing and (self.form != "duration" or self.upper is not None):
            raise ValueError("ongoing is a duration with no upper endpoint")
        return self


class EvidenceSpan(StrictModel):
    text_sha256: str = Field(pattern=r"^[a-f0-9]{64}$")
    start: int = Field(ge=0)
    end: int = Field(gt=0)
    quote: str
    locator: str | None = None

    @model_validator(mode="after")
    def ordered(self):
        if self.end <= self.start or len(self.quote) != self.end - self.start:
            raise ValueError("span length does not match quote")
        return self


class Assessment(StrictModel):
    textual: Literal["exact_span", "mechanical", "paraphrase", "unverified", "failed"] = (
        "unverified"
    )
    semantic: Literal["pending", "checked", "rejected"] = "pending"
    mapping: Literal["pending", "checked", "rejected"] = "pending"
    limitations: list[str] = Field(default_factory=list)
    reviewer: str | None = None
    rationale: str | None = None


class EvidenceClaim(StrictModel):
    schema_version: Literal[2] = 2
    id: str
    legacy_id: str | None = None
    subject: Subject
    event: Literal[
        "first_attestation",
        "last_attestation",
        "conventional_period",
        "native_transmission_end",
        "documentary_sampling",
        "composition",
        "manuscript_production",
        "last_documentation",
        "last_fluent_speaker_report",
        "regional_language_shift",
        "literary_use",
        "crown_divergence",
        "stem_divergence",
        "model_lineage_span",
        "linguistic_divergence",
        "sound_change",
        "formation",
        "contact",
        "population_event",
        "relative_constraint",
        "unspecified",
    ]
    role: Literal[
        "observation",
        "convention",
        "calibration_prior",
        "posterior",
        "secondary_report",
        "hypothesis",
        "editorial",
        "unknown",
    ]
    method: str
    time: Temporal
    source_id: str
    study_id: str
    analysis_id: str | None = None
    result_id: str | None = None
    reported_from: str | None = None
    spans: list[EvidenceSpan] = Field(default_factory=list)
    assessment: Assessment = Field(default_factory=Assessment)
    dependencies: list[str] = Field(default_factory=list)
    lineage_span_endpoints: list[str] = Field(default_factory=list)
    provenance: dict = Field(default_factory=dict)
    status: Literal["candidate", "supported", "quarantined", "superseded"] = "candidate"

    @model_validator(mode="after")
    def supported(self):
        if self.status == "supported":
            if (
                not (self.assessment.reviewer or "").strip()
                or not (self.assessment.rationale or "").strip()
            ):
                raise ValueError("supported claim requires reviewer and rationale")
            if self.assessment.textual == "mechanical" and not self.provenance.get("manifest"):
                raise ValueError("mechanical support requires a verifiable analysis manifest")
            if self.assessment.semantic != "checked" or self.assessment.mapping != "checked":
                raise ValueError("supported claim requires semantic and mapping checks")
            relative = self.event == "relative_constraint" and self.time.form == "unknown"
            if self.assessment.textual not in ("exact_span", "mechanical") or (
                not relative and self.time.normalization != "program_verified"
            ):
                raise ValueError("supported claim requires verified evidence and time")
            if self.assessment.textual == "exact_span" and not self.spans:
                raise ValueError("exact-span evidence has no span")
            if self.subject.mapping != "exact" or (self.time.form == "unknown" and not relative):
                raise ValueError("supported claim requires exact subject and known temporal form")
            if self.role in ("secondary_report", "unknown") or self.event == "unspecified":
                raise ValueError("unresolved role/event cannot be supported")
            if self.event in ("crown_divergence", "stem_divergence") and not self.analysis_id:
                raise ValueError("divergence requires study-specific analysis identity")
            if self.role == "posterior" and (not self.analysis_id or not self.result_id):
                raise ValueError("posterior requires analysis and result identity")
            if (
                self.role == "posterior"
                and self.event in ("crown_divergence", "stem_divergence")
                and (self.subject.type != "sampled_clade" or len(set(self.subject.taxa)) < 2)
            ):
                raise ValueError("posterior divergence requires the sampled descendant set")
            if self.subject.id.startswith("wikt:") and self.subject.type not in (
                "wiktionary_stage",
                "reconstructed_stage",
            ):
                raise ValueError(
                    "Wiktionary chronology requires a stage subject, not a population or clade"
                )
            if (self.event == "population_event") != (self.subject.type == "population"):
                raise ValueError("population dates require population subjects and events")
            if self.event == "model_lineage_span" and (
                not self.analysis_id or self.subject.type != "sampled_clade"
            ):
                raise ValueError("model lineage spans require a sampled clade and one analysis")
            if self.event == "model_lineage_span" and len(self.lineage_span_endpoints) != 2:
                raise ValueError("model lineage span requires two explicit endpoint claims")
        return self


class Decision(StrictModel):
    subject_id: str
    event: str
    claim_id: str
    evidence_digest: str
    reviewer: str
    rationale: str
    dependencies: list[str] = Field(default_factory=list)
    kind: Literal["editorial_default"] = "editorial_default"
    # Only for event crown_divergence on a proto-language code: why the selected
    # clade's sampled taxa cover the family whose ancestor the code names.
    proto_mapping: str | None = None
    # Reviewed alternative crown ages for the same proto-language (other analyses or
    # scenarios), shown beside the default; each says why its taxa cover the family.
    proto_alternatives: list["ProtoAlternative"] = Field(default_factory=list)


class ProtoAlternative(StrictModel):
    claim_id: str
    proto_mapping: str
    note: str | None = None


Decision.model_rebuild()


def evidence_digest(claims, context):
    """Includes complete evidence plus taxonomy/rules, not positional claim IDs."""
    return digest({"claims": sorted(claims, key=lambda c: c["id"]), "context": context})


def convert_number(value: float, unit: str, present: int | None = None) -> int:
    """Only explicit epochs. Calendar BC has no year zero; internal years do."""
    if unit in ("BP", "ka_BP", "century_BP", "years_ago", "ka_ago"):
        if present is None:
            raise ValueError("BP epoch must be explicit; do not assume 1950 or 2000")
        return round(
            present
            - value * (1000 if unit in ("ka_BP", "ka_ago") else 100 if unit == "century_BP" else 1)
        )
    if unit == "BCE":
        if value <= 0:
            raise ValueError("BCE year must be positive")
        return 1 - round(value)
    if unit == "CE":
        if value <= 0:
            raise ValueError("CE year must be positive")
        return round(value)
    if unit == "astronomical":
        return round(value)
    raise ValueError("unknown time unit: " + unit)


def endpoint_bounds(raw: dict) -> tuple[tuple[int, int] | None, tuple[int, int] | None]:
    """(earliest, latest) astronomical years for each endpoint of a period whose
    endpoints are stated separately: a period spanning eras ("1250 BC – AD 220") or
    with uncertain boundaries ("~500/400 – 300 BC/AD 1"). Each endpoint is
    {value, unit} or {earliest: {value, unit}, latest: {value, unit}}."""
    from .time_interpretation import PERIOD_UNITS, period_bounds

    def point(p):
        if p["unit"] in PERIOD_UNITS:
            return period_bounds(p["value"], p["unit"])
        year = convert_number(p["value"], p["unit"])
        return year, year

    def bounds(ep):
        if ep is None:
            return None
        if set(ep) == {"value", "unit"}:
            return point(ep)
        if set(ep) == {"earliest", "latest"}:
            lo, hi = point(ep["earliest"])[0], point(ep["latest"])[1]
            if lo > hi:
                raise ValueError("endpoint earliest is after latest")
            return lo, hi
        raise ValueError("endpoint requires {value, unit} or {earliest, latest}")

    return bounds(raw.get("start")), bounds(raw.get("end"))


def normalize_time(raw: dict) -> Temporal:
    """Normalize extracted/source-native quantities; never trust converted LLM years."""
    if "start" in raw or "end" in raw:
        if raw.get("form") != "duration" or any(
            raw.get(k) is not None for k in ("unit", "value", "lower", "upper")
        ):
            raise ValueError("separately stated endpoints belong to a duration without unit/bounds")
        start, end = endpoint_bounds(raw)
        if start is None and end is None:
            raise ValueError("empty range")
        return Temporal(
            form="duration",
            lower=start[0] if start else None,
            upper=end[1] if end else None,
            ongoing=raw.get("ongoing", False),
            precision=raw.get("precision", "unknown"),
            original=raw,
            normalization="program_verified",
        )
    if raw.get("form") == "unknown":
        if any(raw.get(k) is not None for k in ("value", "mean", "median", "lower", "upper")):
            raise ValueError("unknown native time cannot contain numeric assertions")
        return Temporal(form="unknown", original=raw)
    from .time_interpretation import calendar_epoch, check_age_scale, period_bounds, PERIOD_UNITS

    unit = raw["unit"]
    epoch = calendar_epoch(raw)
    check_age_scale(raw)
    kw = {
        k: raw[k]
        for k in ("form", "relation", "interval_type", "probability", "ongoing", "precision")
        if k in raw
    }
    if unit in PERIOD_UNITS:
        if raw["form"] not in ("bound", "event_range", "duration"):
            raise ValueError(
                "century/millennium expressions require a range or bound, not a point/distribution"
            )
        kw["precision"] = "millennium" if unit.startswith("millennium") else "century"
        if raw["form"] == "bound":
            lo, hi = period_bounds(raw["value"], unit)
            kw["value"] = hi if raw["relation"] in ("by", "after") else lo
        else:
            bounds = [
                period_bounds(raw[k], unit) for k in ("lower", "upper") if raw.get(k) is not None
            ]
            if len(bounds) != 2:
                raise ValueError("coarse calendar range requires both ordinal endpoints")
            kw.update(lower=min(x[0] for x in bounds), upper=max(x[1] for x in bounds))
        return Temporal(**kw, original=raw, normalization="program_verified")
    for k in ("value", "mean", "median", "lower", "upper"):
        if raw.get(k) is not None:
            kw[k] = convert_number(raw[k], unit, epoch)
    if unit in ("BCE", "BP", "ka_BP", "century_BP", "years_ago", "ka_ago"):
        lo, hi = kw.pop("lower", None), kw.pop("upper", None)
        if hi is not None:
            kw["lower"] = hi
        if lo is not None:
            kw["upper"] = lo
    return Temporal(**kw, original=raw, normalization="program_verified")


class RelativeConstraint(StrictModel):
    """Relations between source-specific events, never whole-language lifespans."""

    before_event: str
    after_event: str
    relation: Literal["precedes", "not_after", "overlaps"]
    basis_claim_ids: list[str] = Field(min_length=1)
    rationale: str = Field(min_length=1)


class CatalogRecord(StrictModel):
    schema_version: Literal[2] = 2
    glottocode: str
    name: str
    level: str
    status: str
    evidence_ids: list[str]
    study_ids: list[str]
    chronology_policy: str
    review_state: Literal["needs_review", "supported_evidence", "unknown"]


class CodeView(StrictModel):
    schema_version: Literal[2] = 2
    code: str
    name: str
    kind: str
    glottocode: str | None
    mapping: dict
    spoken: None = None
    chronology: list[dict]
    evidence_ids: list[str]
    catalog_context: list[str]
    ancestral_context: list[str]
    display_status: Literal["documented_claims", "no_verified_code_specific_chronology"]
    defaults: list[Decision]
