"""Evidence registry, conservative migration, editorial decisions and publication.

Legacy claims are preserved verbatim, never silently promoted to verified evidence.
Publication is a deterministic snapshot; a content-addressed generation is the
canonical transaction, and _derived/CURRENT.json is its atomic commit pointer.
"""

from __future__ import annotations
from workflow_tools.paths import DATING_PRODUCT, DATING_GENERATIONS, DATING_HISTORY
import csv
import io
import json
from pathlib import Path
from workflow_tools.paths import REPO, IECOR_PRODUCT, historical
import shutil
from collections import Counter, defaultdict
from .config import DATA_DIR, CACHE_DIR, LANGCODES_DIR, DATING_DIR
from .io_utils import atomic_json, atomic_text, digest, locked
from .evidence import (
    EvidenceClaim,
    Subject,
    Temporal,
    Assessment,
    EvidenceSpan,
    Decision,
    normalize_time,
    evidence_digest,
)
from .store import all_claims, load_sources
from . import glotto

RULE_VERSION = "evidence-registry-2.1"
# Bump deliberately when adoption eligibility or evidence semantics change in a way
# the checks below would not catch. Every build re-checks each decision's eligibility
# with current code, and validation re-normalizes every program-verified time.
DECISION_RULES = 2
# 2 (2026-10-04): a parent code is matched against the stage it names, its Wiktionary
# varieties being dated as their own codes; a proto code's end may be a crown age.
EVENTS = {
    "attestation_first": "first_attestation",
    "attestation_last": "last_attestation",
    "extinction": "native_transmission_end",
    "conventional_period": "conventional_period",
}


def study_identity(source, sources):
    sid = source["id"]
    parent = sources.get(sid.split("_data_")[0], source)
    doi = parent.get("doi") or source.get("doi")
    return parent.get("study_id") or (
        "doi:" + doi.lower().removeprefix("https://doi.org/") if doi else "source:" + sid
    )


def legacy_time(c):
    t = c["time"]
    kw = {
        "precision": t.get("precision", "unknown"),
        "original": t,
        "normalization": "legacy_unverified",
    }
    if (c.get("evidence") or {}).get("source_values"):
        return normalize_time(c["evidence"]["source_values"])
    if t.get("median") is not None or t.get("mean") is not None or t.get("hpd95"):
        hp = t.get("hpd95")
        # Legacy posterior quantiles were incorrectly labeled HPD. Correct the TYPE,
        # retain raw field in original; never change underlying numbers during migration.
        interval_type = "equal_tailed" if "2.5–97.5" in t.get("conversion", "") else "unknown"
        return Temporal(
            form="distribution",
            mean=t.get("mean"),
            median=t.get("median"),
            lower=hp[0] if hp else None,
            upper=hp[1] if hp else None,
            interval_type=interval_type if hp else None,
            probability=0.95 if hp else None,
            **kw,
        )
    if t.get("value") is not None and t.get("earliest") is None and t.get("latest") is None:
        return Temporal(form="point", value=t["value"], **kw)
    lo, hi = t.get("earliest"), t.get("latest")
    if t.get("value") is not None:
        return Temporal(form="unknown", **kw)  # mixed legacy representations retained as raw only
    if lo is not None or hi is not None:
        if c["kind"] == "conventional_period" or c.get("node_role") == "interval":
            return Temporal(
                form="duration", lower=lo, upper=hi, ongoing=t.get("open_end", False), **kw
            )
        if lo is not None and hi is not None:
            return Temporal(form="event_range", lower=lo, upper=hi, **kw)
        # Missing endpoint does not tell us if this is an event bound or a duration.
    return Temporal(form="unknown", **kw)


