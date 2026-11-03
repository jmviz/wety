"""LiLa's de Vaan records matched to Wiktionary's, and Wiktionary's credits to de Vaan relayed.

Matching (rule `agent:wety-match-lila@3`). Only a language LiLa's map gives as `exact`
matches, against Wiktionary records of the same code. Each candidate pair is described
by its evidence, and `matching.Linkage` weighs it: how close the two forms are (`key`),
how much of the record's lineage in de Vaan's chains wety shares (`lineage`: its cited
reflexes, the Latin entries whose etymology in LiLa runs through it, among the
candidate's descendants; its ancestors' targets among the candidate's ancestors), and
for an etymon whether the candidate is an entry or a record wety knows only from
citations. The weights are learnt by EM from every candidate pair, judged pairs
(`etc/judgements.jsonl`) held at their label, so a pair's score is the probability that
the two records stand for one word. A candidate of probability `THRESHOLD` or more that
beats the next by `MARGIN` is the target; candidates within the margin of each other
stay candidates. De Vaan's homographs (records of one form and language) do not share a
target (`matching.assign`): when serō 'sow' takes one homograph, serō 'join' loses it
from its tied candidates. Rounds repeat, each weighing the targets the last decided,
until nothing changes.

- A Latin entry's candidates are Wiktionary's of the same form: `equal` once LiLa's
  notation for variants (`ve/ortō`, `(h)olus`) is expanded, `near` without vowel length,
  breves and j/i. A real entry is preferred to an imputed record (`#n?`), a lemma to a
  form-of entry, and a candidate of the same case. Its lineage is its etymons' targets.
- An etymon's (a Proto-Italic or PIE reconstruction's) candidates come two ways. By key:
  Wiktionary's records whose comparison keys agree (see `ProtoKeys`: palatals kept, vowel
  length compared when both forms mark it; `cut` when both lose an ending, `loose` when
  they agree only once length and palatals are folded). By lineage: the records of its
  language above the targets of the records below it in de Vaan's chains, where wety
  places his etymon whatever its form; such a candidate is kept when its key is `close`,
  within `CLOSE` by the learnt distance (`notation.Distance`, costs mined from confirmed
  pairs in `etc/substitutions-wiktionary.tsv`). Its match then has the key `different`:
  the same word, reconstructed otherwise (*lēg‑ against *lēks), which a relay reports
  as a `form` difference. A stem never matches a root (`shape`). LiLa's etymon-to-etymon
  cognate links tie a derivative to the head of de Vaan's line, not to reflexes, so they
  are not weighed. Glosses are not weighed: LiLa has none.

A match whose outcome would differ without its lineage "rests on lineage": wety's own
etymology decided it, so a relay through it cannot confirm that etymology, and relays
count such agreements apart.

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
import unicodedata
from dataclasses import dataclass
from pathlib import Path

from etymology import assertions as A
from etymology.convert import SOURCES
from etymology.matching import (
    Judgements,
    Linkage,
    assign,
    closure,
    decide,
    found_in,
    harvest_into,
    lines,
)
from etymology.notation import Distance, closest
from etymology.orthography import Profile, key

NAME = "lila-edl"
RULE = "agent:wety-match-lila@3"
LILA = SOURCES / NAME
WIKTIONARY_PROFILE = SOURCES / "wiktionary/etc/orthography-ine-itc.tsv"
EDL = "work:wiktionary:R:itc:EDL"
JUDGEMENTS = LILA / "etc/judgements.jsonl"
SUBSTITUTIONS = LILA / "etc/substitutions-wiktionary.tsv"
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
LENGTH = "ː"  # the key of the macron in the orthography profiles
LATIN_NEAR = Profile({**HYPHENS, "\u0304": "", "\u0306": "", "\u0301": "", "j": "i"}, ())


def de_vaan_2008(work):
    """Whether a Wiktionary work is de Vaan's Etymological Dictionary of Latin (2008)."""
    if work["id"] == EDL:
        return True
    c = work.get("citation", "")
    return "Vaan" in c and "Etymological Dictionary of Latin" in c


