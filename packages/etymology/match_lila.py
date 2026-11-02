"""LiLa's de Vaan records matched to Wiktionary's, and Wiktionary's credits to de Vaan relayed.

Matching (rule `agent:wety-match-lila@1`). Only a language LiLa's map gives as `exact`
matches, against Wiktionary records of the same code; a real entry is preferred to an
imputed one (`#n?`), which is a candidate only when no entry fits.

- A Latin entry matches by its form: `equal` when the forms are the same once LiLa's
  notation for variants (`ve/ortō`, `(h)olus`) is expanded, `near` when they are the same
  without vowel length, breves and j/i. De Vaan cites no reflexes for a Latin entry, and
  LiLa gives no glosses, so the form is all a Latin match weighs: homographs (Wiktionary's
  Etymology 1 and 2, de Vaan's serō 1 and 2) stay candidates.
- An etymon (a Proto-Italic or PIE reconstruction) is compared with every Wiktionary
  record of its language by the comparison keys of the two orthography profiles. Its
  cited reflexes are the Latin entries whose etymology in LiLa runs through it; one is
  shared when wety holds a Latin word of that form among the candidate's descendants.
  The candidates whose keys are equal are ranked by shared reflexes; one ahead of the
  rest is the target, a tie keeps them all as candidates. LiLa's etymon-to-etymon
  cognate links tie a derivative to the head of de Vaan's line, not to reflexes, so they
  are not weighed. Glosses are not weighed: LiLa has none.

Relaying (same rule). An assertion of Wiktionary's that credits de Vaan (2008), by its
References template `R:itc:EDL` or a footnote citing the book, is linked to de Vaan's
assertion in LiLa about the same word, once both are rewritten through the matches; see
`compare` for what counts as a difference.

`scan` reads a graph's assertion export once and keeps what this needs.
"""

import argparse
import collections
import json
import random
import re
import subprocess
import unicodedata
from pathlib import Path

from etymology import assertions as A
from etymology.convert import SOURCES
from etymology.orthography import Profile, key

NAME = "lila-edl"
RULE = "agent:wety-match-lila@1"
LILA = SOURCES / NAME
WIKTIONARY_PROFILE = SOURCES / "wiktionary/etc/orthography-ine-itc.tsv"
EDL = "work:wiktionary:R:itc:EDL"
# Wiktionary codes of de Vaan's records (LiLa's language map) and of the languages a
# Latin word descends through from them in wety.
CODES = re.compile(r'"code":"(la|ine-pro|itc-[a-z]+)"')
LINEAGE = re.compile(r"wiktionary:(la|ine-pro|itc-[a-z]+):")
FOOTNOTE_WORK = re.compile(r'"work":"(work:wiktionary:[0-9a-f]{16})"')
CONTEXT = ('"type":"source"', '"type":"agent"', '"type":"extraction"')
# Latin forms: `equal` keeps vowel length, `near` drops it, the breve and the j/i spelling.
HYPHENS = {"‑": "-", "-": "-"}
LATIN_EQUAL = Profile({**HYPHENS, "\u0304": "\u0304", "\u0306": "\u0306"}, ())
NEAR_LENGTH = 3
LATIN_NEAR = Profile({**HYPHENS, "\u0304": "", "\u0306": "", "\u0301": "", "j": "i"}, ())


def de_vaan_2008(work):
    """Whether a Wiktionary work is de Vaan's Etymological Dictionary of Latin (2008)."""
    if work["id"] == EDL:
        return True
    c = work.get("citation", "")
    return "Vaan" in c and "Etymological Dictionary of Latin" in c


def lines(path):
    path = Path(path)
    if path.suffix != ".gz":
        with path.open(encoding="utf-8") as f:
            yield from f
        return
    with subprocess.Popen(["gzip", "-dc", str(path)], stdout=subprocess.PIPE) as p:
        for line in p.stdout:
            yield line.decode()


