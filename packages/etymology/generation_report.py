"""A graph generation's report: what the maintainer reviews instead of words.

It reads the generation's readings (`graph-edges --readings`), the lines in which its
edge dump differs from the previous generation's (`comm -3` of the two sorted dumps) and
each claims product's matches, and writes `report.json`, `report.md` and `sample.json`:

- readings by tier, and how many are disputed;
- the words whose followed parents changed, and why: a source's claims touch them, or
  not (a change the claims do not explain is a consistency failure);
- the relays that differ in substance, and the claims that found no word;
- a random sample of the readings of tiers 3 and 4 (consensus, preference) and of the
  unsettled ones, for a reviewer to audit.
"""

import argparse
import collections
import json
import random
from pathlib import Path

from etymology import assertions as A

SAMPLE = 50
SEED = 20261103


def readings(path):
    with open(path, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def word_key(cols):
    """The record id of an edge line's child, as `graph-edges` keys it."""
    lang, term, ety, kind = cols[:4]
    return f"wiktionary:{lang}:{term}#{ety}{'?' if kind == 'imputed' else ''}"


def changed_words(diff_path):
    """Children whose followed edges differ, from `comm -3` output: a line only in the
    previous dump starts with no tab, one only in the new dump with one tab."""
    words = collections.defaultdict(lambda: {"before": [], "after": []})
    with open(diff_path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            side = "after" if line.startswith("\t") else "before"
            cols = line.lstrip("\t").split("\t")
            if len(cols) < 15 or cols[14] != "true":
                continue
            words[word_key(cols)][side].append(f"{cols[4]}:{cols[5]}#{cols[6]} ({cols[8]})")
    return {w: v for w, v in words.items() if sorted(v["before"]) != sorted(v["after"])}


def relays(matches_paths):
    """Each product's relays that differ in substance, with their differences."""
    out = []
    for path in matches_paths:
        for o in A.read(path):
            if o["type"] != "relay" or o["agrees"]:
                continue
            if set(o["differences"]) <= {"locator"}:
                continue
            out.append({"source": str(path), "relay": o["id"], "differences": o["differences"]})
    return out


def report(readings_path, diff_path, matches_paths, out):
    rs = readings(readings_path)
    by_word = {r["word"]: r for r in rs}
    tiers = collections.Counter(r["tier"] for r in rs)
    disputed = sum(r["disputed"] for r in rs)
    relations = collections.Counter(
        c["relation"] for r in rs for e in r["etymologies"] for c in e["claims"]
    )
    origins = collections.Counter(
        c["origin"]
        for r in rs
        for e in r["etymologies"]
        for c in e["claims"]
        if c["stance"] == "endorses" and not c["compiled"]
    )
    changed = changed_words(diff_path)
    explained = {w: v for w, v in changed.items() if w in by_word}
    unexplained = {w: v for w, v in changed.items() if w not in by_word}
    differing = relays(matches_paths)
    rng = random.Random(SEED)
    sample = []
    for tier in ("consensus", "preference", "unsettled"):
        pool = [r for r in rs if r["tier"] == tier]
        for r in rng.sample(pool, min(SAMPLE, len(pool))):
            sample.append({"tier": tier, "reading": r, "verdict": None, "note": None})
    result = {
        "readings": len(rs),
        "by tier": dict(tiers.most_common()),
        "disputed": disputed,
        "claims by relation": dict(relations.most_common()),
        "endorsing claims by origin": dict(origins.most_common()),
        "words whose followed parents changed": len(changed),
        "changed, touched by claims": len(explained),
        "changed, not explained by claims": len(unexplained),
        "relays differing in substance": len(differing),
        "unexplained (first 50)": dict(list(unexplained.items())[:50]),
    }
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n")
    (out / "sample.json").write_text(
        json.dumps({"rows": sample}, ensure_ascii=False, indent=1) + "\n"
    )
    (out / "changed.json").write_text(json.dumps(explained, ensure_ascii=False, indent=1) + "\n")
    lines = [
        "# Generation report",
        "",
        f"{len(rs):,} words are touched by other sources' claims or not read by one lineage.",
        "",
        "| Tier | Words |",
        "| --- | ---: |",
        *[f"| {t} | {n:,} |" for t, n in tiers.most_common()],
        "",
        f"Disputed: {disputed:,}. Claims by relation: "
        + ", ".join(f"{k} {v:,}" for k, v in relations.most_common())
        + ".",
        "",
        (
            f"Followed parents changed for {len(changed):,} words: {len(explained):,} "
            f"touched by claims, {len(unexplained):,} not explained by them."
        ),
        "",
        f"Relays that differ in substance: {len(differing):,}.",
        "",
        (
            f"`sample.json`: {len(sample)} readings of tiers consensus, preference and "
            "unsettled to audit."
        ),
    ]
    (out / "report.md").write_text("\n".join(lines) + "\n")
    return result


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("readings", type=Path, help="graph-edges --readings output")
    p.add_argument("diff", type=Path, help="comm -3 of the previous and new sorted edge dumps")
    p.add_argument("--matches", type=Path, nargs="*", default=[], help="claims' matches files")
    p.add_argument("--out", type=Path, required=True)
    args = p.parse_args(argv)
    print(json.dumps(report(args.readings, args.diff, args.matches, args.out), indent=1))


if __name__ == "__main__":
    main()