class Wiktionary:
    """What matching and relaying need from a Wiktionary assertion export: de Vaan's works,
    the records and hypotheses of Latin, Italic and PIE words, the assertions that credit
    de Vaan with what they refer to, and the sources, agents, works and extractions."""

    def __init__(self, path):
        self.works, self.all_works, self.records, self.hypotheses = {}, {}, {}, {}
        self.alternatives, self.crediting, self.context = {}, {}, {}
        self.scan(path)
        self.children = collections.defaultdict(set)
        self.parents = collections.defaultdict(set)
        self.form_of = set()
        self.by_code = collections.defaultdict(list)
        self.cache = {}
        for h in self.hypotheses.values():
            for link in h["links"]:
                for p in link["parents"]:
                    self.children[p].add(link["child"])
                    self.parents[link["child"]].add(p)
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
        if (rid, code) not in self.cache:
            self.cache[rid, code] = frozenset(
                c
                for c in reach(rid, self.children)
                if self.records.get(c, {}).get("language", {}).get("code") == code
            )
        return self.cache[rid, code]

    def ancestors(self, rid):
        """The records `rid` descends from in wety, by any chain of links."""
        if rid not in self.cache:
            self.cache[rid] = frozenset(reach(rid, self.parents))
        return self.cache[rid]


def reach(rid, edges):
    seen, todo = {rid}, [rid]
    while todo:
        for c in edges.get(todo.pop(), ()):
            if c not in seen:
                seen.add(c)
                todo.append(c)
    return seen - {rid}


class Lila:
    """LiLa's de Vaan as lila-2 converted it: records, hypotheses, de Vaan's assertions,
    each etymon's cited reflexes and each record's ancestors (the etymons above it in
    any of de Vaan's chains)."""

    def __init__(self, objs):
        self.objs = {o["id"]: o for o in objs}
        self.records = {i: o for i, o in self.objs.items() if o["type"] == "record"}
        self.reflexes = collections.defaultdict(set)
        self.ancestors = collections.defaultdict(set)
        self.by_word = collections.defaultdict(list)
        for o in self.objs.values():
            if o["type"] == "hypothesis":
                chain = [o["word"]] + [p for link in o["links"] for p in link["parents"]]
                for i, r in enumerate(chain):
                    self.ancestors[r].update(chain[i + 1 :])
                    if i:
                        self.reflexes[r].add(o["word"])
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


def long_enough(k):
    """A cut key of `NEAR_LENGTH` segments at least, the length mark not counted."""
    return len(k.replace(LENGTH, "")) >= NEAR_LENGTH


def compare_keys(a, b):
    """`equal` when two forms share a key as written, `near` when one of them needs an
    ending cut (de Vaan's stem *lukʷo‑ against Wiktionary's *lukʷos), `cut` when both do
    (*tang‑e‑ and *tangō), None otherwise. A cut key shorter than `NEAR_LENGTH` is not
    compared (*ki‑eje‑ against the particle *ki). `a` and `b` are pairs (keys as
    written, every key)."""
    if a[0] & b[0]:
        return "equal"
    if any(long_enough(k) for k in a[0] & b[1] | a[1] & b[0]):
        return "near"
    if any(long_enough(k) for k in a[1] & b[1]):
        return "cut"
    return None


MACRON = "\u0304"


FOLD = {LENGTH: "", "K": "k", "G": "g"}


def fold(text, marks):
    for k in marks:
        text = text.replace(k, FOLD[k])
    return text


def variants(profile):
    """A profile in three strictnesses, each as (with its endings, with none): `length`
    as written, `palatal` without vowel length, `loose` without the palatals either."""
    out = {}
    for name, marks in (("length", ()), ("palatal", (LENGTH,)), ("loose", tuple(FOLD))):
        p = Profile(
            {g: fold(k, marks) for g, k in profile.graphemes.items()},
            tuple(fold(e, marks) for e in profile.endings),
        )
        out[name] = (p, Profile(p.graphemes, ()))
    return out