class Wiktionary:
    """What matching and relaying need from a Wiktionary assertion export: de Vaan's works,
    the records and hypotheses of Latin, Italic and PIE words, the assertions that credit
    de Vaan with what they refer to, and the sources, agents, works and extractions."""

    def __init__(self, path):
        self.works, self.all_works, self.records, self.hypotheses = {}, {}, {}, {}
        self.alternatives, self.crediting, self.context = {}, {}, {}
        self.scan(path)
        self.children = collections.defaultdict(set)
        self.form_of = set()
        self.by_code = collections.defaultdict(list)
        for h in self.hypotheses.values():
            for link in h["links"]:
                for p in link["parents"]:
                    self.children[p].add(link["child"])
                if link.get("mode") == "form":
                    self.form_of.add(link["child"])
        for r in self.records.values():
            self.by_code[r["language"]["code"]].append(r["id"])

    def scan(self, path):
        """One pass keeps what the filters name; later passes fetch the hypotheses and
        records of other languages that a crediting assertion refers to."""
        alternatives = {}
        for line in lines(path):
            if '"type":"work"' in line:
                w = json.loads(line)
                self.all_works[w["id"]] = w
                if de_vaan_2008(w):
                    self.works[w["id"]] = w
            elif '"type":"record"' in line:
                if CODES.search(line):
                    r = json.loads(line)
                    self.records[r["id"]] = r
            elif '"type":"hypothesis"' in line:
                if LINEAGE.search(line):
                    h = json.loads(line)
                    self.hypotheses[h["id"]] = h
            elif '"type":"alternative_set"' in line:
                alt = json.loads(line)
                alternatives[alt["id"]] = alt
            elif '"type":"assertion"' in line:
                if '"credits"' in line and (EDL in line or FOOTNOTE_WORK.search(line)):
                    a = json.loads(line)
                    self.crediting[a["id"]] = a
            elif any(c in line for c in CONTEXT):
                o = json.loads(line)
                self.context[o["id"]] = o
        self.crediting = {
            i: a
            for i, a in self.crediting.items()
            if any(c["work"] in self.works for c in a["credits"])
        }
        targets = {a["target"] for a in self.crediting.values()}
        self.alternatives = {i: alt for i, alt in alternatives.items() if i in targets}
        targets = (targets - self.alternatives.keys()) | {
            m for alt in self.alternatives.values() for m in alt["members"]
        }
        self.fetch(path, "hypothesis", targets - self.hypotheses.keys(), self.hypotheses)
        needed = {
            r
            for h in targets
            for link in self.hypotheses[h]["links"]
            for r in (link["child"], *link["parents"])
        }
        self.fetch(path, "record", needed - self.records.keys(), self.records)

    @staticmethod
    def fetch(path, kind, ids, into):
        if not ids:
            return
        tag = f'"type":"{kind}"'
        for line in lines(path):
            if tag in line:
                o = json.loads(line)
                if o["id"] in ids:
                    into[o["id"]] = o

    def objects(self):
        for group in (
            self.context,
            self.all_works,
            self.records,
            self.hypotheses,
            self.alternatives,
            self.crediting,
        ):
            yield from group.values()

    def descendants(self, rid, code="la"):
        """The records of `code` that descend from `rid` in wety, by any chain of links."""
        seen, todo, out = {rid}, [rid], set()
        while todo:
            for c in self.children.get(todo.pop(), ()):
                if c not in seen:
                    seen.add(c)
                    todo.append(c)
                    if self.records.get(c, {}).get("language", {}).get("code") == code:
                        out.add(c)
        return out


class Lila:
    """LiLa's de Vaan as lila-2 converted it: records, hypotheses, de Vaan's assertions,
    and each etymon's cited reflexes."""

    def __init__(self, objs):
        self.objs = {o["id"]: o for o in objs}
        self.records = {i: o for i, o in self.objs.items() if o["type"] == "record"}
        self.reflexes = collections.defaultdict(set)
        self.by_word = collections.defaultdict(list)
        for o in self.objs.values():
            if o["type"] == "hypothesis":
                for link in o["links"]:
                    for p in link["parents"]:
                        self.reflexes[p].add(o["word"])
            elif o["type"] == "assertion" and self.objs[o["target"]]["type"] == "hypothesis":
                self.by_word[self.objs[o["target"]]["word"]].append(o)


def latin_keys(form, profile):
    """A Latin form's readings: LiLa's `(x)` and `a/b` expanded, and `a/b` also read as
    two whole forms (`lēvir/laevir`, not `vēnus/m`)."""
    keys = set(key(form, profile).keys)
    parts = form.split("/")
    if len(parts) > 1 and all(len(p) >= NEAR_LENGTH for p in parts):
        for part in parts:
            keys |= key(part, profile).keys
    return keys