def migrate_claim(c, sources, g):
    ev = c.get("evidence") or {}
    src = sources[c["source"]]
    limitations = (
        ["legacy claim requires semantic and entity-scope review"]
        if ev.get("schema_version") != 2
        else ["extracted assertion requires semantic and entity-scope review"]
    )
    try:
        temporal = legacy_time(c)
    except (ValueError, KeyError, TypeError) as e:
        temporal = Temporal(form="unknown", original=ev.get("source_values") or c["time"])
        limitations.append("invalid legacy temporal form: " + str(e))
    event = EVENTS.get(c["kind"], "unspecified")
    if c.get("node_role") in ("crown", "stem"):
        event = c["node_role"] + "_divergence"
    if c.get("node_role") == "interval":
        event = "formation"
    role = (
        "posterior"
        if c["kind"] == "phylo_node_age"
        else "convention"
        if c["kind"] == "conventional_period"
        else "observation"
        if c["kind"] in EVENTS
        else "hypothesis"
    )
    if "tip-date prior" in (c.get("notes") or ""):
        event, role = "documentary_sampling", "calibration_prior"
    if (c.get("notes") or "").startswith("secondary:"):
        role = "secondary_report"
    if (
        "bootstrap.glottolog" in c.get("extracted_by", {}).get("program", "")
        and c["kind"] == "conventional_period"
    ):
        event, role = "documentary_sampling", "observation"
        limitations.append(
            "legacy documentation window is not a conventional language-stage period"
        )
    event = ev.get("event", event)
    role = ev.get("role", role)
    gid = c.get("languoid")
    typ = ev.get("subject_type") or (
        "sampled_clade" if c["kind"] == "phylo_node_age" else "catalog" if gid else "unresolved"
    )
    subject_id = ev.get("subject_id") or (
        "legacy-subject:" + c["id"]
        if typ == "sampled_clade"
        else "glottolog:" + gid
        if gid
        else "unresolved:" + c["id"]
    )
    name = c.get("languoid_name") or (g.rows.get(gid) or {}).get("name") or subject_id
    textual = "unverified"
    spans = []
    textsha = ev.get("text_sha256")
    textpath = Path(CACHE_DIR) / "texts" / ((textsha or "") + ".txt")
    if textsha and textpath.exists() and c.get("quote"):
        text = textpath.read_text()
        pos = text.find(c["quote"])
        import hashlib

        if pos >= 0 and hashlib.sha256(text.encode()).hexdigest() == textsha:
            textual = "exact_span"
            spans = [
                EvidenceSpan(
                    text_sha256=textsha,
                    start=pos,
                    end=pos + len(c["quote"]),
                    quote=c["quote"],
                    locator=c.get("locator"),
                )
            ]
    if c.get("quote_verified") is False:
        textual = "failed"
    if "curated" in c.get("extracted_by", {}).get("program", ""):
        textual = "paraphrase"
        limitations.append("handbook summary is not a verified quotation")
    if "dplace" in c.get("extracted_by", {}).get("program", "") or "ingest_trees" in c.get(
        "extracted_by", {}
    ).get("program", ""):
        limitations.append(
            "legacy tree mapping/units/analysis and summary statistic require re-ingestion with manifest"
        )
    if temporal.normalization != "program_verified":
        limitations.append("legacy date conversion not independently verified")
    if role == "secondary_report":
        limitations.append("secondary report is not independent confirmation")
    cid = "ev_" + digest(c)
    return EvidenceClaim(
        id=cid,
        legacy_id=c["id"],
        subject=Subject(
            id=subject_id,
            type=typ,
            name=name,
            catalog_id=gid,
            source_label=c.get("languoid_name"),
            taxa=ev.get("sampled_taxa", []),
            geographic_scope=ev.get("geographic_scope"),
            identification_alternatives=ev.get("identification_alternatives", []),
        ),
        event=event,
        role=role,
        method=ev.get("method") or c["kind"],
        time=temporal,
        source_id=c["source"],
        study_id=study_identity(src, sources),
        analysis_id=ev.get("analysis_id"),
        result_id=ev.get("result_id"),
        reported_from=ev.get("reported_from") or c.get("notes"),
        spans=spans,
        assessment=Assessment(textual=textual, limitations=limitations),
        status="quarantined" if textual == "failed" or temporal.form == "unknown" else "candidate",
        provenance={"legacy": c, "migration": RULE_VERSION},
    )


