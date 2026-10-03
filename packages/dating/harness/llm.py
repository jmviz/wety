"""The three model-driven stages (plan §B3 screening, §B5 extraction, §B6
synthesis review). Prompts and output schemas live HERE and only here; the
model call goes through llm_backends (OpenRouter / Anthropic SDK / Claude Agent
SDK), all of which receive the same (role, system, user, schema).

Extraction is forbidden from emitting a claim without a verbatim quote, and
the harness — not the model — sets `quote_verified` by finding the quote in the
retrieved text. Unverified claims are kept, flagged, and never adopted."""

from __future__ import annotations
from .io_utils import atomic_json

import json
import os
import re
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator

from . import glotto
from .config import family_dir, now
from .llm_backends import LLMResult, get_backend
from .retrieve import text_for
from .schema import Claim, MethodNotes, TimeValue
from .store import append_log, load_claims, load_languoids, save_languoids, upsert_claims
from .tasks import task_subtree
from .time_utils import parse_era
from .time_interpretation import (
    BPConvention,
    AGE_UNITS,
    PERIOD_UNITS,
    UnresolvedCalendar,
    calendar_epoch,
    native_numbers_match,
)

CALENDAR_RULES = (
    "Calendar convention: report years as integers in astronomical numbering (year 0 = 1 BCE, so 500 BCE = -499, "
    "1200 BCE = -1199, 100 CE = 100). Retain source-native values; the program converts them. "
    "BP conventionally refers to 1950, while a source-specific epoch overrides that convention. "
    "Keep source-stated present separate from epoch_interpretation (conventional_bp_1950 with literal BP notation). "
    "Never assign the age, publication year or current year as the epoch. Bare years ago/kya need context. "
    "Set age_scale to calendar only when justified (model/calendar years or calibrated cal BP); "
    "radiocarbon_uncalibrated and unknown scales cannot be converted by subtraction."
)

KINDS_RULES = """Record source-specific events, observations, constraints and models, not language lifespans.
The legacy kind field is only a transport category; method, event and epistemic_role carry the interpretation.
- attestation_first / attestation_last: documentary observations, with disputed attribution retained.
- extinction: use the specific event (native transmission, regional shift, last documentation or reported speaker).
- conventional_period: a scholarly stage convention, not an inferred biological lifetime.
- phylo_node_age: an analysis-specific sampled divergence; distinguish mean, median, HPD and equal-tailed intervals.
- archaeo_linguistic: an argued association, never automatic identity of cultures, populations and languages.
- lexicostatistic: a method-labeled estimate, not a fallback birth date.
- expert_estimate: legacy transport for other arguments; describe the actual method, including contact chronology.
Use linguistic_divergence for a source-argued differentiation of a speech continuum and sound_change for an identified innovation. These do not by themselves identify a sampled-tree crown.
Use model_lineage_span for a within-analysis stem/crown branch span; never equate it to a reconstructed stage.
Keep source clade labels and sampled descendants. Catalog correspondence is a separate, possibly unresolved mapping.
Keep primary results, secondary quotations, calibration priors, critiques and replies distinct.
For source_values, a reported mean, median or stated probability distribution (including a uniform calibration prior) uses form distribution. Use event_range only for uncertain event bounds without a stated probability distribution. Do not put a mean or median in value, and do not give a point or event_range distribution metadata.
A mean or median without interval endpoints requires interval_type=null and probability=null, even if the uncertainty type is unknown. A node age is an event, not a model_lineage_span: the latter requires two linked endpoints. A bound such as 'by 2500 BP' is not a point. Population expansion is not linguistic attestation or divergence merely because speakers are identified linguistically.
Posterior estimates remain posterior even when a study presents them only as a methodological sensitivity demonstration; that limited purpose does not turn them into editorial dates or independent historical confirmation.
Dates of composition, surviving manuscripts, sampled corpora and earliest surviving attestations differ.
Do not rank by publication year, method label, calibration count or reference count. Never average alternatives.
Unknown absolute dates and relative chronological constraints are legitimate evidence.
Every assertion needs a verbatim quotation; quotation matching alone does not validate its interpretation."""


# --------------------------------------------------------------------------- #
# screening
# --------------------------------------------------------------------------- #


class ScreenDecision(BaseModel):
    decision: Literal["include", "exclude"]
    reason: str = Field(description="one sentence")
    likely_kinds: list[str] = Field(
        default_factory=list, description="claim kinds the paper plausibly contains"
    )