@dataclass(frozen=True)
class ProtoKeys:
    """A reconstruction's keys in each strictness; `length` only when it marks length."""

    length: tuple | None
    palatal: tuple
    loose: tuple

    def compare(self, other):
        """Vowel length counts, but for a final vowel's, when both forms mark length
        somewhere (de Vaan's *wēlo‑ against *welō, not *spondā‑ against an unmarked
        *sponda, nor *plāng‑e/o‑ against *plāngō), palatals always (*ǵlh₂‑i‑
        against *gl̥Hyós). Keys that agree only once both are folded are `loose`; like
        `cut` ones, they stand only with shared evidence (*frāter‑ against *frātēr)."""
        strict = (
            compare_keys(self.length, other.length)
            if self.length and other.length
            else compare_keys(self.palatal, other.palatal)
        )
        if strict:
            return strict
        return "loose" if compare_keys(self.loose, other.loose) else None


# What a pair's evidence may be, strongest first (`matching.Linkage`). Reflexes and
# ancestors both count matched lineage, so they are one field.
LATIN_FIELDS = {
    "key": ("equal", "near"),
    "lineage": ("an ancestor shared", "cited, none shared", "none cited"),
}
ETYMON_FIELDS = {
    "key": ("equal", "near", "cut", "loose", "close"),
    "lineage": (
        "reflexes shared",
        "a reflex shared",
        "an ancestor shared",
        "cited, none shared",
        "none cited",
    ),
}
WEAK = ("cut", "loose", "close")
# The schema's name for each key level.
KEY_BASIS = {"equal": "equal", "near": "near", "cut": "near", "loose": "near", "close": "different"}
THRESHOLD = 0.5  # a candidate's probability of being the record's word, to be kept
MARGIN = 0.3  # log2 odds by which the best candidate must beat the next to stand alone
# The learnt key distance below which a record's lineage may align it: in a reviewed
# sample of 40 lineage alignments (lila-3-linkage), 25 of 27 within it were the same
# word, 6 of 13 beyond it.
CLOSE = 0.075
ROUNDS = 12
FIT_ROUNDS = 3
LABEL_WEIGHT = 100  # how many unjudged pairs a judged pair counts as, in the fit


def lineage(basis):
    """A pair's lineage level, from its basis."""
    rx = basis.get("reflexes", {}).get("shared", 0)
    an = basis.get("ancestors", {}).get("shared", 0)
    cited = sum(basis.get(k, {}).get("cited", 0) for k in ("reflexes", "ancestors"))
    if rx >= 2:
        return "reflexes shared"
    if rx:
        return "a reflex shared"
    if an:
        return "an ancestor shared"
    return "cited, none shared" if cited else "none cited"