def context_hashes():
    import hashlib

    files = (
        list(Path(DATING_DIR, "harness").rglob("*.py"))
        + list((REPO / "packages/workflow_tools").glob("*.py"))
        + [REPO / "uv.lock", REPO / "pyproject.toml"]
    )
    files += [
        Path(LANGCODES_DIR, n)
        for n in (
            "CURRENT.json",
            "glottolog_langcodes.csv",
            "wiktionary_langcodes.json",
            "wiktionary_families.json",
        )
    ]
    files += [
        IECOR_PRODUCT / n
        for n in (
            "iecor_languages.csv",
            "iecor_matches_receipt.json",
            "iecor_wiktionary_matches.json",
        )
    ]
    return {
        str(p.relative_to(REPO)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(files)
        if p.is_file()
    }


def load_evidence():
    sources = load_sources()
    g = glotto.load()
    result = []
    for fam, cs in sorted(all_claims().items()):
        for c in cs:
            item = migrate_claim(c, sources, g)
            item.provenance["family"] = fam
            result.append(item.model_dump(mode="json"))
    # Explicit version-2 records (e.g. mechanically checked deposited analyses).
    for p in sorted((Path(DATA_DIR) / "_evidence").glob("*.json")):
        for raw in json.loads(p.read_text()):
            result.append(EvidenceClaim.model_validate(raw).model_dump(mode="json"))
    ids = [c["id"] for c in result]
    if len(ids) != len(set(ids)):
        raise ValueError("duplicate evidence IDs")
    return sorted(result, key=lambda c: c["id"])


def validate_span(c):
    import hashlib

    errors = []
    for s in c["spans"]:
        path = Path(CACHE_DIR) / "texts" / (s["text_sha256"] + ".txt")
        if not path.exists():
            errors.append("missing evidence text " + s["text_sha256"])
            continue
        raw = path.read_bytes()
        text = raw.decode()
        if (
            hashlib.sha256(raw).hexdigest() != s["text_sha256"]
            or text[s["start"] : s["end"]] != s["quote"]
        ):
            errors.append("evidence span/hash mismatch " + c["id"])
    return errors


def validate_bundle(bundle):
    errors = []
    claims = {}
    sources = bundle["sources"]
    for key in bundle.get("source_conflicts") or []:
        errors.append(
            f"source {key} was registered differently by concurrent sessions; "
            "resolve with `dating ledger resolve-source`"
        )
    checked_analyses = {}
    reproduced = {}
    if bundle.get("pilot_coverage", {}).get("packet_errors"):
        errors.append(
            "invalid editorial source packet: " + str(bundle["pilot_coverage"]["packet_errors"])
        )
    from .research import validate_graph

    errors += validate_graph(bundle.get("research_graph", {}))
    research = bundle.get("research_graph", {})
    analysis_owners = {
        (r["source"], r["target"])
        for r in research.get("relations", [])
        if r["relation"] == "analysis_of"
    }
    for c in bundle["claims"]:
        try:
            EvidenceClaim.model_validate(c)
        except ValueError as e:
            errors.append(c.get("id", "?") + ": " + str(e))
        if c["id"] in claims:
            errors.append("duplicate evidence ID " + c["id"])
        claims[c["id"]] = c
        if c["source_id"] not in sources:
            errors.append("missing source " + c["source_id"])
        if c["status"] == "supported":
            errors += validate_span(c)
            if c["assessment"]["textual"] == "exact_span" and any(
                s["text_sha256"] != sources.get(c["source_id"], {}).get("sha256")
                for s in c["spans"]
            ):
                errors.append("supported span is not bound to registered source " + c["id"])
            if (
                c["role"] == "posterior"
                and ("analysis:" + str(c["analysis_id"]), c["study_id"]) not in analysis_owners
            ):
                errors.append(
                    "supported posterior lacks registered study/analysis relation " + c["id"]
                )
        if c["time"]["normalization"] == "program_verified":
            try:
                if normalize_time(c["time"]["original"]).model_dump() != c["time"]:
                    errors.append("temporal normalization mismatch " + c["id"])
            except (ValueError, KeyError, TypeError) as e:
                errors.append("unreproducible temporal normalization " + c["id"] + ": " + str(e))
        m = c.get("provenance", {}).get("manifest")
        if m and m["id"] in checked_analyses and checked_analyses[m["id"]] != m:
            errors.append("conflicting embedded analysis manifest " + m["id"])
        if m and m["id"] not in checked_analyses:
            checked_analyses[m["id"]] = m
            import hashlib

            path = historical(m["path"])
            if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != m["sha256"]:
                errors.append("analysis input missing or changed: " + m["id"])
            manifest_path = Path(DATA_DIR) / "_analyses" / (m["id"] + ".json")
            if not manifest_path.exists() or json.loads(manifest_path.read_text()) != m:
                errors.append("analysis manifest changed; re-ingest: " + m["id"])
        if m and c["assessment"]["textual"] == "mechanical":
            if m["id"] not in reproduced:
                from .tree_ingest import summarize

                try:
                    reproduced[m["id"]] = {x["id"]: x for x in summarize(m)}
                except (ValueError, KeyError, OSError) as e:
                    errors.append("unreproducible mechanical evidence: " + str(e))
                    reproduced[m["id"]] = {}
            expected = reproduced[m["id"]].get(c["id"])
            if not expected or any(
                c.get(k) != expected.get(k)
                for k in (
                    "subject",
                    "event",
                    "role",
                    "method",
                    "time",
                    "source_id",
                    "study_id",
                    "analysis_id",
                    "result_id",
                    "provenance",
                )
            ):
                errors.append("mechanical result differs from source " + c["id"])
    for c in claims.values():
        if c["status"] == "supported" and c["event"] == "model_lineage_span":
            endpoints = [claims.get(i) for i in c.get("lineage_span_endpoints", [])]
            if len(endpoints) != 2 or any(
                not e or e["status"] != "supported" or e["analysis_id"] != c["analysis_id"]
                for e in endpoints
            ):
                errors.append(
                    "lineage span endpoints must belong to the same supported analysis " + c["id"]
                )
            elif not all(e.get("provenance", {}).get("joint_samples") for e in endpoints):
                errors.append(
                    "lineage span needs joint samples; marginal summaries are insufficient "
                    + c["id"]
                )
        if (
            c["status"] == "supported"
            and c["event"] == "relative_constraint"
            and not any(
                c["id"] in r.get("basis_claim_ids", []) for r in bundle.get("relations", [])
            )
        ):
            errors.append("supported relative claim lacks structured relation " + c["id"])
        for dep in c["dependencies"]:
            if dep not in claims:
                errors.append("missing dependency " + dep)
            elif c["status"] == "supported" and claims[dep]["status"] != "supported":
                errors.append("unsupported dependency " + dep)
    from .evidence import RelativeConstraint

    graph = defaultdict(list)
    strict_edges = []
    for raw in bundle.get("relations", []):
        try:
            relation = RelativeConstraint.model_validate(raw)
            refs = [relation.before_event, relation.after_event, *relation.basis_claim_ids]
            if any(ref not in claims for ref in refs):
                errors.append("relative constraint references missing evidence")
            if relation.relation in ("precedes", "not_after"):
                graph[relation.before_event].append(relation.after_event)
            if relation.relation == "precedes":
                strict_edges.append((relation.before_event, relation.after_event))
        except ValueError as e:
            errors.append("invalid relative constraint: " + str(e))
    # Non-strict cycles can express equal dates; a cycle containing a strict
    # precedence edge is contradictory. Overlap is not an ordering edge.
    for before, after in strict_edges:
        todo = [after]
        seen = set()
        while todo:
            node = todo.pop()
            if node == before:
                errors.append("strict relative chronology cycle: " + before)
                break
            if node in seen:
                continue
            seen.add(node)
            todo.extend(graph.get(node, []))
    # A dependency cycle is never a valid derivation.
    visited = set()
    active = set()

    def visit(cid):
        if cid in active:
            errors.append("dependency cycle " + cid)
            return
        if cid in visited or cid not in claims:
            return
        active.add(cid)
        for dep in claims[cid]["dependencies"]:
            visit(dep)
        active.remove(cid)
        visited.add(cid)

    for cid in claims:
        visit(cid)
    from .evidence import CatalogRecord, CodeView

    for r in bundle["records"].values():
        try:
            CatalogRecord.model_validate(r)
        except ValueError as e:
            errors.append(str(e))
        if any(i not in claims for i in r["evidence_ids"]):
            errors.append("missing record evidence")
    for code, e in bundle["view"].items():
        try:
            CodeView.model_validate(e)
        except ValueError as err:
            errors.append(str(err))
        if any(
            i not in claims
            for k in ("evidence_ids", "catalog_context", "ancestral_context")
            for i in e[k]
        ):
            errors.append("missing view evidence")
        if e.get("spoken") is not None:
            errors.append("generic spoken interval forbidden: " + code)
        for x in e["chronology"]:
            c = claims.get(x["claim_id"])
            if (
                not c
                or c["status"] != "supported"
                or c["subject"]["id"] != "wikt:" + code
                or c["subject"]["mapping"] != "exact"
            ):
                errors.append("unsafe code-specific chronology " + code)
            elif x != chronology_entry(c):
                errors.append("chronology differs from evidence " + code)
    return errors


def chronology_entry(c):
    return {
        **{
            k: c[k]
            for k in (
                "event",
                "role",
                "method",
                "time",
                "source_id",
                "study_id",
                "analysis_id",
                "result_id",
            )
        },
        "claim_id": c["id"],
        "limitations": c["assessment"]["limitations"],
    }


def default_scope_error(claim, graph):
    if claim["role"] == "calibration_prior":
        return "calibration prior is not a historical default"
    for n in graph.get("nodes", []):
        if n["id"] in (claim["study_id"], "analysis:" + str(claim.get("analysis_id"))) and n.get(
            "scope"
        ) in ("methodological_demonstration", "sensitivity"):
            return "methodological/sensitivity result is contextual, not a historical default"
    return None


def is_proto_language(record):
    """A proto-language is a reconstructed common ancestor, whether Wiktionary files it
    as a reconstructed code (Proto-Germanic), an attested one (Proto-Norse, known from
    runic inscriptions) or a variety (Proto-Romance). Wiktionary names them all
    "Proto-…"; its `kind` only says how a code's entries are filed, and its family
    data names attested ancestors such as Latin, whose end a crown age must not date."""
    return (record or {}).get("name", "").startswith("Proto-")


def decision_claim_error(decision, claim, subject):
    """Why `claim` cannot be the default for `decision`, or None.

    A decision selects a supported claim about its own subject and event, with one
    exception: the end of a proto-language (`is_proto_language`) may be the crown
    age of a sampled clade, the split of its sampled descendants, when the decision
    says in `proto_mapping` why those taxa cover the family. The clade claim stays
    about the clade; the decision records the correspondence. A source that dates the
    family's breakup itself (a family age from lexicostatistics, an article's node age
    for a named family) gives a crown claim about the proto code, mapped by review."""
    if claim["status"] != "supported":
        return "claim not supported"
    if decision["event"] == "crown_divergence":
        if not is_proto_language(subject):
            return "a crown age can date only a proto-language (a code named Proto-…)"
        if (
            claim["event"] == "crown_divergence"
            and claim["subject"]["id"] == decision["subject_id"]
        ):
            return None
        if (
            claim["event"] != "crown_divergence"
            or claim["subject"]["type"] != "sampled_clade"
            or len(set(claim["subject"]["taxa"])) < 2
        ):
            return "a proto-language's end needs the crown age of a sampled clade"
        if not (decision.get("proto_mapping") or "").strip():
            return "proto_mapping must say why the sampled taxa cover the family"
        return None
    if claim["subject"]["id"] != decision["subject_id"] or claim["event"] != decision["event"]:
        return "ineligible claim or subject/event mismatch"
    return None


def proto_alternative_error(decision, by_id):
    """Why a crown decision's listed alternatives cannot be shown, or None. They may be
    sensitivity or other analyses (contextual results are shown, never defaults)."""
    alternatives = decision.get("proto_alternatives") or []
    if alternatives and decision["event"] != "crown_divergence":
        return "only a proto-language's crown decision lists alternatives"
    for alt in alternatives:
        claim = by_id.get(alt["claim_id"])
        if (
            not claim
            or claim["status"] != "supported"
            or claim["event"] != "crown_divergence"
            or claim["subject"]["type"] != "sampled_clade"
            or len(set(claim["subject"]["taxa"])) < 2
        ):
            return "alternative is not a supported crown age of a sampled clade: " + alt["claim_id"]
        if not (alt.get("proto_mapping") or "").strip():
            return "an alternative needs its own proto_mapping"
    return None


def decision_digest(claims, sources, catalog_record, subject_id):
    """What an editorial decision reviewed: every claim about the subject, their
    registered sources and the code's catalog record (its mapping links). Tooling
    edits elsewhere in the harness do not make a decision stale; a change to the
    subject's evidence or mapping does."""
    subject = sorted((c for c in claims if c["subject"]["id"] == subject_id), key=lambda c: c["id"])
    return digest(
        {
            "rules": DECISION_RULES,
            "subject_id": subject_id,
            "claims": subject,
            "sources": {c["source_id"]: sources.get(c["source_id"]) for c in subject},
            "catalog": catalog_record,
        }
    )


def bundle_decision_digest(bundle, subject_id, g=None):
    g = g or glotto.load()
    code = subject_id.removeprefix("wikt:")
    return decision_digest(bundle["claims"], bundle["sources"], g.wikt.get(code), subject_id)


def build_bundle():
    g = glotto.load()
    sources = load_sources()
    claims = load_evidence()
    context = context_hashes()
    context["sources_digest"] = digest(sources)
    manifests = {
        p.name: json.loads(p.read_text())
        for p in sorted((Path(DATA_DIR) / "_analyses").glob("*.json"))
    }
    context["analyses_digest"] = digest(manifests)
    from .research import load_graph

    research_graph = load_graph(claims, manifests)
    context["research_graph_digest"] = digest(research_graph)
    from .coverage import pilot_coverage

    coverage = pilot_coverage()
    context["pilot_coverage_digest"] = digest(coverage)
    relations_path = Path(DATA_DIR) / "_constraints/relations.json"
    relations = json.loads(relations_path.read_text()) if relations_path.exists() else []
    context["relative_constraints_digest"] = digest(relations)
    fingerprint = evidence_digest(claims, context)
    by_id = {c["id"]: c for c in claims}
    by_catalog = defaultdict(list)
    by_subject = defaultdict(list)
    for c in claims:
        if c["subject"]["catalog_id"]:
            by_catalog[c["subject"]["catalog_id"]].append(c["id"])
        by_subject[c["subject"]["id"]].append(c["id"])
    decisions = []
    rejected = []
    from . import ledger

    current, conflicts = ledger.current("decision")
    for key, competing in sorted(conflicts.items()):
        for raw in competing:
            rejected.append(
                {
                    "decision": raw,
                    "reason": "conflicting decisions from concurrent sessions for "
                    + key
                    + "; a new decision must replace them",
                }
            )
    for _, raw in sorted(current.items()):
        try:
            d = Decision.model_validate(raw)
            c = by_id[d.claim_id]
            if d.evidence_digest != decision_digest(
                claims, sources, g.wikt.get(d.subject_id.removeprefix("wikt:")), d.subject_id
            ):
                raise ValueError("stale decision: the subject's evidence or mapping changed")
            subject = g.wikt.get(d.subject_id.removeprefix("wikt:"))
            if reason := decision_claim_error(d.model_dump(), c, subject):
                raise ValueError(reason)
            if reason := proto_alternative_error(d.model_dump(), by_id):
                raise ValueError(reason)
            if reason := default_scope_error(c, research_graph):
                raise ValueError(reason)
            if any(x not in by_id or by_id[x]["status"] != "supported" for x in d.dependencies):
                raise ValueError("unknown or unsupported dependency")
            decisions.append(d.model_dump())
        except (ValueError, KeyError) as e:
            rejected.append({"decision": raw, "reason": str(e)})
    records = {}
    targets = set(by_catalog) | set(g.wikt_by_glotto) | set(g.proto_by_family_node)
    for gid in sorted(targets):
        if gid not in g.rows:
            continue
        refs = by_catalog[gid]
        records[gid] = {
            "schema_version": 2,
            "glottocode": gid,
            "name": g.rows[gid]["name"],
            "level": g.rows[gid]["level"],
            "status": g.status(gid),
            "evidence_ids": refs,
            "study_ids": sorted({by_id[i]["study_id"] for i in refs}),
            "chronology_policy": "No birth, extinction, or proto-period inferred from catalog ancestry.",
            "review_state": "needs_review"
            if any(by_id[i]["status"] != "supported" for i in refs)
            else "supported_evidence"
            if refs
            else "unknown",
        }
    view = {}
    for code, w in sorted(g.wikt.items()):
        gid = w.get("glottocode")
        exact = [
            by_id[i]
            for i in by_subject["wikt:" + code]
            if by_id[i]["status"] == "supported" and by_id[i]["subject"]["mapping"] == "exact"
        ]
        chronology = [chronology_entry(c) for c in exact]
        ancestors = g.genetic_path(gid) if gid in g.rows else []
        context_ids = sorted(
            {
                i
                for a in ancestors
                for i in by_catalog[a]
                if by_id[i]["event"] in ("crown_divergence", "stem_divergence")
            }
        )
        view[code] = {
            "schema_version": 2,
            "code": code,
            "name": w["name"],
            "kind": w["kind"],
            "glottocode": gid,
            "mapping": {
                "relation": "catalog_link_only" if gid else "unresolved",
                "crossref_generation": g.crossref_generation,
                "links": w.get("links", []),
                "temporal_transfer_allowed": False,
            },
            "spoken": None,
            "chronology": chronology,
            "evidence_ids": by_subject["wikt:" + code],
            "catalog_context": by_catalog[gid] if gid else [],
            "ancestral_context": context_ids,
            "display_status": "documented_claims"
            if chronology
            else "no_verified_code_specific_chronology",
            "defaults": [d for d in decisions if d["subject_id"] == "wikt:" + code],
        }
    queue = []
    for c in claims:
        if c["status"] != "supported":
            queue.append(
                {
                    "claim_id": c["id"],
                    "legacy_id": c["legacy_id"],
                    "subject": c["subject"]["name"],
                    "family": c["provenance"].get("family"),
                    "priority": len(g.wikt_by_glotto.get(c["subject"]["catalog_id"], [])),
                    "status": c["status"],
                    "reasons": c["assessment"]["limitations"],
                }
            )
    queue.sort(key=lambda r: (-r["priority"], r["claim_id"]))
    bundle = {
        "schema_version": 2,
        "rule_version": RULE_VERSION,
        "input_digest": fingerprint,
        "input_hashes": context,
        "claims": claims,
        "relations": relations,
        "research_graph": research_graph,
        "pilot_coverage": coverage,
        "sources": sources,
        "records": records,
        "view": view,
        "review_queue": queue,
        "decisions": decisions,
        "rejected_decisions": rejected,
        "source_conflicts": sorted(ledger.current("source")[1]),
    }
    errors = validate_bundle(bundle)
    if errors:
        raise ValueError("\n".join(errors[:20]))
    return bundle


def snapshot_v1():
    dest = Path(DATING_HISTORY) / "pre_v2"
    if (dest / "MANIFEST.json").exists():
        return
    manifest = {}
    for path in sorted(Path(DATA_DIR).glob("*/*")):
        if not path.is_file() or path.suffix not in (".json", ".jsonl", ".csv", ".md"):
            continue
        if path.parent.name.startswith("_") and path.parent.name != "_derived":
            continue
        target = dest / path.relative_to(DATA_DIR)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
        import hashlib

        manifest[str(path.relative_to(DATA_DIR))] = hashlib.sha256(path.read_bytes()).hexdigest()
    atomic_json(dest / "MANIFEST.json", manifest)


def report(bundle):
    counts = Counter(c["status"] for c in bundle["claims"])
    roles = Counter(c["role"] for c in bundle["claims"])
    lines = [
        "Chronological evidence registry v2",
        "",
        f"Input digest: {bundle['input_digest']}",
        "",
        f"{len(bundle['claims'])} claims; {len(bundle['records'])} catalog records; {len(bundle['view'])} Wiktionary entities.",
        "Evidence states: " + json.dumps(dict(counts), sort_keys=True),
        "Evidence roles: " + json.dumps(dict(roles), sort_keys=True),
        f"{sum(bool(v['chronology']) for v in bundle['view'].values())} codes with verified code-specific claims. No inherited spoken starts.",
        f"{len(bundle['review_queue'])} claims require review; complete machine-readable queue: review_queue.json.",
        f"{len(bundle['rejected_decisions'])} stale/ineligible decisions withheld.",
        "",
        "Counts measure recovered evidence, not historical correctness. Publication fidelity benchmarks do not establish linguistic truth.",
        "",
        "| Priority | Claim | Subject | State | Reasons |",
        "|---|---|---|---|---|",
    ]
    for r in bundle["review_queue"]:
        safe = lambda x: str(x).replace("|", "/").replace("\n", " ")
        lines.append(
            "| "
            + " | ".join(
                map(
                    safe,
                    [
                        r["priority"],
                        r["claim_id"],
                        r["subject"],
                        r["status"],
                        "; ".join(r["reasons"]),
                    ],
                )
            )
            + " |"
        )
    return "\n".join(lines) + "\n"


def publish():
    with locked(Path(DATA_DIR) / "_registry"):
        snapshot_v1()
        bundle = build_bundle()
        generation = digest(bundle)
        root = Path(DATING_GENERATIONS) / generation
        root.mkdir(parents=True, exist_ok=True)
        atomic_json(root / "bundle.json", bundle)
        atomic_json(
            root / "manifest.json",
            {
                "generation": generation,
                "bundle_sha256": __import__("hashlib")
                .sha256((root / "bundle.json").read_bytes())
                .hexdigest(),
                "input_digest": bundle["input_digest"],
            },
        )
        # Compatibility mirrors contain the new contract; authoritative readers use CURRENT.
        for fam in __import__("harness.store", fromlist=["families_on_disk"]).families_on_disk():
            records = {
                gid: r
                for gid, r in bundle["records"].items()
                if glotto.load().top_family(gid) == fam
            }
            atomic_json(Path(DATA_DIR) / fam / "languoids.json", records)
        out = Path(DATING_PRODUCT)
        atomic_json(out / "wiktionary_dates.json", bundle["view"])
        atomic_json(out / "evidence.json", bundle["claims"])
        atomic_json(out / "review_queue.json", bundle["review_queue"])
        atomic_json(out / "rejected_decisions.json", bundle["rejected_decisions"])
        atomic_json(out / "pilot_coverage.json", bundle["pilot_coverage"])
        atomic_json(out / "research_graph.json", bundle["research_graph"])
        buf = io.StringIO()
        w = csv.writer(buf)
        w.writerow(["code", "name", "display_status", "chronology_json"])
        for e in bundle["view"].values():
            w.writerow(
                [
                    e["code"],
                    e["name"],
                    e["display_status"],
                    json.dumps(e["chronology"], ensure_ascii=False, sort_keys=True),
                ]
            )
        atomic_text(out / "wiktionary_dates.csv", buf.getvalue())
        atomic_text(out / "report.md", report(bundle))
        atomic_json(
            out / "CURRENT.json",
            {
                "schema_version": 2,
                "generation": generation,
                "bundle": str((root / "bundle.json").relative_to(REPO)),
            },
        )
        return {
            "generation": generation,
            "claims": len(bundle["claims"]),
            "review_required": len(bundle["review_queue"]),
            "codes": len(bundle["view"]),
        }


def current_bundle():
    root = REPO
    pointer = json.loads((Path(DATING_PRODUCT) / "CURRENT.json").read_text())
    path = root / pointer["bundle"]
    bundle = json.loads(path.read_text())
    if digest(bundle) != pointer["generation"]:
        raise ValueError("generation digest mismatch")
    return bundle


def propose_review(family, run, max_languoids=80):
    """Model commentary is a review proposal. It never changes evidence or decisions."""
    from .llm import FamilyReview, SYNTH_SYSTEM
    from .llm_backends import get_backend
    from .model_requests import freeze_request, record_response, record_failure

    if max_languoids <= 0:
        raise ValueError("max_languoids must be positive")
    bundle = current_bundle()
    claims = [c for c in bundle["claims"] if c["provenance"].get("family") == family]
    grouped = defaultdict(list)
    for c in claims:
        grouped[c["subject"]["id"]].append(c)
    payload = [{"subject": k, "claims": v} for k, v in sorted(grouped.items())[:max_languoids]]
    coverage = {
        "available_subjects": len(grouped),
        "submitted_subject_ids": [p["subject"] for p in payload],
        "omitted_subject_ids": sorted(grouped)[max_languoids:],
        "subject_limit": max_languoids,
    }
    if not payload:
        return {"reviewed": 0, "submitted": 0, "input_coverage": coverage, "adopted": 0}
    system = (
        SYNTH_SYSTEM
        + " Propose only. No dates are adopted. Use exact source-specific subject_id values. Explicitly discuss scope, dependency, prior/result role, and missing primary evidence. Coverage omissions have not been reviewed."
    )
    user = json.dumps({"subjects": payload, "input_coverage": coverage})
    backend = get_backend(role="synthesize")
    request_ref = freeze_request(
        "synthesize",
        family,
        family,
        system,
        user,
        FamilyReview,
        backend,
        run=run,
        context={"input_digest": bundle["input_digest"], "input_coverage": coverage},
    )
    try:
        res = backend.complete("synthesize", system, user, FamilyReview, session_id=family)
    except Exception as e:
        record_failure(request_ref, e, getattr(backend, "last_failure", None))
        raise
    response_ref = record_response(request_ref, res)
    validation = review_response_coverage(payload, res.parsed)
    proposal = {
        "input_digest": bundle["input_digest"],
        "run": run,
        "review": res.parsed.model_dump(),
        "request_ref": request_ref,
        "response_ref": response_ref,
        "input_coverage": coverage,
        "response_coverage": validation,
        **res.log_fields(),
    }
    path = Path(DATA_DIR) / "_reviews" / (digest(proposal) + ".json")
    atomic_json(path, proposal)
    return {
        "reviewed": len(validation["valid_subject_ids"]),
        "submitted": len(payload),
        "input_coverage": coverage,
        "response_coverage": validation,
        "proposal": str(path),
        "adopted": 0,
    }


def review_response_coverage(payload, review):
    """Report response coverage and out-of-scope suggestions, not review correctness."""
    allowed = {p["subject"]: {c["id"] for c in p["claims"]} for p in payload}
    counts = Counter(r.subject_id for r in review.reviews)
    valid, errors = [], []
    for r in review.reviews:
        if r.subject_id not in allowed:
            errors.append("unknown subject " + r.subject_id)
        elif counts[r.subject_id] != 1:
            errors.append("duplicate subject " + r.subject_id)
        elif r.override_crown is not None:
            errors.append("forbidden override " + r.subject_id)
        elif set(r.recommended_claim_ids) - allowed[r.subject_id]:
            errors.append("claim outside subject " + r.subject_id)
        else:
            valid.append(r.subject_id)
    return {
        "valid_subject_ids": valid,
        "unreviewed_subject_ids": sorted(set(allowed) - set(valid)),
        "errors": sorted(set(errors)),
        "scope": "structural coverage only; no semantic validation or adoption",
    }
