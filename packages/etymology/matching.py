"""What every matcher shares: reading an export, the closure of what objects refer to,
reviewed judgements and the regression check they give, a Fellegi–Sunter weighing of
evidence, and the assignment of a source's homographs.

A match is decided per record pair by evidence of several kinds (comparison keys,
reflexes, ancestors, glosses, shape). `Linkage` turns each kind's level into a weight
learnt from the pairs themselves (EM, as Fellegi and Sunter's record linkage) and from
the judgements reviewers made, so that a pair's score is the probability that its two
records stand for one word, not a sum of hand-set constants.

Judgements are kept per source in `etc/judgements.jsonl`, apart from any rule's
version: each says of a record and a Wiktionary record whether they stand for the same
word (`same`), or of a record and a set of candidates that its word is among them
(`among`). Every run is scored against all of them (`Judgements.score`), so a new rule
cannot lose a reviewed right match unnoticed.
"""

import argparse
import collections
import json
import math
import subprocess
from pathlib import Path

from etymology import assertions as A


def lines(path, binary=False):
    """The lines of a file, as text or as bytes; a gzipped one decompressed by gzip(1),
    which is faster than Python's."""
    path = Path(path)
    if path.suffix != ".gz":
        with path.open("rb" if binary else "r", **({} if binary else {"encoding": "utf-8"})) as f:
            yield from f
        return
    with subprocess.Popen(["gzip", "-dc", str(path)], stdout=subprocess.PIPE, bufsize=1 << 20) as p:
        for line in p.stdout:
            yield line if binary else line.decode()


def references(o):
    """The ids an object refers to."""
    t = o["type"]
    refs = []
    if t in ("record", "cognate_set", "assertion"):
        refs.append(o["source"])
    if t == "work":
        refs += o.get("authors", [])
    elif t == "hypothesis":
        refs += [r for link in o["links"] for r in (link["child"], *link["parents"])]
    elif t == "cognate_set":
        refs += [r for r in [o["reconstruction"], *o["members"]] if r]
    elif t == "alternative_set":
        refs += o["members"]
    elif t == "assertion":
        refs += [o["asserter"], o["holder"], o["extraction"], o["target"]]
        refs += [c["work"] for c in o.get("credits", [])] + o.get("in_sets", [])
        refs += [o["replaces"]] if "replaces" in o else []
    elif t == "match":
        refs += [o["record"], o["holder"], *o.get("candidates", [])]
        refs += [o[k] for k in ("target", "extraction") if k in o]
    elif t == "relay":
        refs += [o["assertion"], o["relays"], o["work"], o["holder"]]
    return refs


def closure(seeds, *lookups):
    """The seeds with every object they refer to, transitively, as an `A.Writer`."""
    out = A.Writer()
    todo = list(seeds)
    while todo:
        o = todo.pop()
        if o["id"] in out.objects:
            continue
        out.add(o)
        for ref in references(o):
            if ref not in out.objects:
                found = next((lk[ref] for lk in lookups if ref in lk), None)
                if found is None:
                    raise KeyError(f"{o['id']} refers to {ref}, which no input holds")
                todo.append(found)
    return out


# Judgements


def outcome(found):
    """A match's outcome from what a rule found: None, a target, or a tuple of candidates."""
    if not found:
        return None
    if isinstance(found, str):
        return found
    found = tuple(sorted(found))
    return found[0] if len(found) == 1 else found


def found_in(objs):
    """What each record's match object found: its target or its candidates."""
    return {
        o["record"]: o.get("target") or tuple(o["candidates"]) for o in objs if o["type"] == "match"
    }


