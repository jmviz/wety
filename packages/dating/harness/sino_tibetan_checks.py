"""Audit retained ST files; no registry import, epoch inference, or convergence claim."""

import hashlib
import json
import math
from pathlib import Path
import statistics
import xml.etree.ElementTree as ET

from .bootstrap.dplace import node_ages
from .config import DATA_DIR
from .tree_ingest import read_trees
from workflow_tools.paths import resolve


def shortest_sample_interval(values, probability=0.95):
    """Smallest endpoint interval containing ceil(p*n) observations, not an ETI."""
    values = sorted(values)
    if not values or not 0 < probability <= 1:
        raise ValueError("nonempty sample and probability in (0,1] required")
    count = math.ceil(probability * len(values))
    return list(
        min(
            ((values[i], values[i + count - 1]) for i in range(len(values) - count + 1)),
            key=lambda pair: pair[1] - pair[0],
        )
    )


def check_deposits(packet_path=None):
    packet_path = Path(
        packet_path or Path(DATA_DIR) / "_research/packets/sino-tibetan-2026-09-20/packet.json"
    )
    packet = json.loads(packet_path.read_text())
    files = {}
    for source in packet["source_reviews"]:
        path = resolve(source["raw_path"])
        if hashlib.sha256(path.read_bytes()).hexdigest() != source["raw_sha256"]:
            raise ValueError("source hash mismatch " + source["key"])
        files[source["key"]] = path
    trees = read_trees(files["zhang2020-trees"])
    states = [int(name.removeprefix("STATE_")) for name, _ in trees]
    if states != list(range(0, 50_000_001, 50_000)):
        raise ValueError("unexpected deposited chain states")
    taxa = sorted(t.name for t in trees[0][1].tips())
    if len(taxa) != 131 or len(set(taxa)) != 131:
        raise ValueError("unexpected or duplicate taxa")
    # Figure 2 states units in thousands of years. Check ALL dated tips and
    # both basal descendants; do not infer dating semantics from root magnitude.
    sinitic = {"Beijing", "CHN", "Fuzhou", "Hakka", "Old_Chinese"}
    tip_ranges = {name: [float("inf"), -float("inf")] for name in taxa}
    root_ages = []
    for (name, root), state in zip(trees, states):
        tips = root.tips()
        if sorted(t.name for t in tips) != taxa:
            raise ValueError("taxon set changes across samples")
        ages = node_ages(root)
        if not any({t.name for t in c.tips()} == sinitic for c in root.children):
            raise ValueError("sample lacks the declared Sinitic basal split")
        for tip in tips:
            value = ages[id(tip)] * 1000
            low, high = tip_ranges[tip.name]
            tip_ranges[tip.name] = [min(low, value), max(high, value)]
        if state > 5_000_000:
            root_ages.append(ages[id(root)] * 1000)
    expected_tips = {"Old_Chinese": 2550, "Padam": 120, "Shaiyang": 120}
    for name, limits in tip_ranges.items():
        if any(abs(v - expected_tips.get(name, 0)) > 1e-6 for v in limits):
            raise ValueError("unexpected sampled tip age " + name)
    hpd = shortest_sample_interval(root_ages)
    mean = statistics.mean(root_ages)
    xml = ET.parse(files["zhang2019-xml"])
    priors = {}
    for element in xml.iter("distribution"):
        if element.attrib.get("spec", "").endswith("MRCAPrior"):
            prior = {
                "attributes": element.attrib,
                "taxa": [t.get("id") or t.get("idref") for t in element.findall("taxonset/taxon")],
            }
            normal = element.find("Normal")
            if normal is not None:
                prior["normal"] = {
                    p.get("name"): float(p.text) for p in normal.findall("parameter")
                }
            uniform = element.find("Uniform")
            if uniform is not None:
                prior["uniform_explicit_attributes"] = uniform.attrib
            priors[element.get("id")] = prior
    return {
        "scope": "Pinned-file consistency audit; not historical validation, a fresh inference, or verified convergence.",
        "source_hashes": {s["key"]: s["raw_sha256"] for s in packet["source_reviews"]},
        "zhang2020": {
            "deposited_trees": len(trees),
            "retained_trees": len(root_ages),
            "selection": "STATE > 5000000; matches reported 900 samples; no production import authorized",
            "mean_years_bp": mean,
            "sample_hpd_95_years_bp": hpd,
            "taxa": taxa,
            "tip_ranges_years_bp": tip_ranges,
            "sinitic_basal_split": "present in all deposited trees; constrained, not inferred support",
            "calendar_epoch": None,
            "convergence": "unverified; other chains and traces unavailable",
        },
        "zhang2019_xml_priors": priors,
        "passed": len(root_ages) == 900
        and round(mean) == 7983
        and [round(x) for x in hpd] == [4778, 11285],
    }


def main():
    result = check_deposits()
    print(json.dumps(result, ensure_ascii=False, indent=2))
    raise SystemExit(not result["passed"])


if __name__ == "__main__":
    main()