class Matcher:
    """The rule `wety-match-lila@3` (see the module's docstring)."""

    def __init__(
        self, lila, wikt, lila_profile, wikt_profile, judgements=None, distance=None, fit=True
    ):
        self.lila, self.wikt = lila, wikt
        self.judgements = judgements or Judgements()
        self.distance = distance or Distance()
        self.profiles = {"lila": variants(lila_profile), "wikt": variants(wikt_profile)}
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
                    for k in self.keys[rid].loose[1]:
                        self.proto[code][k].add(rid)
        self.lila_keys = {
            rid: self.proto_keys(r["form"], "lila")
            for rid, r in lila.records.items()
            if r["language"]["code"] != "la" and r["form"]
        }
        # each LiLa record's records below it in de Vaan's chains
        self.below = collections.defaultdict(set)
        for rid, ancestors in lila.ancestors.items():
            for a in ancestors:
                self.below[a].add(rid)
        self.reflex_candidates = {
            rid: set(self.latin_candidates(r)[1])
            for rid, r in lila.records.items()
            if r["language"]["code"] == "la" and r["form"]
        }
        self.groups = self.homographs()
        missing = {"lineage": "none cited"}
        self.models = {
            "la": Linkage(LATIN_FIELDS, missing),
            "proto": Linkage(ETYMON_FIELDS, missing),
        }
        self.fit = fit
        self.results, self.rounds = {}, []

    def proto_keys(self, form, side):
        """A reconstruction's keys in each strictness, as written and with every ending
        cut too."""
        profiles = self.profiles[side]

        def pair(name):
            # A final vowel's length is the citation form's (de Vaan's thematic *‑e/o‑
            # against Wiktionary's *‑ō), not the word's: it is not compared.
            every, written = profiles[name]
            return tuple(
                frozenset(k.removesuffix(LENGTH) for k in key(form, p).keys)
                for p in (written, every)
            )

        marks = MACRON in unicodedata.normalize("NFD", form)
        return ProtoKeys(pair("length") if marks else None, pair("palatal"), pair("loose"))

    def homographs(self):
        """LiLa's records that share a form and a language: de Vaan's distinct words of one
        form, which `matching.assign` keeps from sharing a target."""
        by = collections.defaultdict(list)
        for rid, r in self.lila.records.items():
            if r["form"]:
                f = unicodedata.normalize("NFC", r["form"])
                by[(r["language"]["code"], f)].append(rid)
        return [sorted(g) for g in by.values() if len(g) > 1]

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

    def ancestry(self, record, candidate):
        """Of the record's ancestors in LiLa that have a target, how many are among the
        candidate's ancestors in wety; None when de Vaan gives the record no ancestor."""
        mine = self.lila.ancestors.get(record["id"])
        if not mine:
            return None
        cited = [t for t in map(self.target, mine) if t]
        theirs = self.wikt.ancestors(candidate) if cited else set()
        return {"shared": sum(t in theirs for t in cited), "cited": len(cited)}

    def latin_pairs(self, record):
        strength, found = self.latin_candidates(record)
        out = []
        for c in found:
            basis = {"key": strength}
            ancestry = self.ancestry(record, c)
            if ancestry:
                basis["ancestors"] = ancestry
            out.append((c, {"key": strength, "lineage": lineage(basis)}, basis))
        return out

    def structural(self, record):
        """The Wiktionary records of the record's language above the targets of the LiLa
        records below it: where wety's lineage puts de Vaan's etymon, whatever its form."""
        code = record["language"]["code"]
        out = set()
        for below in self.below.get(record["id"], ()):
            t = self.target(below)
            if t:
                out |= {
                    a
                    for a in self.wikt.ancestors(t)
                    if self.wikt.records.get(a, {}).get("language", {}).get("code") == code
                }
        return out

    def etymon_pairs(self, record):
        """Candidates by key; failing any, by lineage (`structural`), kept when their key
        is within `CLOSE` by the learnt distance. Lineage fills the gaps keys leave, and
        never outranks a key: an entry wety places there does not displace a record of
        de Vaan's own form."""
        code = record["language"]["code"]
        mine = self.lila_keys[record["id"]]
        shapes = SHAPES[shape(record["form"], True)]

        def fits(r):
            return shape(self.wikt.records[r]["form"], False) in shapes

        by_key = {}
        for k in mine.loose[1]:
            for r in self.proto[code].get(k, ()):
                if r not in by_key and fits(r):
                    strength = mine.compare(self.keys[r])
                    if strength:
                        by_key[r] = strength
        out = self.weigh(record, by_key)
        if out:
            return out
        by_lineage = {}
        for r in self.structural(record):
            if r in self.keys and fits(r):
                strength = mine.compare(self.keys[r])
                if not strength:
                    d, _, _ = closest(
                        {k for k in mine.loose[1] if long_enough(k)},
                        {k for k in self.keys[r].loose[1] if long_enough(k)},
                        self.distance,
                    )
                    strength = "close" if d is not None and d <= CLOSE else None
                if strength:
                    by_lineage[r] = strength
        return self.weigh(record, by_lineage)

    def weigh(self, record, found):
        """Each candidate's evidence and basis."""
        reflexes = sorted(self.lila.reflexes.get(record["id"], ()))
        out = []
        for c, strength in sorted(found.items()):
            desc = self.wikt.descendants(c)
            basis = {"key": KEY_BASIS[strength]}
            if reflexes:
                shared = sum(bool(self.reflex_candidates.get(r, set()) & desc) for r in reflexes)
                basis["reflexes"] = {"shared": shared, "cited": len(reflexes)}
            ancestry = self.ancestry(record, c)
            if ancestry:
                basis["ancestors"] = ancestry
            # A weak key is a candidate only with shared lineage behind it: a shared reflex
            # when de Vaan cites any (an ancestor shared at the root does not make *lēg‑
            # *legō), else a shared ancestor. Weights add up, so they cannot say that a
            # weak key and no lineage together make another word; this does.
            direct = basis["reflexes"]["shared"] if reflexes else (ancestry or {}).get("shared")
            if strength in WEAK and not direct:
                continue
            features = {"key": strength, "lineage": lineage(basis)}
            out.append((c, features, basis))
        return out

    def pairs(self, record):
        if record["language"]["relation"] != "exact" or not record["form"]:
            return []
        if record["language"]["code"] == "la":
            return self.latin_pairs(record)
        return self.etymon_pairs(record)

    def decide(self, pairs, fit=True):
        """Fits each model to every candidate pair, judged pairs held at their label
        (unless `fit` is false or the matcher does not fit), and decides every record
        (`matching.assign`)."""
        results = {}
        for name, codes in (("la", ("la",)), ("proto", ("itc-pro", "ine-pro"))):
            model = self.models[name]
            mine = {
                rid: ps
                for rid, ps in pairs.items()
                if self.lila.records[rid]["language"]["code"] in codes
            }
            flat = [(rid, c, f) for rid, ps in mine.items() for c, f, _ in ps]
            if fit and self.fit:
                model.fit(
                    [f for _, _, f in flat],
                    [self.judgements.label(rid, c) for rid, c, _ in flat],
                    label_weight=LABEL_WEIGHT,
                )
            scored = {
                rid: [(c, model.probability(f)) for c, f, _ in ps] for rid, ps in mine.items() if ps
            }
            found = assign(self.groups, scored, THRESHOLD, MARGIN)
            for rid, ps in mine.items():
                kept = found.get(rid) or []
                if not kept:
                    results[rid] = None
                    continue
                _, f, basis = next(p for p in ps if p[0] == kept[0])
                # whether the decision rests on lineage: without it, would it stand?
                plain = [(c, model.probability({**f2, "lineage": "none cited"})) for c, f2, _ in ps]
                results[rid] = {
                    "found": kept,
                    "score": round(dict(scored[rid])[kept[0]], 3),
                    "basis": basis,
                    "features": f,
                    "rests on lineage": decide(plain, THRESHOLD, MARGIN) != kept,
                }
        return results

    def run(self):
        """Every LiLa record's match result. Each round weighs the targets the round before
        decided (ancestors for a record, targets below an etymon for its structural
        candidates). The first `FIT_ROUNDS` refit the weights; later ones keep them, so
        that the decisions settle, until a round decides as one before it."""
        seen = []
        for n in range(ROUNDS):
            pairs = {rid: self.pairs(r) for rid, r in sorted(self.lila.records.items())}
            results = self.decide(pairs, fit=n < FIT_ROUNDS)
            changed = sum(
                (results.get(r) or {}).get("found") != (self.results.get(r) or {}).get("found")
                for r in results
            )
            self.results = results
            self.rounds.append(changed)
            state = {r: (v or {}).get("found") for r, v in results.items()}
            if n >= FIT_ROUNDS and state in seen:
                break
            seen.append(state)
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