def screen_system(subtree_names: list[str]) -> str:
    return (
        "You screen bibliographic records for a registry of source-specific dated events, observations, constraints and models. "
        "A language has no universally identifiable start or end; distinguish sampled divergence, documentary evidence and transmission states. "
        f"The family under study: {', '.join(subtree_names)}. Decide from title and abstract only whether the paper plausibly "
        "contains a dated claim of one of these kinds about a language or node in this family: attestation dates, extinction "
        "dates, conventional period boundaries, phylogenetic node ages, archaeological-linguistic dating, lexicostatistic dates, "
        "or expert date estimates. Also INCLUDE methodological critiques, calibration arguments, borrowing strata, relative chronology, and sources testing classifications even without numeric dates. Include when plausible; exclude papers about synchronic grammar, typology, acquisition, or "
        "unrelated families. Genetics, archaeology and population-history papers are included ONLY if the title/abstract itself "
        "makes a relevant language-association argument, which must be kept distinct from population dates; otherwise "
        "exclude them — their full texts are long and their dated claims are about populations. Be brief.\n"
        + KINDS_RULES
    )


def screen_user(c: dict) -> str:
    extra = ""
    if c.get("tldr"):
        extra += f"\nTL;DR (Semantic Scholar): {c['tldr']}"
    for seed, ctxs in (c.get("citation_contexts") or {}).items():
        extra += f"\nHow this paper cites {seed}: " + " | ".join(x[:300] for x in ctxs[:2])
    return f"Title: {c.get('title')}\nYear: {c.get('year')}\nVenue: {c.get('venue')}\nAuthors: {', '.join(c.get('authors') or [])}\nAbstract: {c.get('abstract') or '(none)'}{extra}"


def _apply_screen(c: dict, res: LLMResult) -> None:
    d: ScreenDecision = res.parsed
    c["screen"] = {
        "decision": d.decision,
        "reason": d.reason,
        "likely_kinds": d.likely_kinds,
        "model": res.model,
        "backend": res.backend,
        "at": now(),
        "policy_version": 3,
    }


def screen_candidates(family: str, subtree_names: list[str], batch: bool = False) -> dict:
    path = os.path.join(family_dir(family), "candidates.json")
    with open(path, encoding="utf-8") as f:
        cands = json.load(f)
    system = screen_system(subtree_names)
    todo = [
        c
        for c in cands
        if not c.get("screen")
        or c["screen"].get("model") == "keyword"
        or c["screen"].get("policy_version") != 3
    ]
    counts = {"include": 0, "exclude": 0, "error": 0, "submitted": 0}
    backend = get_backend(role="screen")
    if batch:
        from .batch import submit

        counts["submitted"] = submit(
            family,
            "screen",
            [(c["key"], system, screen_user(c)) for c in todo],
            ScreenDecision,
            backend,
        )
    else:
        from .model_requests import freeze_request, record_response, record_failure

        for n, c in enumerate(todo, 1):
            user = screen_user(c)
            request_ref = freeze_request(
                "screen", family, c["key"], system, user, ScreenDecision, backend
            )
            try:
                res = backend.complete("screen", system, user, ScreenDecision, session_id=family)
                response_ref = record_response(request_ref, res)
                _apply_screen(c, res)
                c["screen"].update(request_ref=request_ref, response_ref=response_ref)
                counts[c["screen"]["decision"]] += 1
                append_log(
                    family,
                    "screen_log.jsonl",
                    {
                        "key": c["key"],
                        "title": c.get("title"),
                        "screen": c["screen"],
                        **res.log_fields(),
                    },
                )
            except Exception as e:  # noqa: BLE001
                c["screen_error"] = str(e)
                counts["error"] += 1
                failure_ref = record_failure(request_ref, e, getattr(backend, "last_failure", None))
                append_log(
                    family,
                    "screen_log.jsonl",
                    {
                        "key": c["key"],
                        "title": c.get("title"),
                        "request_ref": request_ref,
                        "failure_ref": failure_ref,
                        "error": str(e),
                    },
                )
            if n % 10 == 0:
                atomic_json(path, cands)
    atomic_json(path, cands)
    counts["already"] = (
        sum(1 for c in cands if c.get("screen") and c["screen"].get("model") != "keyword")
        - counts["include"]
        - counts["exclude"]
    )
    return counts


# --------------------------------------------------------------------------- #
# extraction
# --------------------------------------------------------------------------- #