def shape(form, lila):
    """How a reconstruction is written. Wiktionary's is an `affix` (a hyphen at its start),
    a `root` (a hyphen after a consonant at its end and none inside: *ḱel-, not the stem
    *porso-) or a `word`. De Vaan writes a
    hyphen after nearly everything, so his is an `affix`, a `stem` when it shows a
    morpheme boundary inside or ends in a stem vowel (*streu‑e/o‑, *lukʷo‑), else `bare`
    (*ḱēl‑, *gustu‑), which may be either a root or a word."""
    f = unicodedata.normalize("NFC", form).lstrip("*(").replace("‑", "-")
    if f.startswith("-"):
        return "affix"
    core = f.rstrip("-")
    if not lila:
        last = unicodedata.normalize("NFD", core[-1:])[:1]
        root = f.endswith("-") and "-" not in core and last not in "aeiouə"
        return "root" if root else "word"
    return "stem" if "-" in core or core[-1:] in ("o", "e") else "bare"


SHAPES = {"affix": {"affix"}, "stem": {"word"}, "bare": {"root", "word"}}


def compare_keys(a, b):
    """`equal` when two forms share a key as written, `near` when one of them needs an
    ending cut (de Vaan's stem *lukʷo‑ against Wiktionary's *lukʷos), `cut` when both do
    (*tang‑e‑ and *tangō), None otherwise. A cut key shorter than `NEAR_LENGTH` is not
    compared (*ki‑eje‑ against the particle *ki). `a` and `b` are pairs (keys as
    written, every key)."""
    if a[0] & b[0]:
        return "equal"
    if any(len(k) >= NEAR_LENGTH for k in a[0] & b[1] | a[1] & b[0]):
        return "near"
    if any(len(k) >= NEAR_LENGTH for k in a[1] & b[1]):
        return "cut"
    return None


class Matcher:
    def __init__(self, lila, wikt, lila_profile, wikt_profile):
        self.lila, self.wikt = lila, wikt
        self.profiles = {
            "lila": (lila_profile, Profile(lila_profile.graphemes, ())),
            "wikt": (wikt_profile, Profile(wikt_profile.graphemes, ())),
        }
        self.latin = {"equal": collections.defaultdict(set), "near": collections.defaultdict(set)}
        for rid in wikt.by_code["la"]:
            form = wikt.records[rid]["form"]
            if form:
                for k in latin_keys(form, LATIN_EQUAL):
                    self.latin["equal"][k].add(rid)
                for k in latin_keys(form, LATIN_NEAR):
                    self.latin["near"][k].add(rid)
        self.proto = collections.defaultdict(lambda: collections.defaultdict(set))
        self.keys = {}
        for code in ("itc-pro", "ine-pro"):
            for rid in wikt.by_code[code]:
                form = wikt.records[rid]["form"]
                if form:
                    self.keys[rid] = self.proto_keys(form, "wikt")
                    for k in self.keys[rid][1]:
                        self.proto[code][k].add(rid)
        self.results = {}

    def proto_keys(self, form, side):
        """A reconstruction's keys as written and with every ending cut too."""
        every, written = self.profiles[side]
        return frozenset(key(form, written).keys), frozenset(key(form, every).keys)

    def target(self, lila_record):
        res = self.results.get(lila_record)
        return res["found"][0] if res and len(res["found"]) == 1 else None

    @staticmethod
    def prefer_entries(rids, form_of):
        """Real entries over imputed records, and lemmas over form-of entries."""
        for keep in (
            lambda r: not r.endswith("?") and r not in form_of,
            lambda r: not r.endswith("?"),
            lambda r: True,
        ):
            chosen = sorted(r for r in rids if keep(r))
            if chosen:
                return chosen
        return []

    def latin_candidates(self, record):
        for strength in ("equal", "near"):
            profile = LATIN_EQUAL if strength == "equal" else LATIN_NEAR
            found = set()
            for k in latin_keys(record["form"], profile):
                found |= self.latin[strength][k]
            if found:
                # A capital marks a proper noun on both sides (Stēlla against stēlla).
                case = record["form"].lstrip("‑-(*")[:1].isupper()
                same_case = {r for r in found if self.wikt.records[r]["form"][:1].isupper() == case}
                return strength, self.prefer_entries(same_case or found, self.wikt.form_of)
        return None, []

    def match_latin(self, record):
        strength, found = self.latin_candidates(record)
        if not found:
            return None
        score = 0.9 if strength == "equal" else 0.7
        return {"found": found, "score": score, "basis": {"key": strength}}

    def match_etymon(self, record):
        code = record["language"]["code"]
        lila_keys = self.proto_keys(record["form"], "lila")
        shapes = SHAPES[shape(record["form"], True)]
        found = {}
        for k in lila_keys[1]:
            for r in self.proto[code].get(k, ()):
                if r not in found and shape(self.wikt.records[r]["form"], False) in shapes:
                    strength = compare_keys(lila_keys, self.keys[r])
                    if strength:
                        found[r] = strength
        if not found:
            return None
        reflexes = sorted(self.lila.reflexes.get(record["id"], ()))
        reflex_candidates = {
            r: set(self.latin_candidates(self.lila.records[r])[1]) for r in reflexes
        }
        ranked = []
        for c, strength in found.items():
            desc = self.wikt.descendants(c)
            shared = sum(bool(cands & desc) for cands in reflex_candidates.values())
            # Keys that both needed a cut stand only with a shared reflex behind them.
            if strength != "cut" or shared:
                ranked.append((shared, strength == "equal", c))
        if not ranked:
            return None
        ranked.sort(key=lambda t: (-t[0], -t[1], t[2]))
        top = [t for t in ranked if t[:2] == ranked[0][:2]]
        shared, equal = top[0][:2]
        basis = {"key": "equal" if equal else "near"}
        if reflexes:
            basis["reflexes"] = {"shared": shared, "cited": len(reflexes)}
        score = (0.6 if equal else 0.5) + (0.4 * shared / len(reflexes) if reflexes else 0)
        return {"found": [t[2] for t in top], "score": round(score, 3), "basis": basis}

    def match(self, record):
        if record["language"]["relation"] != "exact" or not record["form"]:
            return None
        if record["language"]["code"] == "la":
            return self.match_latin(record)
        return self.match_etymon(record)

    def run(self):
        """Every LiLa record's match result, Latin entries first (etymons weigh them)."""
        for rid, r in sorted(self.lila.records.items()):
            self.results[rid] = self.match(r)
        return self.results

    def objects(self, extraction):
        for rid, res in sorted(self.results.items()):
            if res is None:
                continue
            found = res["found"]
            yield A.match(
                rid,
                RULE,
                res["score"],
                res["basis"],
                target=found[0] if len(found) == 1 else None,
                candidates=found if len(found) > 1 else None,
                extraction=extraction,
            )


