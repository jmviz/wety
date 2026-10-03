"""Narrow numerical checks of recovered Uralic supplements, not dating inference.

Uses only the standard library and pinned source bytes. The Mann–Kendall check
implements the reported one-sided normal approximation with continuity correction;
it does not execute the deposited R script or verify its historical environment.
"""

from collections import Counter
import hashlib
import json
import math
from pathlib import Path, PurePosixPath
from statistics import NormalDist
import zipfile
from xml.etree import ElementTree as ET

from .config import DATA_DIR
from workflow_tools.paths import resolve

NS = {"s": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"


def xlsx_cells(path):
    """Read stored cell values with coordinates; never recalculate/modify a source.

    Formula cells expose cached values separately from their original formula.
    This intentionally small reader serves the pinned source workbooks only.
    """
    with zipfile.ZipFile(path) as z:
        strings = []
        if "xl/sharedStrings.xml" in z.namelist():
            strings = ["".join(e.itertext()) for e in ET.fromstring(z.read("xl/sharedStrings.xml"))]
        rels = {
            e.attrib["Id"]: e.attrib["Target"]
            for e in ET.fromstring(z.read("xl/_rels/workbook.xml.rels"))
        }
        result = {}
        for sheet in ET.fromstring(z.read("xl/workbook.xml")).findall("s:sheets/s:sheet", NS):
            target = rels[sheet.attrib["{" + R + "}id"]]
            member = (
                target.lstrip("/") if target.startswith("/") else str(PurePosixPath("xl") / target)
            )
            cells = {}
            for c in ET.fromstring(z.read(member)).findall(".//s:sheetData/s:row/s:c", NS):
                v = c.find("s:v", NS)
                value = v.text if v is not None else None
                kind = c.attrib.get("t")
                if kind == "s" and value is not None:
                    value = strings[int(value)]
                elif kind == "inlineStr":
                    value = "".join(c.find("s:is", NS).itertext())
                elif value is not None and kind not in ("str", "e", "b"):
                    value = float(value)
                    if value.is_integer():
                        value = int(value)
                formula = c.find("s:f", NS)
                if value is not None or formula is not None:
                    cells[c.attrib["r"]] = {
                        "value": value,
                        "formula": formula.text if formula is not None else None,
                    }
            result[sheet.attrib["name"]] = cells
        return result


def mann_kendall(values):
    n = len(values)
    s = sum((b > a) - (b < a) for i, a in enumerate(values) for b in values[i + 1 :])
    ties = sum(t * (t - 1) * (2 * t + 5) for t in Counter(values).values())
    variance = (n * (n - 1) * (2 * n + 5) - ties) / 18
    if variance <= 0:
        raise ValueError("trend statistic undefined")
    z = (s - ((s > 0) - (s < 0))) / math.sqrt(variance)
    return {"n": n, "S": s, "tau": s / (n * (n - 1) / 2), "p_less": NormalDist().cdf(z)}


def check_supplements(packet_path=None):
    packet_path = Path(
        packet_path
        or Path(DATA_DIR) / "_research/packets/uralic-supplements-2026-09-19/packet.json"
    )
    packet = json.loads(packet_path.read_text())
    files = {}
    for s in packet["source_reviews"]:
        raw = resolve(s["raw_path"]).read_bytes()
        if hashlib.sha256(raw).hexdigest() != s["raw_sha256"]:
            raise ValueError("source hash mismatch " + s["key"])
        files[s["key"]] = resolve(s["raw_path"])
    cells = xlsx_cells(files["grunthal-data"])["Branches"]
    rows = [[cells[f"{col}{row}"]["value"] for col in "ABCDEFGH"] for row in range(2, 11)]
    # Published Table S4.2, visually checked against supplement PDF page 20.
    expected = [
        (14, 0.500, 0.946, 18, 0.500, 0.962),
        (14, 0.500, 0.946, 14, 0.389, 0.912),
        (10, 0.357, 0.867, 6, 0.167, 0.699),
        (10, 0.357, 0.867, -6, -0.167, 0.301),
        (0, 0.000, 0.500, -12, -0.333, 0.126),
    ]
    comparisons = []
    for i in range(1, 6):
        if len({r[i] for r in rows}) != len(rows):
            raise ValueError("tied distances require review")
        ordered = sorted(rows, key=lambda r: r[i])
        pu = mann_kendall([r[6] for r in ordered if r[0] != "Samoyedic"])
        ii = mann_kendall([r[7] for r in ordered])
        observed = (
            pu["S"],
            round(pu["tau"], 3),
            round(pu["p_less"], 3),
            ii["S"],
            round(ii["tau"], 3),
            round(ii["p_less"], 3),
        )
        comparisons.append(
            {
                "origin": cells[f"{chr(65 + i)}1"]["value"],
                "PU": pu,
                "II": ii,
                "reported": expected[i - 1],
                "passed": observed == expected[i - 1],
            }
        )
    # Bind the main-article parameters to the earlier reviewed source packet.
    from .packets import validate_packet

    prior_packet = Path(DATA_DIR) / "_research/packets/uralic-2026-09-19/packet.json"
    if validate_packet(prior_packet)["errors"]:
        raise ValueError("prior source packet is invalid")
    prior_bytes = (prior_packet.parent / "claims.json").read_bytes()
    prior = next(
        c
        for c in json.loads(prior_bytes)["claims"]
        if c["provenance"]["packet_key"] == "maurits-prior-permic"
    )
    original = prior["time"]["original"]
    params = original["raw_parameters"]
    if params["unit"] != "millennia":
        raise ValueError("unexpected prior parameter units")
    normal = NormalDist()
    offset, mu, sigma = params["offset"], params["log_mean"], params["log_sd"]
    permic = {
        "mode_years_bp": 1000 * (offset - math.exp(mu - sigma * sigma)),
        "equal_tailed_95_years_bp": [
            1000 * (offset - math.exp(mu + sigma * normal.inv_cdf(p))) for p in (0.975, 0.025)
        ],
        "positive_age_probability": normal.cdf((math.log(offset) - mu) / sigma),
    }
    permic["main_article_claim_id"] = prior["id"]
    permic["input_claims_sha256"] = hashlib.sha256(prior_bytes).hexdigest()
    permic["passed"] = round(permic["mode_years_bp"]) == original["reported_mode"] and [
        round(x) for x in permic["equal_tailed_95_years_bp"]
    ] == [original["lower"], original["upper"]]
    return {
        "scope": "Numerical consistency of reported supplements; not historical validation or execution of original R environment.",
        "source_hashes": {s["key"]: s["raw_sha256"] for s in packet["source_reviews"]},
        "grunthal_table_s4_2": comparisons,
        "maurits_permic": permic,
        "passed": all(c["passed"] for c in comparisons) and permic["passed"],
    }


def main():
    result = check_supplements()
    print(json.dumps(result, ensure_ascii=False, indent=2))
    raise SystemExit(not result["passed"])


if __name__ == "__main__":
    main()