SUBSTANCE = ("parents", "form", "relation", "mode", "hedge", "edition")


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
        - `form`: Wiktionary's parent is de Vaan's ancestor, aligned by lineage, but
          reconstructed otherwise (the match's key is `different`: *lēks against *lēg‑).
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
                if self.results[ancestors[j]]["basis"]["key"] == "different":
                    differences.append("form")
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
            if mine.loose[1] & theirs.loose[1]:
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
                    candidate = (99, la, None, kind, ancestors)
                else:
                    weight = sum(d in SUBSTANCE for d in differences)
                    candidate = (weight, la, differences, kind, ancestors)
                if best is None or candidate[0] < best[0]:
                    best = candidate
        if best is None:
            row["outcome"] = (
                "no LiLa counterpart: the matched record is in no etymology of de Vaan's"
            )
            return
        _, la, differences, kind, ancestors = best
        row["lila"], row["kind"] = la["id"], kind
        # A match that wety's own lineage decided cannot confirm that lineage: the
        # comparison is marked when the word's match, or its parent's, rests on it.
        involved = [rid for rid in mine] + [
            r for r in ancestors if self.target(r) in link["parents"]
        ]
        row["rests on lineage"] = any(
            (self.results.get(r) or {}).get("rests on lineage") for r in involved
        )
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


KINDS = {"la": "Latin entry", "itc-pro": "Proto-Italic etymon", "ine-pro": "PIE etymon"}
REVIEW_SIZE = 100
UNMATCHED_SIZE = 20
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
            "agree in substance, by a match resting on Wiktionary's lineage": sum(
                r["rests on lineage"] for r in chosen if r not in substance
            ),
            "differences": dict(
                collections.Counter(d for r in chosen for d in r["differences"]).most_common()
            ),
            "kinds": dict(collections.Counter(r["kind"] for r in chosen).most_common()),
        }
    return out