SUBSTANCE = ("parents", "relation", "mode", "hedge", "edition")


class Relayer:
    """Wiktionary's assertions that credit de Vaan (2008), each compared with de Vaan's
    assertion in LiLa about the same word, through the matches."""

    def __init__(self, matcher):
        self.matcher = matcher
        self.lila, self.wikt, results = matcher.lila, matcher.wikt, matcher.results
        self.results = results
        self.target = matcher.target
        self.by_target = collections.defaultdict(list)
        self.by_candidate = collections.defaultdict(list)
        for rid, res in results.items():
            if res is None:
                continue
            if len(res["found"]) == 1:
                self.by_target[res["found"][0]].append(rid)
            else:
                for c in res["found"]:
                    self.by_candidate[c].append(rid)
        self.relays, self.rows = [], []

    def credit(self, a):
        return next(c for c in a["credits"] if c["work"] in self.wikt.works)

    def chain(self, lila_record):
        """De Vaan's chains through a LiLa record: for each of his assertions that holds
        it, the assertion and the ancestors from that record up."""
        if lila_record in self.lila.by_word:
            for la in self.lila.by_word[lila_record]:
                h = self.lila.objs[la["target"]]
                yield la, [link["parents"][0] for link in h["links"]]
            return
        for word in self.lila.reflexes.get(lila_record, ()):
            for la in self.lila.by_word[word]:
                h = self.lila.objs[la["target"]]
                parents = [link["parents"][0] for link in h["links"]]
                if lila_record in parents:
                    yield la, parents[parents.index(lila_record) + 1 :]

    def compare(self, a, link, ancestors, la):
        """How Wiktionary's link (a's) differs from de Vaan's chain of `ancestors`.

        - `parents`: Wiktionary's parent is none of de Vaan's ancestors, or it names
          several (a morphological analysis where LiLa gives one etymon); a parent found
          further up de Vaan's chain is not a difference (Wiktionary skips a step).
        - `relation`: Wiktionary says the origin is unknown, or only an influence.
        - `mode`: Wiktionary's step is not an inheritance (LiLa's are all inheritances).
        - `hedge`: one of the two doubts the parent and the other does not.
        - `locator`: Wiktionary's credit names a page; LiLa locates by entry, not page.
        Returns the differences, a kind for the report, and whether the comparison is
        not established (a parent of de Vaan's that is matched to several records, one
        of them Wiktionary's)."""
        differences, kind = [], "agrees"
        if link["relation"] in ("unknown", "influence"):
            differences.append("relation")
            kind = f"Wiktionary: {link['relation']}"
        else:
            parents = link["parents"]
            targets = [self.target(r) for r in ancestors]
            if len(parents) > 1:
                differences.append("parents")
                kind = "Wiktionary analyses the word (several parents)"
            elif parents[0] in targets:
                j = targets.index(parents[0])
                if j:
                    kind = "agrees, skipping a step of de Vaan's chain"
                hedged = any(h.get("link") == 0 for h in a.get("hedges", []))
                doubted = "certainty" in self.lila.records[ancestors[j]]
                if hedged != doubted:
                    differences.append("hedge")
            elif any(parents[0] in (self.results.get(r) or {}).get("found", ()) for r in ancestors):
                return (
                    None,
                    "de Vaan's parent is matched to several records, Wiktionary's among them",
                )
            else:
                differences.append("parents")
                kind = self.parent_kind(parents[0], ancestors)
            if link["relation"] == "derivation" and link["mode"] != "inherited":
                differences.append("mode")
        if self.credit(a).get("locator"):
            differences.append("locator")
        return differences, kind

    def parent_kind(self, parent, ancestors):
        """What kind of `parents` difference: Wiktionary's parent against de Vaan's
        ancestor of the same language."""
        code = self.wikt.records[parent]["language"]["code"]
        same = [r for r in ancestors if self.lila.records[r]["language"]["code"] == code]
        if not same:
            return f"Wiktionary's parent is {code}; de Vaan's chain in LiLa has none"
        if self.target(same[0]) is None:
            mine = self.matcher.proto_keys(self.lila.records[same[0]]["form"], "lila")
            theirs = self.matcher.proto_keys(self.wikt.records[parent]["form"], "wikt")
            if mine[1] & theirs[1]:
                return "de Vaan's etymon is unmatched; its key shares a cut form with Wiktionary's"
            return "another reconstruction (de Vaan's etymon is unmatched)"
        return "another reconstruction (de Vaan's etymon matches another record)"

    def run(self):
        for a in sorted(self.wikt.crediting.values(), key=lambda a: a["id"]):
            self.relay(a)
        return self.relays

    def relay(self, a):
        credit = self.credit(a)
        row = {"assertion": a["id"], "locator": a["locator"], "work": credit["work"]}
        self.rows.append(row)
        if a["target"] in self.wikt.alternatives:
            row["outcome"] = "not compared: Wiktionary offers alternatives"
            return
        link = self.wikt.hypotheses[a["target"]]["links"][0]
        word = link["child"]
        code = self.wikt.records[word]["language"]["code"]
        row["word"], row["code"] = word, code
        if code not in ("la", "itc-pro", "ine-pro"):
            row["outcome"] = "no LiLa counterpart: LiLa encodes no records of this language"
            return
        mine = self.by_target.get(word, [])
        if not mine:
            row["outcome"] = (
                "not compared: de Vaan's record is matched to several, this one among them"
                if self.by_candidate.get(word)
                else "no LiLa counterpart: no LiLa record matches the word"
            )
            return
        best = None
        for rid in mine:
            for la, ancestors in self.chain(rid):
                differences, kind = self.compare(a, link, ancestors, la)
                if differences is None:
                    candidate = (99, la, None, kind)
                else:
                    weight = sum(d in SUBSTANCE for d in differences)
                    candidate = (weight, la, differences, kind)
                if best is None or candidate[0] < best[0]:
                    best = candidate
        if best is None:
            row["outcome"] = (
                "no LiLa counterpart: the matched record is in no etymology of de Vaan's"
            )
            return
        _, la, differences, kind = best
        row["lila"], row["kind"] = la["id"], kind
        if differences is None:
            row["outcome"] = "not compared: " + kind
            return
        row["differences"] = differences
        if code != "la":
            # LiLa asserts each chain of the Latin word it explains; a link between two
            # reconstructions is part of that chain, not an assertion of its own.
            row["outcome"] = (
                "compared, not relayed: LiLa states this link only inside a Latin word's chain"
            )
            return
        row["outcome"] = "relayed"
        self.relays.append(A.relay(a["id"], la["id"], credit["work"], RULE, differences))


