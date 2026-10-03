"""Glottolog First/Last_Year_Of_Documentation -> claims.

Glottolog gives these for extinct and historical languages only. Documentation endpoints are retained as documentation assertions for all entities;
they are never turned into stage periodizations.
Glottolog's negative years are historical (-100 = 100 BCE) and are converted."""

from __future__ import annotations

from .. import glotto
from ..config import now
from ..schema import Claim, Source
from ..time_utils import historical_to_astronomical
from . import claim_id, interval, point, write_claims

SOURCE = Source(
    id="src_glottolog_5_3",
    type="database",
    title="Glottolog 5.3 (First/Last_Year_Of_Documentation, endangerment status)",
    authors=["Hammarström, Harald", "Forkel, Robert", "Haspelmath, Martin", "Bank, Sebastian"],
    year=2026,
    url="https://glottolog.org",
    version="5.3",
    license="CC-BY-4.0",
    citation="Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.",
)


def run() -> dict:
    g = glotto.load()
    claims: list[Claim] = []
    for gid, r in g.rows.items():
        first = int(r["first_year"]) if r["first_year"] else None
        last = int(r["last_year"]) if r["last_year"] else None
        if first is None and last is None:
            continue
        a = historical_to_astronomical(first) if first is not None else None
        b = historical_to_astronomical(last) if last is not None else None
        types = {g.wikt[c].get("iso639_3_type") for c in g.wikt_by_glotto.get(gid, [])}
        extracted = {"program": "harness.bootstrap.glottolog", "run": "bootstrap"}
        quote = f"{r['name']}: First_Year_Of_Documentation={r['first_year'] or ''}, Last_Year_Of_Documentation={r['last_year'] or ''}"
        if a is not None:
            claims.append(
                Claim(
                    id=claim_id(gid, SOURCE.id, "first"),
                    languoid=gid,
                    languoid_name=r["name"],
                    kind="attestation_first",
                    time=point(a, "year", conversion="historical->astronomical"),
                    source=SOURCE.id,
                    locator=f"languages.csv row {gid}",
                    quote=quote,
                    quote_verified=True,
                    extracted_by=extracted,
                )
            )
        if b is not None:
            claims.append(
                Claim(
                    id=claim_id(gid, SOURCE.id, "last"),
                    languoid=gid,
                    languoid_name=r["name"],
                    kind="attestation_last",
                    time=point(b, "year", conversion="historical->astronomical"),
                    source=SOURCE.id,
                    locator=f"languages.csv row {gid}",
                    quote=quote,
                    quote_verified=True,
                    extracted_by=extracted,
                )
            )
    SOURCE.retrieved = now()
    return write_claims(g, SOURCE, claims)