def review_row(matcher, record, m):
    """A record and what it found, as a reviewer reads them."""
    lila, wikt = matcher.lila, matcher.wikt
    r = lila.records[record]
    ext = r.get("x", {}).get(NAME, {})
    found = outcome(m)
    involved = [found] if isinstance(found, str) else list(found or ())
    return {
        "record": record,
        "kind": KINDS[r["language"]["code"]],
        "form": r["form"],
        "entry": [c for c in ext.get("comments", []) if c.startswith("Etymology for")],
        "ancestors": sorted(lila.records[x]["form"] for x in lila.ancestors.get(record, ())),
        "reflexes": sorted(lila.records[x]["form"] for x in lila.reflexes.get(record, ())),
        "found": found,
        "score": m and m["score"],
        "basis": m and m["basis"],
        "wiktionary_parents": {w: sorted(wikt.parents.get(w, ())) for w in involved},
        "judgement": None,
        "answer": None,
        "cause": None,
    }


def sample(matcher, objs):
    """Matches to review, drawn where the judgements say nothing yet: `REVIEW_SIZE` with a
    target, stratified by kind, key and lineage so that rare kinds of evidence are read
    too, and `UNMATCHED_SIZE` records left unmatched, for recall."""
    rng = random.Random(REVIEW_SEED)
    matches = {o["record"]: o for o in objs if o["type"] == "match"}
    strata = collections.defaultdict(list)
    for rid, m in sorted(matches.items()):
        if "target" in m and matcher.judgements.verdict(rid, m["target"]) is None:
            res = matcher.results[rid]
            kind = matcher.lila.records[rid]["language"]["code"]
            strata[(kind, res["features"]["key"], res["features"]["lineage"])].append(rid)
    total = sum(map(len, strata.values()))
    # every stratum gets one, the rest in proportion to size
    quota = {k: 1 + round((REVIEW_SIZE - len(strata)) * len(v) / total) for k, v in strata.items()}
    chosen = [
        rid for k, v in sorted(strata.items()) for rid in rng.sample(v, min(quota[k], len(v)))
    ]
    unmatched = sorted(
        rid
        for rid, r in matcher.lila.records.items()
        if rid not in matches and r["form"] and r["language"]["relation"] == "exact"
    )
    chosen += rng.sample(unmatched, min(UNMATCHED_SIZE, len(unmatched)))
    return [review_row(matcher, rid, matches.get(rid)) for rid in chosen]