class SourceTime(BaseModel):
    """Closed model-output transport; retained historical evidence stays unchanged."""

    model_config = ConfigDict(extra="forbid", allow_inf_nan=False)
    form: Literal["point", "event_range", "duration", "bound", "distribution", "unknown"]
    unit: (
        Literal[
            "BCE",
            "CE",
            "BP",
            "ka_BP",
            "century_BP",
            "years_ago",
            "ka_ago",
            "astronomical",
            "century_BCE",
            "century_CE",
            "millennium_BCE",
            "millennium_CE",
        ]
        | None
    ) = None
    present: int | None = Field(
        default=None,
        description="Explicit source BP epoch; null when unstated. Never infer from another analysis.",
    )
    epoch_interpretation: BPConvention | None = Field(
        default=None,
        description="Optional conventional 1950 interpretation of explicit BP notation, separate from source-stated present. Omit when present is stated; bare years ago is not BP notation.",
    )
    age_scale: Literal["calendar", "radiocarbon_uncalibrated", "unknown"] = Field(
        default="unknown",
        description="Calendar/model years, uncalibrated radiocarbon years, or unresolved scale; knowing the epoch alone does not establish calendar years.",
    )
    value: float | None = None
    mean: float | None = None
    median: float | None = None
    lower: float | None = Field(
        default=None, description="Smaller source-native numeric bound, before calendar conversion"
    )
    upper: float | None = Field(
        default=None, description="Larger source-native numeric bound, before calendar conversion"
    )
    relation: Literal["before", "after", "by", "since"] | None = None
    interval_type: Literal["hpd", "equal_tailed", "reported_range", "unknown"] | None = None
    probability: float | None = Field(
        default=None,
        description="Interval probability as a fraction, e.g. 0.95; null for nonstatistical ranges",
    )
    ongoing: bool = False
    precision: Literal["year", "decade", "century", "millennium", "unknown"] = "unknown"

    @model_validator(mode="after")
    def native_shape(self):
        # Validate before conversion, including when BP has no epoch. Shapes are
        # independent of calendar orientation and preserve fractional native units.
        from .evidence import Temporal

        values = self.model_dump(exclude_none=True)
        shape = {
            k: v
            for k, v in values.items()
            if k not in ("unit", "present", "epoch_interpretation", "age_scale")
        }
        # Temporal's integer calendar fields are used only for structural checks;
        # ordinal stand-ins retain equality/order without fabricating calendar dates.
        numbers = sorted(
            {shape[k] for k in ("value", "mean", "median", "lower", "upper") if k in shape}
        )
        for key in ("value", "mean", "median", "lower", "upper"):
            if key in shape:
                shape[key] = numbers.index(shape[key])
        Temporal(**shape)
        if self.form != "unknown" and self.unit is None:
            raise ValueError("numeric native time requires a unit")
        calendar_epoch(values)
        if self.unit in PERIOD_UNITS:
            from .time_interpretation import period_bounds

            if self.form not in ("bound", "event_range", "duration"):
                raise ValueError("coarse calendar expression requires range or bound")
            for key in ("value", "lower", "upper"):
                if values.get(key) is not None:
                    period_bounds(values[key], self.unit)
        return self


class SourcePoint(SourceTime):
    form: Literal["point"]
    value: float
    mean: None = None
    median: None = None
    lower: None = None
    upper: None = None
    relation: None = None
    interval_type: None = None
    probability: None = None
    ongoing: Literal[False] = False


class SourceDistribution(SourceTime):
    form: Literal["distribution"]
    value: None = None
    relation: None = None
    ongoing: Literal[False] = False


class SourceRange(SourceTime):
    form: Literal["event_range", "duration"]
    value: None = None
    mean: None = None
    median: None = None
    relation: None = None
    interval_type: None = None
    probability: None = None


class SourceBound(SourceTime):
    form: Literal["bound"]
    value: float
    relation: Literal["before", "after", "by", "since"]
    mean: None = None
    median: None = None
    lower: None = None
    upper: None = None
    interval_type: None = None
    probability: None = None
    ongoing: Literal[False] = False


class SourceUnknown(SourceTime):
    form: Literal["unknown"]
    value: None = None
    mean: None = None
    median: None = None
    lower: None = None
    upper: None = None
    relation: None = None
    interval_type: None = None
    probability: None = None
    ongoing: Literal[False] = False


class ExtractedTime(BaseModel):
    source_values: (
        SourcePoint | SourceDistribution | SourceRange | SourceBound | SourceUnknown | None
    ) = Field(
        default=None,
        description="Source-native time in the specified fields. Use distribution for any reported mean/median or probability distribution, including uniform priors; point permits only value. Bounds are in increasing source numeric order. Use numbers without unit strings; never guess the BP epoch. Null fields must stay null for the selected form. This is the authoritative input to programmatic normalization.",
    )
    expression: str = Field(
        description="the date expression exactly as the text gives it, e.g. '2450 BP', '8th century BCE', '6.5 kya (95% HPD 5.9–7.3)'"
    )
    value: int | None = Field(default=None, description="point estimate, astronomical year")
    earliest: int | None = None
    latest: int | None = None
    median: int | None = None
    mean: int | None = None
    hpd95_low: int | None = Field(
        default=None, description="older bound of the 95% HPD, astronomical year"
    )
    hpd95_high: int | None = Field(default=None, description="younger bound")
    precision: Literal["year", "decade", "century", "millennium", "unknown"] = "year"
    present: int | None = Field(
        default=None, description="the paper's 'present' when the expression is in years BP"
    )
    conversion: str = Field(description="how you converted the expression to astronomical years")


