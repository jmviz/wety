"""Manifest-driven extraction. Never infer units, epoch, sample type or burn-in."""

from workflow_tools.paths import DATING_PRODUCT, DATING_GENERATIONS, DATING_HISTORY

from pathlib import Path
from workflow_tools.paths import REPO, historical
import hashlib
import json
import math
import statistics
from .config import DATA_DIR, DATING_DIR
from .io_utils import atomic_json, digest
from .evidence import EvidenceClaim, Subject, Temporal, Assessment, normalize_time
from .bootstrap.dplace import parse_nexus_trees, parse_newick, node_ages


def read_trees(path):
    text = Path(path).read_text()
    return (
        parse_nexus_trees(text)
        if "#nexus" in text[:200].lower()
        else [("tree", parse_newick(text))]
    )


def quantile(xs, q):
    xs = sorted(xs)
    p = q * (len(xs) - 1)
    lo = int(p)
    hi = min(lo + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (p - lo)


def summarize(manifest):
    required = (
        "id",
        "source_id",
        "study_id",
        "path",
        "sha256",
        "sample_type",
        "result_role",
        "unit",
        "unit_evidence",
        "taxa",
        "family",
        "methodology",
    )
    for k in required:
        if k not in manifest:
            raise ValueError("analysis manifest missing " + k)
    if manifest["unit"] not in ("BP", "ka_BP", "century_BP"):
        raise ValueError("tree lengths must be explicitly dated")
    if manifest["result_role"] not in ("posterior", "calibration_prior"):
        raise ValueError("explicit posterior/prior role required")
    path = historical(manifest["path"])
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != manifest["sha256"]:
        raise ValueError("tree hash mismatch")
    trees = read_trees(path)
    if not trees:
        raise ValueError("no trees")
    taxa = set(manifest["taxa"])
    if len(taxa) != len(manifest["taxa"]):
        raise ValueError("duplicate declared taxa")
    for _, root in trees:
        labels = [t.name for t in root.tips()]
        if len(labels) != len(set(labels)) or set(labels) != taxa:
            raise ValueError("tree taxon set differs from manifest")
    if manifest["sample_type"] == "posterior" and manifest["result_role"] != "posterior":
        raise ValueError("prior-only chains must use sample_type mcmc_samples, not posterior")
    posterior = manifest["sample_type"] in ("posterior", "mcmc_samples")
    if posterior:
        # No arbitrary thinning or default burn-in. Diagnostics must identify their data.
        d = manifest.get("diagnostics", {})
        if (
            not d.get("converged")
            or d.get("minimum_ess", 0) < 200
            or not d.get("trace_sha256")
            or not manifest.get("burnin_evidence")
        ):
            raise ValueError(
                "posterior requires trace-linked convergence/ESS and justified burn-in"
            )
        trace = historical(d.get("trace_path", "MISSING"))
        if (
            not trace.is_file()
            or hashlib.sha256(trace.read_bytes()).hexdigest() != d["trace_sha256"]
        ):
            raise ValueError("diagnostic trace missing or hash mismatch")
        if "burnin_trees" not in manifest:
            raise ValueError("explicit burnin_trees required")
        n = manifest["burnin_trees"]
        if not isinstance(n, int) or n < 0 or n >= len(trees):
            raise ValueError("invalid burn-in")
        trees = trees[n:]
        if len(trees) < 200:
            raise ValueError("too few posterior samples")
    elif manifest["sample_type"] != "annotated_summary" or len(trees) != 1:
        raise ValueError("summary manifest must describe exactly one annotated tree")
    values = {}
    annotations = {}
    parents = {}
    joint = {}
    for sample_index, (tree_name, root) in enumerate(trees):
        ages = node_ages(root)
        # Without annotation heights, reference tip ages must establish the time origin.
        if posterior:
            tip_ages = manifest.get("tip_ages")
            if not tip_ages or set(tip_ages) != taxa:
                raise ValueError("posterior requires explicit ages of every tip")
            offsets = [tip_ages[t.name] - ages[id(t)] for t in root.tips()]
            if max(offsets) - min(offsets) > manifest.get("height_tolerance", 1e-5):
                raise ValueError("inconsistent tip ages / branch units")
            offset = statistics.mean(offsets)
        else:
            offset = 0
        for n in root.walk():
            if not n.children:
                continue
            key = tuple(sorted(t.name for t in n.tips()))
            parents.setdefault(key, set()).add(
                tuple(sorted(t.name for t in n.parent.tips())) if n.parent else ()
            )
            annotations[key] = n.annotations
            if posterior:
                age = ages[id(n)] + offset
                values.setdefault(key, []).append(age)
                joint.setdefault(key, []).append(
                    {
                        "sample_index": sample_index,
                        "tree_name": tree_name,
                        "age": age,
                        "parent_taxa": sorted(t.name for t in n.parent.tips()) if n.parent else [],
                    }
                )
            else:
                if "height_median" not in n.annotations and "height" not in n.annotations:
                    # TreeAnnotator leaves clades below its posterior limit unannotated;
                    # those yield no claim. An unannotated root means an undated tree.
                    if n is root:
                        raise ValueError("summary root lacks height annotations")
                    continue
                values[key] = [n.annotations.get("height_median", n.annotations.get("height"))]
    out = []
    for taxa_key, xs in sorted(values.items()):
        native = {
            "form": "distribution",
            "unit": manifest["unit"],
            "present": manifest.get("present"),
            "precision": "century",
        }
        # A source without a stated epoch may carry a labeled conventional reading
        # (BP as AD 1950) and an explicit calendar age scale; never both epochs.
        for key in ("epoch_interpretation", "age_scale"):
            if manifest.get(key) is not None:
                native[key] = manifest[key]
        if posterior:
            native.update(
                median=statistics.median(xs),
                lower=quantile(xs, 0.025),
                upper=quantile(xs, 0.975),
                interval_type="equal_tailed",
                probability=0.95,
            )
            support = len(xs) / len(trees)
        else:
            ann = annotations[taxa_key]
            support = ann.get("posterior")
            if "height_median" in ann:
                native["median"] = ann["height_median"]
            if manifest.get("height_statistic") == "mean" and "height" in ann:
                native["mean"] = ann["height"]
            if "height_95%_HPD" in ann:
                native.update(
                    lower=ann["height_95%_HPD"][0],
                    upper=ann["height_95%_HPD"][1],
                    interval_type="hpd",
                    probability=0.95,
                )
            if not any(k in native for k in ("median", "mean", "lower")):
                raise ValueError("height summary statistic is unspecified")
        if any(
            not math.isfinite(native[k]) or native[k] < 0
            for k in ("mean", "median", "lower", "upper")
            if k in native
        ):
            raise ValueError("nonfinite or negative native age")
        if native.get("lower", 0) > native.get("upper", float("inf")):
            raise ValueError("reversed native interval")
        if support is not None and (not math.isfinite(support) or not 0 <= support <= 1):
            raise ValueError("invalid clade support")
        try:
            temporal = normalize_time(native)
            limitations = (
                [
                    "Calendar years read the source's BP as AD 1950 by convention; the source"
                    " states no epoch."
                ]
                if native.get("epoch_interpretation")
                else []
            )
        except ValueError:
            if manifest.get("present") is not None or manifest.get("epoch_interpretation"):
                raise
            temporal = Temporal(form="unknown", original=native)
            limitations = [
                "BP epoch unverified; source-native distribution retained without calendar conversion"
            ]
        limitations += [
            "Conditional on this analysis and sampled taxa; not a language lifespan.",
            "Catalog equivalence has not been established.",
        ]
        key = digest({"analysis": manifest, "taxa": taxa_key})
        out.append(
            {
                "id": "ev_tree_" + key,
                "subject": Subject(
                    id=manifest["id"] + ":clade:" + key[:16],
                    type="sampled_clade",
                    name="MRCA(" + ", ".join(taxa_key) + ")",
                    taxa=list(taxa_key),
                    mapping="exact",
                    mapping_basis="exact descendant set in hashed source tree",
                ).model_dump(),
                "event": "crown_divergence",
                "role": manifest["result_role"],
                "method": "phylogenetic",
                "time": temporal.model_dump(),
                "source_id": manifest["source_id"],
                "study_id": manifest["study_id"],
                "analysis_id": manifest["id"],
                "result_id": key,
                "assessment": Assessment(
                    textual="mechanical",
                    semantic="checked",
                    mapping="checked",
                    reviewer="manifest-driven parser",
                    rationale="Extracts published source-node annotations or declared posterior; does not assess historical truth.",
                    limitations=limitations,
                ).model_dump(),
                "status": "supported"
                if temporal.normalization == "program_verified"
                else "candidate",
                "provenance": {
                    "family": manifest["family"],
                    "manifest": manifest,
                    "node_support": support,
                    "age_conditioning": "conditional on this exact clade being present",
                    "samples": len(xs),
                    "native_annotations": annotations[taxa_key],
                    "parent_taxa_sets": [list(p) for p in sorted(parents[taxa_key])],
                    "joint_samples": joint.get(taxa_key, []),
                    "topology_uncertainty": (support < 1 or len(parents[taxa_key]) > 1)
                    if posterior
                    else None,
                    "topology_observation": "MCMC tree samples"
                    if posterior
                    else "one summary topology; alternatives not recoverable",
                },
            }
        )
    return [EvidenceClaim.model_validate(c).model_dump(mode="json") for c in out]


def ingest_manifests(source_id=None, family=None, studies=None):
    summary = {"analyses": 0, "claims": 0, "skipped": []}
    seen = set()
    for p in sorted((Path(DATA_DIR) / "_analyses").glob("*.json")):
        m = json.loads(p.read_text())
        if source_id and m["source_id"] != source_id:
            continue
        if family and m["family"] != family:
            continue
        if studies and m["study_id"] not in studies:
            continue
        key = (m["study_id"], m["sha256"])
        if key in seen:
            continue
        seen.add(key)
        try:
            claims = summarize(m)
        except (ValueError, KeyError, OSError) as e:
            summary["skipped"].append({"analysis": m.get("id"), "reason": str(e)})
            continue
        dest = Path(DATA_DIR) / "_evidence" / (m["id"] + ".json")
        if dest.exists():
            previous = json.loads(dest.read_text())
            if previous != claims:
                atomic_json(
                    Path(DATING_HISTORY) / "tree_ingestion" / (digest(previous) + ".json"), previous
                )
        atomic_json(dest, claims)
        summary["analyses"] += 1
        summary["claims"] += len(claims)
    if not summary["analyses"] and not summary["skipped"]:
        summary["skipped"].append(
            {"reason": "No explicit analysis manifests; no guessed tree dates imported."}
        )
    return summary