def outcome(m):
    """A match object's outcome: its target, or its candidates; None for no match."""
    if m is None:
        return None
    return m["target"] if "target" in m else tuple(m["candidates"])


def change_kind(before, after):
    if before is None:
        return "gained"
    if after is None:
        return "lost: was a target" if isinstance(before, str) else "lost: was ambiguous"
    if isinstance(before, str):
        return "retargeted" if isinstance(after, str) else "target to ambiguous"
    if isinstance(after, str):
        return "resolved" if after in before else "resolved outside the candidates"
    return "candidates changed"


def changes(matcher, objs, earlier):
    """Every LiLa record whose match differs from an earlier check's, as a reviewer reads
    it. Earlier reviews are checked through the judgements, not here."""
    before = {o["record"]: o for o in A.read(earlier / "assertions.jsonl") if o["type"] == "match"}
    after = {o["record"]: o for o in objs if o["type"] == "match"}
    rows = []
    for rid in sorted(set(before) | set(after)):
        b, a = outcome(before.get(rid)), outcome(after.get(rid))
        if a == b:
            continue
        row = review_row(matcher, rid, after.get(rid))
        row["change"] = change_kind(b, a)
        row["before"], row["after"] = b, a
        row["judged"] = matcher.judgements.verdict(rid, a)
        rows.append(row)
    counts = collections.Counter(f"{r['kind']}: {r['change']}" for r in rows)
    return rows, dict(sorted(counts.items()))


def precision(rows):
    """Of a review's judged rows, how many leave the record right: a target that stands
    for de Vaan's word, candidates among which it is, or no match where none does."""
    judged = [r for r in rows if r["judgement"]]
    by = collections.defaultdict(collections.Counter)
    for r in judged:
        found = r["found"]
        shape = "target" if isinstance(found, str) else "candidates" if found else "no match"
        by[f"{r['kind']}, {shape}"][r["judgement"]] += 1
    right = sum(r["judgement"] == "right" for r in judged)
    return {
        "reviewed": len(judged),
        "right": right,
        "precision": round(right / len(judged), 3) if judged else None,
        "by kind and outcome": {k: dict(v) for k, v in sorted(by.items())},
        "wrong": [
            {"record": r["record"], "form": r["form"], "found": r["found"], "cause": r["cause"]}
            for r in judged
            if r["judgement"] != "right"
        ],
    }