class ExtractedClaim(BaseModel):
    glottocode: str | None = Field(
        description="from the provided list, or null if the languoid is not in it"
    )
    languoid_name: str = Field(description="the language/node as the paper names it")
    kind: Literal[
        "attestation_first",
        "attestation_last",
        "extinction",
        "conventional_period",
        "phylo_node_age",
        "archaeo_linguistic",
        "lexicostatistic",
        "expert_estimate",
    ]
    node_role: Literal["crown", "stem", "interval"] | None = Field(
        default=None,
        description="Legacy structural label: crown = sampled clade's modeled internal divergence; stem = modeled split from its sampled sister; interval = source-stated span. None establishes a named proto-language lifespan; use event to distinguish model lineage spans and conventional stages.",
    )
    time: ExtractedTime
    locator: str = Field(description="page / section / table / figure where the quote is")
    quote: str = Field(
        description="verbatim sentence(s) from the text that state the claim; copy exactly"
    )
    calibrations: int | None = None
    clock: str | None = None
    data: str | None = None
    taxa: int | None = None
    code_available: bool | None = None
    method_notes: str | None = None
    subject_type: Literal[
        "catalog",
        "wiktionary_stage",
        "sampled_clade",
        "documentary_sample",
        "reconstructed_stage",
        "population",
        "unresolved",
    ] = "unresolved"
    subject_id: str | None = None
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
        "model_lineage_span",
        "linguistic_divergence",
        "sound_change",
        "crown_divergence",
        "stem_divergence",
        "formation",
        "contact",
        "population_event",
        "relative_constraint",
        "unspecified",
    ] = "unspecified"
    epistemic_role: Literal[
        "observation",
        "convention",
        "calibration_prior",
        "posterior",
        "secondary_report",
        "hypothesis",
        "editorial",
        "unknown",
    ] = "unknown"
    analysis_id: str | None = None
    result_id: str | None = None
    method: str = Field(
        default="unspecified",
        description="Actual dating method, independent of legacy kind and epistemic role",
    )
    sampled_taxa: list[str] = Field(default_factory=list)
    geographic_scope: str | None = None
    identification_alternatives: list[str] = Field(default_factory=list)
    relative_constraints: list[str] = Field(default_factory=list)
    reported_from: str | None = Field(
        default=None,
        description="if the text is reporting another work's result, that work (author/year); null when this text's own result",
    )


class ResearchGap(BaseModel):
    kind: Literal[
        "missing_source",
        "missing_context",
        "unreadable_text",
        "unknown_epoch",
        "uncertain_entity",
        "missing_diagnostics",
        "other",
    ]
    description: str
    locator: str | None = None
    affected_subjects: list[str] = Field(default_factory=list)
    next_action: str = Field(
        description="Bounded recovery or review action; do not invent missing evidence"
    )


class ConflictSide(BaseModel):
    locator: str = Field(description="Exact file and page/section/row")
    quote: str = Field(description="Verbatim supporting text, not harness metadata")


class ResearchConflict(BaseModel):
    kind: Literal[
        "quantity",
        "statistic",
        "epoch",
        "entity",
        "prior_or_model",
        "scenario",
        "interpretation",
        "other",
    ]
    description: str
    sides: list[ConflictSide] = Field(
        min_length=2, description="At least two separately located source assertions; retain both"
    )
    next_action: str


class Extraction(BaseModel):
    claims: list[ExtractedClaim]
    paper_summary: str = Field(description="two sentences: what the paper dates and how")
    gaps: list[ResearchGap] = Field(
        default_factory=list,
        description="Explicit access/context/interpretation gaps, even when no claims are extracted",
    )
    conflicts: list[ResearchConflict] = Field(
        default_factory=list, description="Conflicting source assertions; do not reconcile silently"
    )


def normalize_ws(s: str) -> str:
    s = s.replace("’", "'").replace("‘", "'").replace("“", '"').replace("”", '"')
    s = s.replace("–", "-").replace("—", "-").replace("­", "").replace("ﬁ", "fi").replace("ﬂ", "fl")
    s = re.sub(r"-\s*\n\s*", "", s)  # hyphenation at line ends
    return re.sub(r"\s+", " ", s).strip().casefold()


def verify_quote(quote: str, text: str) -> bool:
    q, t = normalize_ws(quote), normalize_ws(text)
    if len(q) < 20:
        return False
    if q in t:
        return True
    return False


