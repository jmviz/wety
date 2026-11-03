"""How two sources' comparison keys differ where their records are matched, and a
distance between keys that learns from it.

An orthography profile folds what curators know to be notation. What they have not yet
written down shows in the matches: aligned key by key, the pairs a matcher has
confirmed by other evidence differ again and again in the same places (de Vaan's
palatal *ḱ against Wiktionary's *k, a laryngeal one writes and the other drops). `mine`
counts those differences and writes them as a table of substitutions
(`etc/substitutions-<other>.tsv`), with a cost for each and examples, for a curator to
read: a difference seen often costs little, and a curator may move it into the profile
once it is plainly notation. `Distance` reads the table and compares keys by an edit
distance whose costs are the table's, so that a matcher can tell a reconstruction
written otherwise from another reconstruction.
"""

import argparse
import collections
import csv
import json
from pathlib import Path

from etymology import assertions as A

# A difference seen this many times among confirmed pairs costs `LOW`; one seen fewer
# times costs in between; any other costs 1.
OFTEN = 5
LOW = 0.25
GAP = "∅"


def align(a, b, cost=lambda x, y: 0 if x == y else 1):
    """The cheapest alignment of two keys, character by character: (distance, the pairs
    of aligned characters, GAP for none)."""
    n, m = len(a), len(b)
    d = [[0.0] * (m + 1) for _ in range(n + 1)]
    for i in range(1, n + 1):
        d[i][0] = d[i - 1][0] + cost(a[i - 1], GAP)
    for j in range(1, m + 1):
        d[0][j] = d[0][j - 1] + cost(GAP, b[j - 1])
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            d[i][j] = min(
                d[i - 1][j - 1] + cost(a[i - 1], b[j - 1]),
                d[i - 1][j] + cost(a[i - 1], GAP),
                d[i][j - 1] + cost(GAP, b[j - 1]),
            )
    pairs, i, j = [], n, m
    while i or j:
        if i and j and d[i][j] == d[i - 1][j - 1] + cost(a[i - 1], b[j - 1]):
            pairs.append((a[i - 1], b[j - 1]))
            i, j = i - 1, j - 1
        elif i and d[i][j] == d[i - 1][j] + cost(a[i - 1], GAP):
            pairs.append((a[i - 1], GAP))
            i -= 1
        else:
            pairs.append((GAP, b[j - 1]))
            j -= 1
    return d[n][m], pairs[::-1]


def differences(a, b):
    """The substitutions, insertions and deletions that turn key `a` into key `b`."""
    return [(x, y) for x, y in align(a, b)[1] if x != y]


def closest(keys_a, keys_b, distance):
    """The two keys, one of each set, nearest each other: (distance, a, b)."""
    return min(((distance(a, b), a, b) for a in keys_a for b in keys_b), default=(None, None, None))


class Distance:
    """An edit distance between keys whose costs are a substitutions table's."""

    def __init__(self, costs=None):
        self.costs = costs or {}

    @classmethod
    def read(cls, path):
        path = Path(path)
        if not path.exists():
            return cls()
        with path.open(newline="") as f:
            rows = csv.DictReader((line for line in f if not line.startswith("#")), delimiter="\t")
            return cls({(r["From"], r["To"]): float(r["Cost"]) for r in rows})

    def cost(self, x, y):
        if x == y:
            return 0.0
        return self.costs.get((x, y), 1.0)

    def __call__(self, a, b):
        """The distance from key `a` to key `b`, per character of the longer."""
        if not a or not b:
            return 1.0
        return align(a, b, self.cost)[0] / max(len(a), len(b))


def mine(pairs):
    """`pairs` are (key set of one source, key set of the other, label) for confirmed
    matches. The differences between each pair's closest keys, with how often each is
    seen and examples."""
    counts, examples = collections.Counter(), collections.defaultdict(list)
    plain = Distance()
    for keys_a, keys_b, label in pairs:
        _, a, b = closest(keys_a, keys_b, plain)
        if a is None or a == b:
            continue
        for d in set(differences(a, b)):
            counts[d] += 1
            if len(examples[d]) < 3:
                examples[d].append(label)
    rows = []
    for (x, y), n in counts.most_common():
        cost = LOW if n >= OFTEN else round(1 - (1 - LOW) * n / OFTEN, 2)
        rows.append(
            {"From": x, "To": y, "Count": n, "Cost": cost, "Examples": "; ".join(examples[(x, y)])}
        )
    return rows


def write_table(rows, path, header):
    with Path(path).open("w", newline="") as f:
        f.write("".join(f"# {line}\n" for line in header))
        w = csv.DictWriter(f, ["From", "To", "Count", "Cost", "Examples"], delimiter="\t")
        w.writeheader()
        w.writerows(rows)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("pairs", type=Path, help="a JSON list of [keys, keys, label] from a matcher")
    p.add_argument("--out", type=Path, required=True, help="the substitutions table to write")
    p.add_argument("--note", default="", help="the table's first comment line")
    args = p.parse_args(argv)
    pairs = [(set(a), set(b), label) for a, b, label in json.loads(args.pairs.read_text())]
    rows = mine(pairs)
    source = args.pairs.resolve().relative_to(A.REPO)
    header = [
        args.note,
        f"Mined by etymology.notation from {len(pairs)} confirmed pairs in {source}:",
        f"the differences between their closest keys. Cost {LOW} when seen {OFTEN} times or more;",
        f"{GAP} is no character.",
    ]
    write_table(rows, args.out, [h for h in header if h])
    print(f"{args.out}: {len(rows)} differences")


if __name__ == "__main__":
    main()