class Judgements:
    """Reviewed judgements on record pairs, kept apart from the rules that proposed them.

    A row is `{"record", "candidate", "same": bool}` or `{"record", "among": [...],
    "same": true}`, with `reviewer`, `check` (where it was judged) and `cause`. A later
    row on the same pair replaces an earlier one."""

    def __init__(self, rows=()):
        self.pairs, self.among = {}, collections.defaultdict(list)
        self.replaced = 0
        for row in rows:
            self.add(row)

    @classmethod
    def read(cls, path):
        path = Path(path)
        return cls(map(json.loads, path.read_text().splitlines()) if path.exists() else ())

    def rows(self):
        out = list(self.pairs.values())
        out += [row for rows in self.among.values() for row in rows]
        return sorted(out, key=lambda r: (r["record"], r.get("candidate", ""), r.get("among", [])))

    def write(self, path):
        text = "".join(
            json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n" for r in self.rows()
        )
        Path(path).write_text(text)

    def add(self, row):
        """Adds a row; returns whether it says something new."""
        if "among" in row:
            among = sorted(row["among"])
            known = self.among[row["record"]]
            if any(r["among"] == among for r in known):
                return False
            known.append({**row, "among": among})
            return True
        key = (row["record"], row["candidate"])
        old = self.pairs.get(key)
        if old and old["same"] == row["same"]:
            return False
        self.replaced += old is not None
        self.pairs[key] = row
        return True

    def same(self, record):
        """The records judged to stand for `record`'s word."""
        return {c for (r, c), row in self.pairs.items() if r == record and row["same"]}

    def label(self, record, candidate):
        """True or False when the pair is judged, else None."""
        row = self.pairs.get((record, candidate))
        return None if row is None else row["same"]

    def verdict(self, record, found):
        """`right`, `wrong`, `missed` (no match where a match or a candidate set was judged
        right) or None (not judged)."""
        found = outcome(found)
        same = self.same(record)
        among = self.among.get(record, [])
        if found is None:
            return "missed" if same or among else None
        if isinstance(found, str):
            label = self.label(record, found)
            if label is not None:
                return "right" if label else "wrong"
            return "wrong" if same else None
        if same & set(found) or any(set(r["among"]) <= set(found) for r in among):
            return "right"
        if same or all(self.label(record, c) is False for c in found):
            return "wrong"
        return None

    def score(self, results):
        """`results` maps each record to what a rule found. Precision of the judged
        targets and candidate sets, recall of the judged matches, and every regression."""
        by = collections.defaultdict(collections.Counter)
        regressions = []
        records = {r for r, _ in self.pairs} | set(self.among)
        for record in sorted(records):
            found = outcome(results.get(record))
            v = self.verdict(record, found)
            shape = (
                "no match"
                if found is None
                else "target"
                if isinstance(found, str)
                else "candidates"
            )
            by[shape][v or "not judged"] += 1
            if v in ("wrong", "missed"):
                regressions.append(
                    {
                        "record": record,
                        "found": found,
                        "judged same": sorted(self.same(record)),
                        "verdict": v,
                        "cause": self.cause(record, found),
                    }
                )
        t = by["target"]
        judged_same = sum(bool(self.same(r)) for r in records)
        hit = sum(outcome(results.get(r)) in self.same(r) for r in records if self.same(r))
        return {
            "judgements": len(self.pairs) + sum(map(len, self.among.values())),
            "records judged": len(records),
            "by outcome": {k: dict(sorted(v.items())) for k, v in sorted(by.items())},
            "target precision": round(t["right"] / (t["right"] + t["wrong"]), 3)
            if t["right"] + t["wrong"]
            else None,
            "recall of judged matches": round(hit / judged_same, 3) if judged_same else None,
            "regressions": regressions,
        }

    def cause(self, record, found):
        rows = [
            self.pairs.get((record, c))
            for c in ([found] if isinstance(found, str) else found or ())
        ]
        rows += [self.pairs[(record, c)] for c in self.same(record)]
        return next((r["cause"] for r in rows if r and r.get("cause")), None)


def harvest(path):
    """Judgement rows from a check's review file: a sample (`review.json`) or a list of
    changes (`changes.json`), in the formats the LiLa and ACD matchers write."""
    path = Path(path)
    doc = json.loads(path.read_text())
    check = path.parent.name
    reviewer = doc.get("reviewer")
    out = []

    def row(record, candidate, same, cause):
        out.append(
            {
                "record": record,
                "candidate": candidate,
                "same": same,
                "reviewer": reviewer,
                "check": check,
                "cause": cause,
            }
        )

    def among(record, candidates, cause):
        out.append(
            {
                "record": record,
                "among": sorted(candidates),
                "same": True,
                "reviewer": reviewer,
                "check": check,
                "cause": cause,
            }
        )

    for key in ("sample", "changed_sample"):
        # the ACD's reviews: a verdict on a match's target
        for e in doc.get(key, []):
            if e.get("verdict") in ("right", "wrong"):
                row(
                    e["acd"]["record"],
                    e["wiktionary"]["record"],
                    e["verdict"] == "right",
                    e["cause"],
                )
    for e in doc.get("rows", []):
        # LiLa's samples and changes: a verdict on what the record found (`found`; the
        # first reviews named it `target` or `after`), and the reviewer's own answer
        j = e.get("judgement")
        if e.get("answer"):
            row(e["record"], e["answer"], True, e.get("cause"))
        if j not in ("right", "wrong"):
            continue
        right, cause = j == "right", e.get("cause")
        found = e["found"] if "found" in e else e.get("after", e.get("target"))
        before = e.get("before")
        if isinstance(found, str):
            row(e["record"], found, right, cause)
        elif found:
            if right:
                among(e["record"], found, cause)
            else:
                for c in found:
                    row(e["record"], c, False, cause)
        elif isinstance(before, str):
            row(e["record"], before, not right, cause)
        elif before and right:
            for c in before:
                row(e["record"], c, False, cause)
        elif before:
            among(e["record"], before, cause)
    return out


def harvest_into(path, reviews):
    """Adds the judgements of `reviews` to the judgements file at `path`; returns how
    many rows were new."""
    j = Judgements.read(path)
    added = sum(j.add(r) for review in reviews for r in harvest(review))
    j.write(path)
    return added


# Weighing evidence