def _time_value(t: ExtractedTime) -> TimeValue:
    from .evidence import normalize_time

    if not t.source_values:
        raise ValueError(
            "source-native values required; model calendar conversions are not accepted"
        )
    native = t.source_values.model_dump(exclude_none=True)
    try:
        x = normalize_time(native)
    except UnresolvedCalendar:
        return TimeValue(unknown=True)
    except ValueError:
        if native.get("unit") in AGE_UNITS and native.get("present") is None:
            return TimeValue(unknown=True)
        raise
    if x.form == "unknown":
        return TimeValue(unknown=True)
    base = dict(
        precision=x.precision,
        present=calendar_epoch(native),
        conversion="program normalized: " + json.dumps(native, sort_keys=True),
    )
    if x.form == "point":
        return TimeValue(value=x.value, **base)
    if x.form == "distribution":
        hpd95 = (
            [x.lower, x.upper]
            if x.interval_type == "hpd" and x.probability == 0.95 and x.lower is not None
            else None
        )
        return TimeValue(
            median=x.median,
            mean=x.mean,
            hpd95=hpd95,
            unknown=x.mean is None and x.median is None and hpd95 is None,
            **base,
        )
    if x.form == "bound":
        return TimeValue(
            earliest=x.value if x.relation in ("after", "since") else None,
            latest=x.value if x.relation in ("before", "by") else None,
            **base,
        )
    return TimeValue(earliest=x.lower, latest=x.upper, open_end=x.ongoing, **base)


def extract_system(family: str) -> str:
    g = glotto.load()
    scope = g.rows.get(family, {}).get("name", family)
    return (
        "You extract source-specific chronological evidence, including relative arguments, for a provenance registry. "
        "Preserve what the source dates, its literal entity label and sampling scope. "
        "Dates of population movements retain population subjects and population_event. "
        "Archaeological layers, artifacts, vegetation and climate have their own literal subjects; "
        "they are not population events merely because the paper discusses speakers. "
        "Use unresolved subject_type and unspecified event when the available labels do not fit; explain the missing category in a gap. "
        "Record a proposed population or language association separately as a hypothesis, never as a language crown. "
        "Ensure event and subject fields agree with method_notes; a note denying a population date cannot justify population_event. "
        "Regional shift, last reported speaker, native transmission end and literary persistence are separate events; "
        "retain the geographic scope and competing linguistic identifications. "
        "(3) conventional_period means a named stage of one language (Old X, Middle X); historical processes are not periods. "
        "(4) Preserve relative chronological constraints without inventing calendar years: use event relative_constraint, source_values form unknown, and relative_constraints. "
        "Keep event, method, and epistemic_role separate. Calibration priors are not posterior results or earliest attestations. "
        "Give subject_type and source-specific subject_id (for clades list source taxon labels in sampled_taxa; do not invent missing taxa); "
        "a catalog identifier is only a proposed mapping. Return relative chronological arguments in paper_summary "
        "even if there is no numeric date. Never infer a language's birth or death from an attestation or tree node. "
        "Every time must include source_values in original units; source-stated present stays null when unstated. "
        "A separate conventional BP interpretation may use 1950; retain unknown age scale when calibration status is unclear. "
        "Bare 'years ago' uses unit years_ago (thousands: ka_ago), with present null unless explicitly defined and no BP convention. Preserve its numeric value even when calendar conversion is impossible. "
        "For 'third millennium BCE' use unit millennium_BCE, form event_range, lower=upper=3; "
        "for 'by the 10th century BCE' use unit century_BCE, form bound, value=10, relation=by. "
        "Never encode an ordinal as a year or invent expanded endpoints; programmatic conversion preserves the whole period. "
        "Elapsed quantities such as '300 years longer' are not calendar dates or year-zero spans; preserve wording in a structured gap. "
        "Each claim must carry a verbatim quote copied exactly from the text; claims with unverified quotations remain quarantined. "
        "Extract before catalog mapping: leave glottocode null and preserve the paper's literal subject and sample. "
        "The research family is a search scope, not evidence that a dated clade equals a catalog entity. "
        "The text may be followed by DATASET FILE sections: the paper's deposited data "
        "(README files, calibration or language tables, supplementary PDFs). Treat them as part of the source: extract the "
        "dated claims they contain (a calibration table row is a claim), quote the row or line verbatim, and set locator to "
        "the file name plus row/line. When the text mentions data or code deposited elsewhere (Zenodo, GitHub, OSF, Dryad, "
        "supplementary materials) that is NOT attached, list it in paper_summary so it can be fetched.\n"
        + KINDS_RULES
        + "\n"
        + CALENDAR_RULES
        + "\n\nResearch scope: "
        + scope
        + " ("
        + family
        + "). This does not restrict which relevant subjects the source may discuss."
    )