def references(o):
    """The ids an object refers to."""
    t = o["type"]
    if t == "record":
        yield o["source"]
    elif t == "work":
        yield from o.get("authors", ())
    elif t == "hypothesis":
        for link in o["links"]:
            yield link["child"]
            yield from link["parents"]
    elif t == "alternative_set":
        yield from o["members"]
    elif t == "assertion":
        yield from (o["source"], o["asserter"], o["holder"], o["target"], o["extraction"])
        yield from (c["work"] for c in o.get("credits", ()))
        yield from o.get("in_sets", ())
    elif t == "match":
        yield from (o["record"], o["holder"], *o.get("candidates", ()))
        yield from (o[k] for k in ("target", "extraction") if k in o)
    elif t == "relay":
        yield from (o["assertion"], o["relays"], o["work"], o["holder"])


def closure(seeds, pool):
    """`seeds` and every object they refer to, by way of `pool`."""
    out, todo = {}, list(seeds)
    while todo:
        o = todo.pop()
        if o["id"] in out:
            continue
        out[o["id"]] = o
        todo.extend(pool[r] for r in references(o) if r not in out)
    return out


KINDS = {"la": "Latin entry", "itc-pro": "Proto-Italic etymon", "ine-pro": "PIE etymon"}
REVIEW_SIZE = 100
REVIEW_SEED = 20261101


