"""Bootstrap ingesters: turn existing datasets into claims without any agent
judgment. Each module exposes `run() -> dict` (a summary) and registers its
source in sources.json. Claims are grouped into the top-level Glottolog family
of their languoid."""

from __future__ import annotations

import re
from collections import defaultdict

from .. import glotto
from ..config import now
from ..schema import Claim, Source, TimeValue
from ..store import delete_claims, upsert_claims, upsert_source


def slug(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", s.lower()).strip("_")


def claim_id(languoid: str | None, source_id: str, n: int | str, bucket: str | None = None) -> str:
    """clm_<glottocode>_<source>_<n>. Claims that could not be placed on a node
    use the family bucket's glottocode (or 'unpl0000') as the prefix."""
    prefix = languoid or bucket or "unpl0000"
    return f"clm_{prefix}_{slug(source_id.removeprefix('src_'))}_{slug(str(n))}"


def bucket_for(g: glotto.Glottolog, languoid: str | None, default: str = "unclassified") -> str:
    if languoid and languoid in g.rows:
        return g.top_family(languoid)
    return default


def write_claims(
    g: glotto.Glottolog, source: Source, claims: list[Claim], replace: bool = True
) -> dict:
    """Group claims by family bucket and write them; replace=True first removes
    this source's earlier claims so a re-run is idempotent."""
    upsert_source(source)
    by_fam: dict[str, list[Claim]] = defaultdict(list)
    for c in claims:
        by_fam[bucket_for(g, c.languoid)].append(c)
    summary = {"source": source.id, "claims": len(claims), "families": len(by_fam), "at": now()}
    if replace:
        from ..store import families_on_disk, replace_source_claims, load_claims

        old = {
            f for f in families_on_disk() if any(c["source"] == source.id for c in load_claims(f))
        }
        for fam in sorted(old | set(by_fam)):
            replace_source_claims(fam, source.id, by_fam.get(fam, []))
    else:
        for fam, cs in by_fam.items():
            upsert_claims(fam, cs)
    return summary


def interval(earliest: int | None, latest: int | None, precision: str = "year", **kw) -> TimeValue:
    return TimeValue(earliest=earliest, latest=latest, precision=precision, **kw)


def point(value: int, precision: str = "year", **kw) -> TimeValue:
    return TimeValue(value=value, precision=precision, **kw)