def extraction_input(c: dict, text: str) -> tuple[str, dict]:
    from .datasets import dataset_bundle
    import hashlib

    attachments, inventory = dataset_bundle(c)
    included = text[:900_000]
    coverage = {
        "version": 1,
        "scope": "supplied candidate only; not a complete study inventory",
        "main_text": {
            "available_chars": len(text),
            "included_chars": len(included),
            "status": "truncated"
            if len(included) < len(text)
            else "included"
            if text.strip()
            else "empty",
            "text_sha256": hashlib.sha256(text.encode()).hexdigest(),
            "included_text_sha256": hashlib.sha256(included.encode()).hexdigest(),
        },
        "attachments": inventory,
    }
    return included + attachments, coverage


def extract_user(c: dict, text: str) -> str:
    evidence, coverage = extraction_input(c, text)
    return _extract_prompt(c, evidence, coverage)


def _extract_prompt(c: dict, evidence: str, coverage: dict) -> str:
    return (
        f"Source: {c.get('title')} ({c.get('year')})\n\n"
        "INPUT COVERAGE (harness metadata, not source evidence; never quote as evidence):\n"
        + json.dumps(coverage, ensure_ascii=False, sort_keys=True)
        + "\nMissing/truncated content cannot establish absence of claims. Record access and context gaps in gaps, including when claims is empty. "
        "Keep conflicting article and attachment assertions separate and identify each file in locator. "
        "Record conflicts with separately quoted sides and a next action; a faithful quote does not settle the conflict. "
        "Tree files listed here require separate declared mechanical ingestion; their presence does not mean they were reviewed."
        "\n\nSOURCE EVIDENCE:\n" + evidence
    )


def prepare_extraction(family, c, text, system, backend, run, batch=False):
    from .model_requests import freeze_request
    from .retrieve import cached_text

    evidence, coverage = extraction_input(c, text)
    user = _extract_prompt(c, evidence, coverage)
    retrieved_sha = c.get("retrieved", {}).get("sha256") if cached_text(c) == text else None
    ref = freeze_request(
        "extract",
        family,
        c["source_id"],
        system,
        user,
        Extraction,
        backend,
        run=run,
        batch=batch,
        context={
            "evidence_text": evidence,
            "input_coverage": coverage,
            "retrieved_text_sha256": retrieved_sha,
            "retrieval_quality": c.get("retrieved", {}).get("quality", "unverified"),
        },
    )
    return user, ref


def extraction_issues(ext, text, coverage):
    """Source matching does not establish entailment or settle disagreement."""
    conflicts = []
    for conflict in ext.conflicts:
        row = conflict.model_dump()
        row["quote_matches"] = [verify_quote(side.quote, text) for side in conflict.sides]
        row["status"] = (
            "pending_semantic_review" if all(row["quote_matches"]) else "unverified_quote"
        )
        conflicts.append(row)
    return {
        "gaps": [g.model_dump() for g in ext.gaps],
        "conflicts": conflicts,
        "input_limits": [
            row
            for row in [coverage["main_text"], *coverage["attachments"]]
            if row["status"] != "included"
        ],
        "status": "model_proposals_and_input_limits; empty lists do not certify completeness",
    }