def match_counts(lila, results):
    counts = collections.defaultdict(collections.Counter)
    for rid, r in lila.records.items():
        kind = KINDS[r["language"]["code"]]
        res = results.get(rid)
        c = counts[kind]
        c["records"] += 1
        if res is None:
            c["unmatched"] += 1
            continue
        c["matched" if len(res["found"]) == 1 else "ambiguous"] += 1
        c[f"key {res['basis']['key']}"] += 1
        reflexes = res["basis"].get("reflexes")
        if len(res["found"]) == 1:
            if reflexes is None:
                c["matched, no reflex cited"] += 1
            elif reflexes["shared"]:
                c["matched, a cited reflex is a descendant"] += 1
            else:
                c["matched, no cited reflex is a descendant"] += 1
    return {k: dict(sorted(v.items())) for k, v in counts.items()}


def relay_counts(rows):
    out = {"crediting assertions": len(rows)}
    out["by outcome"] = dict(collections.Counter(r["outcome"] for r in rows).most_common())
    out["by language of the word"] = dict(
        collections.Counter(r.get("code", "alternatives") for r in rows).most_common()
    )
    compared = [r for r in rows if "differences" in r]
    for label, chosen in (
        ("relayed", [r for r in compared if r["outcome"] == "relayed"]),
        ("compared, not relayed", [r for r in compared if r["outcome"] != "relayed"]),
    ):
        substance = [r for r in chosen if any(d in SUBSTANCE for d in r["differences"])]
        out[label] = {
            "count": len(chosen),
            "agree": sum(not r["differences"] for r in chosen),
            "differ only in locator": sum(r["differences"] == ["locator"] for r in chosen),
            "differ in substance": len(substance),
            "differences": dict(
                collections.Counter(d for r in chosen for d in r["differences"]).most_common()
            ),
            "kinds": dict(collections.Counter(r["kind"] for r in chosen).most_common()),
        }
    return out


def sample(matcher, objs):
    """A random sample of the matches that name a target, with what a reviewer needs."""
    wikt, lila = matcher.wikt, matcher.lila
    targeted = sorted(o["id"] for o in objs if o["type"] == "match" and "target" in o)
    chosen = random.Random(REVIEW_SEED).sample(targeted, REVIEW_SIZE)
    by_id = {o["id"]: o for o in objs}
    parents = collections.defaultdict(set)
    for h in wikt.hypotheses.values():
        for link in h["links"]:
            parents[link["child"]].update(link["parents"])
    rows = []
    for mid in chosen:
        m = by_id[mid]
        r = lila.records[m["record"]]
        ext = r.get("x", {}).get(NAME, {})
        rows.append(
            {
                "match": mid,
                "record": r["id"],
                "kind": KINDS[r["language"]["code"]],
                "form": r["form"],
                "entry": [c for c in ext.get("comments", []) if c.startswith("Etymology for")],
                "reflexes": sorted(lila.records[x]["form"] for x in lila.reflexes.get(r["id"], ())),
                "target": m["target"],
                "target_parents": sorted(parents.get(m["target"], ())),
                "target_latin_descendants": sorted(wikt.descendants(m["target"]))[:12],
                "score": m["score"],
                "basis": m["basis"],
                "judgement": None,
                "cause": None,
            }
        )
    return rows


