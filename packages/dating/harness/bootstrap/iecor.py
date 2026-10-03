"""IE-CoR tip-date calibrations (Heggarty et al. 2023, Science 381:eabg0818,
supplementary table; the local copy is the selected IE-CoR artifact, joined to
Wiktionary by packages/catalogs/iecor.py).

Each historical language carries the prior on the date of the sampled variety:
normal(mean, sd) or log-normal(mean, sd, offset), in years BP. That is the
date of the *lexical stage sampled*, not first attestation, so it is recorded
as `expert_estimate` with the prior's parameters in `notes`. No calendar conversion is attempted until epoch and parameterization are verified."""

from __future__ import annotations

import json
import hashlib
from pathlib import Path
import math
import os

from .. import glotto
from ..config import IECOR_DIR, now
from workflow_tools.paths import REPO
from ..schema import Claim, Source, TimeValue
from ..time_utils import from_bp
from . import claim_id, write_claims


SOURCE = Source(
    id="src_heggarty2023_iecor",
    type="dataset",
    title="IE-CoR: Indo-European Cognate Relationships database, language calibration table",
    authors=["Heggarty, Paul", "Anderson, Cormac", "Scarborough, Matthew", "et al."],
    year=2023,
    doi="10.1126/science.abg0818",
    url="https://iecor.clld.org",
    license="CC-BY-4.0",
    citation="Heggarty, P. et al. 2023. Language trees with sampled ancestors support a hybrid model for the origin of Indo-European languages. Science 381: eabg0818.",
)


def run() -> dict:
    g = glotto.load()
    path = os.path.join(IECOR_DIR, "iecor_wiktionary_matches.json")
    with open(path, encoding="utf-8") as f:
        rows = json.load(f)
    receipt = json.loads((Path(IECOR_DIR) / "iecor_matches_receipt.json").read_text())
    for name, h in receipt["files"].items():
        if hashlib.sha256((REPO / name).read_bytes()).hexdigest() != h:
            raise ValueError("stale IE-CoR proposal receipt: " + name)
    if receipt["crossref_generation"] != g.crossref_generation:
        raise ValueError("IE-CoR proposals need rebuilding")
    claims: list[Claim] = []
    for r in rows:
        ie = r["iecor"]
        if ie.get("historical", "").lower() != "true":
            continue
        # A catalog code is a source assertion, not the documentary sample.
        gid = ie.get("Glottocode")
        if not gid or gid not in g.rows:
            gid = None  # Keep an unplaced sample prior; the raw source code survives.
        if ie.get("normalMean"):
            prior = f"normal(mean={ie['normalMean']}, sd={ie.get('normalStDev')})"
        elif ie.get("logNormalMean"):
            prior = f"log-normal(mean={ie['logNormalMean']}, sd={ie.get('logNormalStDev')}, offset={ie.get('logNormalOffset')})"
        else:
            continue
        t = TimeValue(
            unknown=True,
            precision="unknown",
            conversion="Native prior parameters retained; epoch and lognormal parameterization unverified",
        )
        claims.append(
            Claim(
                id=claim_id(gid, SOURCE.id, ie["ID"]),
                languoid=gid,
                languoid_name=ie["Name"],
                kind="expert_estimate",
                time=t,
                source=SOURCE.id,
                locator=f"iecor_languages.csv ID={ie['ID']} ({ie['Name']})",
                quote=ie.get("Description") or f"{ie['Name']}: {prior} years BP",
                quote_verified=False,
                extracted_by={"program": "harness.bootstrap.iecor", "run": "bootstrap"},
                evidence={
                    "role": "calibration_prior",
                    "event": "documentary_sampling",
                    "normalization": "unknown",
                    "raw_prior": ie,
                    "subject_type": "documentary_sample",
                    "subject_id": "iecor:sample:" + ie["ID"],
                    "catalog_mapping": "unresolved",
                    "catalog_mapping_basis": "literal IE-CoR table code; never replaced by Wiktionary proposal",
                    "limitation": "epoch and lognormal parameterization require source-specific verification",
                },
                notes=f"tip-date prior on the sampled variety ({ie['Name']}): {prior} years BP",
            )
        )
    SOURCE.retrieved = now()
    return write_claims(g, SOURCE, claims)