class Linkage:
    """Fellegi–Sunter weights for record pairs described by categorical evidence.

    Each field (a kind of evidence) has levels, listed from the strongest agreement to
    none. For a field and level, `m` is the probability of the level among pairs that
    stand for one word, `u` among pairs that do not; a pair's weight is the sum of
    log2(m/u) over its fields, and its probability follows from the prior. `fit` learns
    m, u and the prior by EM from all candidate pairs, with judged pairs held at their
    label. Fields must be roughly independent given the class: evidence that counts
    the same thing twice (reflexes and ancestors both count matched lineage) belongs in
    one field. A field's `missing` level (no evidence either way) starts at weight 0.
    Unfitted, the weights fall evenly from the strongest level to the weakest."""

    SMOOTH = 0.5

    def __init__(self, fields, missing=None):
        self.fields = fields
        self.m, self.u = {}, {}
        for f, levels in fields.items():
            graded = [lv for lv in levels if lv != (missing or {}).get(f)]
            n = len(graded)
            total = n * (n + 1) / 2
            self.m[f] = {lv: (n - i) / total for i, lv in enumerate(graded)}
            self.u[f] = {lv: (i + 1) / total for i, lv in enumerate(graded)}
            if len(graded) < len(levels):
                # as likely among matches as among non-matches, renormalised
                self.m[f] = {lv: p * n / (n + 1) for lv, p in self.m[f].items()}
                self.u[f] = {lv: p * n / (n + 1) for lv, p in self.u[f].items()}
                self.m[f][missing[f]] = self.u[f][missing[f]] = 1 / (n + 1)
        self.prior = 0.5

    def weight(self, features):
        return sum(math.log2(self.m[f][lv] / self.u[f][lv]) for f, lv in features.items())

    def probability(self, features):
        w = self.weight(features) + math.log2(self.prior / (1 - self.prior))
        return 1 / (1 + 2**-w)

    def fit(self, pairs, labels=None, iterations=100):
        """`pairs` is a list of feature dicts, `labels` a parallel list of True, False or
        None (not judged)."""
        labels = labels or [None] * len(pairs)
        for _ in range(iterations):
            g = [
                1.0 if lab is True else 0.0 if lab is False else self.probability(p)
                for p, lab in zip(pairs, labels)
            ]
            total = sum(g)
            for f, levels in self.fields.items():
                mc = collections.Counter()
                uc = collections.Counter()
                for p, gi in zip(pairs, g):
                    mc[p[f]] += gi
                    uc[p[f]] += 1 - gi
                ms = total + self.SMOOTH * len(levels)
                us = len(pairs) - total + self.SMOOTH * len(levels)
                self.m[f] = {lv: (mc[lv] + self.SMOOTH) / ms for lv in levels}
                self.u[f] = {lv: (uc[lv] + self.SMOOTH) / us for lv in levels}
            self.prior = min(max(total / len(pairs), 0.01), 0.99) if pairs else 0.5
        return self

    def table(self):
        """The learnt weights, for a check's result."""
        return {
            "prior": round(self.prior, 3),
            "weights": {
                f: {lv: round(math.log2(self.m[f][lv] / self.u[f][lv]), 2) for lv in levels}
                for f, levels in self.fields.items()
            },
        }


def decide(scored, threshold, margin):
    """From (candidate, probability) pairs: the candidates kept. The best one alone when
    it reaches `threshold` and beats the next by `margin` (in log2 odds); every one
    within the margin of the best when several reach it; none when the best does not."""
    ranked = sorted(scored, key=lambda t: (-t[1], t[0]))
    if not ranked or ranked[0][1] < threshold:
        return []
    best = logit(ranked[0][1])
    return [c for c, p in ranked if p >= threshold and best - logit(p) < margin]


def logit(p):
    p = min(max(p, 1e-9), 1 - 1e-9)
    return math.log2(p / (1 - p))


def assign(groups, scored, threshold, margin):
    """Decisions for records that cannot share a target: each group holds a source's
    homographs, distinct words of one form. A record decided on its own takes its
    target; that target is then struck from the other records' tied candidates, unless
    it is the last of them, until nothing changes."""
    found = {r: decide(scored[r], threshold, margin) for r in scored}
    for group in groups:
        group = [r for r in group if r in scored]
        changed = True
        while changed:
            changed = False
            taken = {found[r][0] for r in group if len(found[r]) == 1}
            for r in group:
                mine = [c for c in found[r] if c not in taken]
                if len(found[r]) > 1 and mine and len(mine) < len(found[r]):
                    found[r] = mine
                    changed = True
    return found


def main(argv=None):
    p = argparse.ArgumentParser(description="Reviewed judgements of record pairs.")
    sub = p.add_subparsers(dest="step", required=True)
    h = sub.add_parser("harvest", help="add the judgements of review files to a judgements file")
    h.add_argument("judgements", type=Path)
    h.add_argument("reviews", type=Path, nargs="+")
    args = p.parse_args(argv)
    if args.step == "harvest":
        n = harvest_into(args.judgements, args.reviews)
        print(f"{args.judgements}: {n} new judgements")


if __name__ == "__main__":
    main()