def precision(rows):
    judged = [r for r in rows if r["judgement"]]
    by_kind = collections.defaultdict(collections.Counter)
    for r in judged:
        by_kind[r["kind"]][r["judgement"]] += 1
    right = sum(r["judgement"] == "right" for r in judged)
    return {
        "reviewed": len(judged),
        "right": right,
        "precision": round(right / len(judged), 3) if judged else None,
        "by kind": {k: dict(v) for k, v in by_kind.items()},
        "wrong": [
            {"record": r["record"], "form": r["form"], "target": r["target"], "cause": r["cause"]}
            for r in judged
            if r["judgement"] != "right"
        ],
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--export", type=Path, help="a graph's assertion export (graph-assertions)")
    p.add_argument("--out", type=Path, required=True, help="the check directory")
    p.add_argument("--review", action="store_true", help="only add the reviewed sample's precision")
    args = p.parse_args(argv)
    result_path = args.out / "result.json"
    if args.review:
        result = json.loads(result_path.read_text())
        result["review"] = precision(json.loads((args.out / "review.json").read_text())["rows"])
        result_path.write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n")
        print(json.dumps(result["review"], ensure_ascii=False, indent=1))
        return
    lila_path = LILA / "checks/lila-2/assertions.jsonl"
    profiles = (LILA / "etc/orthography.tsv", WIKTIONARY_PROFILE)
    extraction = A.extraction(
        "converter", "packages/etymology/match_lila.py", [args.export, lila_path, *profiles]
    )
    wikt = Wiktionary(args.export)
    lila = Lila(A.read(lila_path))
    matcher = Matcher(lila, wikt, *map(Profile.read, profiles))
    matcher.run()
    relayer = Relayer(matcher)
    relayer.run()
    rule = {
        "type": "agent",
        "id": RULE,
        "kind": "rule",
        "name": "wety's LiLa matcher, v1: de Vaan's records in LiLa matched to Wiktionary's "
        "by form, comparison key and cited reflexes, and Wiktionary's credits to de Vaan "
        "(2008) relayed to his assertions in LiLa",
    }
    made = [rule, extraction, *matcher.objects(extraction["id"]), *relayer.relays]
    pool = {o["id"]: o for o in [*lila.objs.values(), *wikt.objects(), *made]}
    out = A.Writer()
    for o in closure(made, pool).values():
        out.add(o)
    args.out.mkdir(parents=True, exist_ok=True)
    n = out.write(args.out / "assertions.jsonl")
    objs = list(out.objects.values())
    errors = A.validate(objs)
    result = {
        "milestone": "lila-3",
        "question": "Which of LiLa's de Vaan records stand for which of wety's items, and do "
        "Wiktionary's assertions that credit de Vaan (2008) say what de Vaan says?",
        "done_when": "match precision on a reviewed sample of 100; relays that agree and "
        "that differ; the consistency report",
        "extraction": extraction["id"],
        "objects": n,
        "errors": len(errors),
        "matches": match_counts(lila, matcher.results),
        "relays": relay_counts(relayer.rows),
        "de Vaan's works in the graph": {
            w["id"]: w["citation"] for w in sorted(wikt.works.values(), key=lambda w: w["id"])
        },
    }
    result_path.write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n")
    (args.out / "relays.json").write_text(
        json.dumps(relayer.rows, ensure_ascii=False, indent=1) + "\n"
    )
    (args.out / "sample.json").write_text(
        json.dumps(
            {"seed": REVIEW_SEED, "size": REVIEW_SIZE, "rows": sample(matcher, objs)},
            ensure_ascii=False,
            indent=1,
        )
        + "\n"
    )
    print(f"{args.out}: {n} objects, {len(errors)} errors")
    for e in errors[:20]:
        print("  " + e)


if __name__ == "__main__":
    main()