def mining_pairs(matcher):
    """Pairs for `etymology.notation`: etymons whose target shares a reflex, or that a
    reviewer judged the same word, with both forms' keys (every ending cut)."""
    out = []
    for rid, res in sorted(matcher.results.items()):
        if not res or len(res["found"]) != 1 or rid not in matcher.lila_keys:
            continue
        t = res["found"][0]
        confirmed = res["features"]["lineage"] in ("reflexes shared", "a reflex shared")
        if confirmed or matcher.judgements.label(rid, t):
            mine, theirs = matcher.lila_keys[rid], matcher.keys[t]
            label = f"{matcher.lila.records[rid]['form']} : {matcher.wikt.records[t]['form']}"
            out.append([sorted(mine.loose[1]), sorted(theirs.loose[1]), label])
    return out


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    p.add_argument("--export", type=Path, help="a graph's assertion export (graph-assertions)")
    p.add_argument("--out", type=Path, required=True, help="the check directory")
    p.add_argument(
        "--compare", type=Path, help="an earlier check directory: write what changed against it"
    )
    p.add_argument(
        "--review",
        action="store_true",
        help="only add the judged sample (review.json) and changes (changes.json) to the "
        "judgements, and score the check against them",
    )
    args = p.parse_args(argv)
    result_path = args.out / "result.json"
    if args.review:
        result = json.loads(result_path.read_text())
        reviews = [args.out / f for f in ("review.json", "changes.json")]
        reviews = [r for r in reviews if r.exists()]
        for r in reviews:
            rows = json.loads(r.read_text())["rows"]
            result[f"precision of {r.name}"] = precision(rows)
            print(
                r.name, json.dumps(result[f"precision of {r.name}"], ensure_ascii=False, indent=1)
            )
        n = harvest_into(JUDGEMENTS, reviews)
        found = found_in(A.read(args.out / "assertions.jsonl"))
        result["judgements"] = Judgements.read(JUDGEMENTS).score(found)
        print(f"{n} new judgements; regressions: {len(result['judgements']['regressions'])}")
        result_path.write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n")
        return
    lila_path = LILA / "checks/lila-2/assertions.jsonl"
    profiles = (LILA / "etc/orthography.tsv", WIKTIONARY_PROFILE)
    inputs = [args.export, lila_path, *profiles, JUDGEMENTS, SUBSTITUTIONS]
    extraction = A.extraction(
        "converter", "packages/etymology/match_lila.py", [i for i in inputs if i.exists()]
    )
    wikt = Wiktionary(args.export)
    lila = Lila(A.read(lila_path))
    judgements = Judgements.read(JUDGEMENTS)
    matcher = Matcher(
        lila, wikt, *map(Profile.read, profiles), judgements, Distance.read(SUBSTITUTIONS)
    )
    matcher.run()
    relayer = Relayer(matcher)
    relayer.run()
    rule = {
        "type": "agent",
        "id": RULE,
        "kind": "rule",
        "name": "wety's LiLa matcher, v3: de Vaan's records in LiLa matched to Wiktionary's "
        "by form, comparison key, learnt key distance, cited reflexes and ancestors, weighed "
        "as Fellegi and Sunter do and learnt from the judgements; and Wiktionary's credits to "
        "de Vaan (2008) relayed to his assertions in LiLa",
    }
    made = [rule, extraction, *matcher.objects(extraction["id"]), *relayer.relays]
    pool = {o["id"]: o for o in [*lila.objs.values(), *wikt.objects(), *made]}
    out = closure(made, pool)
    args.out.mkdir(parents=True, exist_ok=True)
    n = out.write(args.out / "assertions.jsonl")
    objs = list(out.objects.values())
    errors = A.validate(objs)
    result = {
        "milestone": "lila-3",
        "question": "Which of LiLa's de Vaan records stand for which of wety's items, and do "
        "Wiktionary's assertions that credit de Vaan (2008) say what de Vaan says?",
        "extraction": extraction["id"],
        "objects": n,
        "errors": len(errors),
        "matches": match_counts(lila, matcher.results),
        "rounds": matcher.rounds,
        "weights": {name: m.table() for name, m in matcher.models.items()},
        "resting on lineage": dict(
            collections.Counter(
                f"{KINDS[lila.records[r]['language']['code']]}: {res['rests on lineage']}"
                for r, res in matcher.results.items()
                if res
            )
        ),
        "judgements": judgements.score(found_in(objs)),
        "relays": relay_counts(relayer.rows),
        "de Vaan's works in the graph": {
            w["id"]: w["citation"] for w in sorted(wikt.works.values(), key=lambda w: w["id"])
        },
    }
    if args.compare:
        rows, counts = changes(matcher, objs, args.compare)
        against = str(args.compare.resolve().relative_to(A.REPO))
        result["changes"] = {"against": against, "records": len(rows), "by kind": counts}
        (args.out / "changes.json").write_text(
            json.dumps({"against": against, "rows": rows}, ensure_ascii=False, indent=1) + "\n"
        )
    result_path.write_text(json.dumps(result, ensure_ascii=False, indent=1) + "\n")
    for name, data in (
        ("relays.json", relayer.rows),
        ("sample.json", {"seed": REVIEW_SEED, "rows": sample(matcher, objs)}),
        ("pairs.json", mining_pairs(matcher)),
    ):
        (args.out / name).write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n")
    print(f"{args.out}: {n} objects, {len(errors)} errors")
    for e in errors[:20]:
        print("  " + e)


if __name__ == "__main__":
    main()