def _ingest_extraction(
    family: str, c: dict, res: LLMResult, request_ref: str, run: str, stats: dict
) -> None:
    from .io_utils import digest, atomic_text
    from .store import replace_extraction
    from .model_requests import load_request, record_response
    import hashlib

    g = glotto.load()
    ext: Extraction = res.parsed
    request = load_request(request_ref)
    if (request["role"], request["family"], request["item"]) != ("extract", family, c["source_id"]):
        raise ValueError("extraction request identity mismatch")
    response_ref = record_response(request_ref, res)
    prompt_digest = request["prompt_sha256"]
    context = request["context"]
    retrieved_input_sha = context["retrieved_text_sha256"]
    text, input_coverage = context["evidence_text"], context["input_coverage"]
    issues = extraction_issues(ext, text, input_coverage)
    for name in ("gaps", "conflicts", "input_limits"):
        stats[name] = stats.get(name, 0) + len(issues[name])
    claims: list[Claim] = []
    for i, ec in enumerate(ext.claims):
        gid = ec.glottocode if ec.glottocode in g.rows else None
        verified = verify_quote(ec.quote, text)
        native = (
            ec.time.source_values.model_dump(exclude_none=True) if ec.time.source_values else {}
        )
        try:
            numeric_ok = native_numbers_match(native, ec.quote)
            verified = verified and numeric_ok
            tv = _time_value(ec.time)
        except Exception as e:  # noqa: BLE001
            append_log(
                family,
                "extract_log.jsonl",
                {"source": c["source_id"], "run": run, "dropped": ec.model_dump(), "error": str(e)},
            )
            continue
        prefix = gid or family
        claims.append(
            Claim(
                id=f"clm_{prefix}_{digest({'claim': ec.model_dump(), 'source': c['source_id']})[:24]}",
                languoid=gid,
                languoid_name=ec.languoid_name,
                kind=ec.kind,
                node_role=ec.node_role,
                time=tv,
                source=c["source_id"],
                locator=ec.locator,
                quote=ec.quote,
                quote_verified=verified,
                method_notes=MethodNotes(
                    calibrations=ec.calibrations,
                    clock=ec.clock,
                    data=ec.data,
                    taxa=ec.taxa,
                    code_available=ec.code_available,
                    other=ec.method_notes,
                ),
                extracted_by={
                    "model": res.model,
                    "backend": res.backend,
                    "run": run,
                    "message_id": res.id,
                },
                evidence={
                    "schema_version": 2,
                    "subject_type": ec.subject_type,
                    "subject_id": ec.subject_id,
                    "event": ec.event,
                    "role": ec.epistemic_role,
                    "analysis_id": ec.analysis_id,
                    "result_id": ec.result_id,
                    "method": ec.method,
                    "sampled_taxa": ec.sampled_taxa,
                    "geographic_scope": ec.geographic_scope,
                    "identification_alternatives": ec.identification_alternatives,
                    "source_values": native,
                    "text_sha256": hashlib.sha256(text.encode()).hexdigest(),
                    "relative_constraints": ec.relative_constraints,
                    "reported_from": ec.reported_from,
                    "semantic_status": "pending",
                    "mapping_status": "pending",
                    "retrieval_quality": context["retrieval_quality"],
                    "retrieved_text_sha256": retrieved_input_sha,
                },
                notes=f"secondary: reported from {ec.reported_from}" if ec.reported_from else None,
            )
        )
        stats["claims"] += 1
        stats["verified" if verified else "unverified"] += 1
        stats["placed"] += 1 if gid else 0
    text_hash = hashlib.sha256(text.encode()).hexdigest()
    from .config import CACHE_DIR

    atomic_text(os.path.join(CACHE_DIR, "texts", text_hash + ".txt"), text)
    replace_extraction(
        family,
        c["source_id"],
        claims,
        {
            "run": run,
            "model": res.model,
            "text_sha256": text_hash,
            "prompt_sha256": prompt_digest,
            "request_ref": request_ref,
            "response_ref": response_ref,
            "schema_sha256": digest(request["schema"]),
            "input_coverage": input_coverage,
            "issues": issues,
            "output": ext.model_dump(),
        },
    )
    c["extracted"] = {
        "run": run,
        "claims": len(claims),
        "summary": ext.paper_summary,
        "at": now(),
        "text_sha256": text_hash,
        "prompt_sha256": prompt_digest,
        "input_kind": "cached_text" if retrieved_input_sha else "abstract_or_dataset_only",
        "retrieved_text_sha256": retrieved_input_sha,
        "input_coverage": input_coverage,
        "issues": issues,
        "request_ref": request_ref,
        "response_ref": response_ref,
        "schema_sha256": digest(request["schema"]),
        **res.log_fields(),
    }
    append_log(
        family,
        "extract_log.jsonl",
        {
            "source": c["source_id"],
            "run": run,
            "claims": len(claims),
            "summary": ext.paper_summary,
            "request_ref": request_ref,
            "response_ref": response_ref,
            "issues": issues,
            **res.log_fields(),
        },
    )


def _extraction_todo(cands: list[dict], run: str, family: str) -> list[tuple[dict, str]]:
    from .io_utils import digest
    from .llm_backends import strict_schema

    out = []
    for c in cands:
        if (
            not c.get("screen")
            or c["screen"].get("decision") != "include"
            or not c.get("source_id")
        ):
            continue
        text = text_for(c)
        if (
            c.get("extracted", {}).get("run") == run
            and c["extracted"].get("schema_sha256") == digest(strict_schema(Extraction))
            and c["extracted"].get("prompt_sha256")
            == digest({"system": extract_system(family), "user": extract_user(c, text or "")})
        ):
            continue
        if text or c.get("datasets"):
            out.append((c, text or ""))
    return out


def extract_family(family: str, run: str, batch: bool = False) -> dict:
    path = os.path.join(family_dir(family), "candidates.json")
    with open(path, encoding="utf-8") as f:
        cands = json.load(f)
    system = extract_system(family)
    todo = _extraction_todo(cands, run, family)
    stats = {
        "papers": len(todo),
        "claims": 0,
        "verified": 0,
        "unverified": 0,
        "placed": 0,
        "errors": 0,
        "submitted": 0,
        "tree_claims": 0,
        "gaps": 0,
        "conflicts": 0,
        "input_limits": 0,
    }
    # mechanical pass first: dated tree files deposited with the papers
    from .datasets import ingest_trees

    for c, _ in todo:
        if c.get("datasets"):
            stats["tree_claims"] += ingest_trees(family, c, run)["claims"]
    backend = get_backend(role="extract")
    if batch:
        from .batch import submit

        items, refs = [], {}
        for c, text in todo:
            user, ref = prepare_extraction(family, c, text, system, backend, run, batch=True)
            items.append((c["source_id"], system, user))
            refs[c["source_id"]] = ref
        stats["submitted"] = submit(
            family, "extract", items, Extraction, backend, run=run, request_refs=refs
        )
        return stats
    for c, text in todo:
        user, request_ref = prepare_extraction(family, c, text, system, backend, run)
        try:
            res = backend.complete("extract", system, user, Extraction, session_id=family)
        except Exception as e:  # noqa: BLE001
            stats["errors"] += 1
            from .model_requests import record_failure, recover_failed_extraction

            failure_ref = record_failure(request_ref, e, getattr(backend, "last_failure", None))
            recovery_ref = recover_failed_extraction(request_ref, failure_ref)
            append_log(
                family,
                "extract_log.jsonl",
                {
                    "source": c["source_id"],
                    "run": run,
                    "request_ref": request_ref,
                    "failure_ref": failure_ref,
                    "recovery_ref": recovery_ref,
                    "error": str(e),
                },
            )
            if recovery_ref:
                c["extraction_recovery"] = {
                    "request_ref": request_ref,
                    "failure_ref": failure_ref,
                    "recovery_ref": recovery_ref,
                    "status": "incomplete_proposals_only",
                    "run": run,
                }
                atomic_json(path, cands)
            continue
        _ingest_extraction(family, c, res, request_ref, run, stats)
        atomic_json(path, cands)
    return stats


# --------------------------------------------------------------------------- #
# synthesis (on top of the rules)
# --------------------------------------------------------------------------- #


class LanguoidReview(BaseModel):
    subject_id: str = Field(
        description="Exact source-specific subject ID from the supplied payload; never replace with a catalog ID"
    )
    glottocode: str | None = Field(
        default=None,
        description="Optional catalog context only, not the reviewed subject's identity",
    )
    rationale: str = Field(
        description="Review source fidelity, scope, dependencies and limits, referencing claim IDs; no adopted values are presumed"
    )
    open_questions: list[str] = Field(default_factory=list)
    override_crown: str | None = Field(
        default=None,
        description="Deprecated compatibility field: leave null; proposals never adopt evidence",
    )
    recommended_claim_ids: list[str] = Field(default_factory=list)


class FamilyReview(BaseModel):
    reviews: list[LanguoidReview]


SYNTH_SYSTEM = (
    "You review a chronological evidence registry. You receive claims grouped by source-specific subject. "
    "No rules-selected dates or language lifespans exist. Propose review actions only; leave override_crown null. "
    "Assess textual fidelity, estimand, entity scope, source completeness, analysis conditions and dependency. "
    "Separate evidence quality, agreement and conditional statistical uncertainty. Treat methodological demonstrations "
    "as such; critiques and replies are arguments to assess, not additional date estimates or established refutations. "
    "Compare complete scenarios; never join endpoints or marginal node ages from different analyses. "
    "Preserve unresolved questions and unknown dates; recommend claim IDs only with explicit reasons. "
    + KINDS_RULES
    + "\n"
    + CALENDAR_RULES
)


def _apply_review(records: dict, claims: dict, review: FamilyReview, run: str) -> int:
    """Legacy API: validate suggestions, but never mutate adopted scientific data."""
    for rv in review.reviews:
        for cid in [rv.override_crown, *rv.recommended_claim_ids]:
            if cid and cid not in claims:
                raise ValueError("review references unknown claim " + cid)
    return 0


def synthesize_family(family: str, run: str, max_languoids: int = 80) -> dict:
    from .registry import propose_review

    return propose_review(family, run, max_languoids)


# --------------------------------------------------------------------------- #
# backend parity probe
# --------------------------------------------------------------------------- #


class Probe(BaseModel):
    languoid: str
    kind: Literal["attestation_first", "conventional_period", "phylo_node_age"]
    earliest: int
    latest: int | None
    quote: str


PROBE_SYSTEM = "You extract one dated claim from a sentence. " + KINDS_RULES + "\n" + CALENDAR_RULES
PROBE_USER = (
    'Glottolog nodes: olde1238 Old English (language).\n\nText: "Old English is conventionally dated from about 450 to 1100 CE."\n'
    "Return the single conventional_period claim with the verbatim quote."
)


def llm_check(backends: list[str]) -> list[dict]:
    """Run one identical structured probe through each backend (the sync check)."""
    from .llm_backends import get_backend as gb

    out = []
    for name in backends:
        try:
            res = gb(name).complete("screen", PROBE_SYSTEM, PROBE_USER, Probe)
            out.append(
                {"backend": name, "ok": True, "parsed": res.parsed.model_dump(), **res.log_fields()}
            )
        except Exception as e:  # noqa: BLE001
            out.append({"backend": name, "ok": False, "error": str(e)[:500]})
    return out
