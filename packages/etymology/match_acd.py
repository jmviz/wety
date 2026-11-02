"""ACD records matched to Wiktionary's, and Wiktionary's assertions that credit the ACD
relayed to the ACD's own (acd-3).

Steps, each writing into a work directory so that later steps rerun quickly:

- `scan`: the Austronesian part of a graph's assertion export (records of an Austronesian
  language, the hypotheses that name one, the assertions on those hypotheses or crediting
  a work, every work and agent), and the glosses of Austronesian entries from the
  wiktextract data the graph was built from;
- `match`: each ACD record of a language the ACD's language map calls `exact` matched to
  a Wiktionary record of that code, by the rule `agent:wety-match-acd@1`;
- `relay`: each Wiktionary assertion that credits the ACD or Blust linked to the ACD's
  own assertion of the same hypothesis, read through the matches.
"""

import argparse
import collections
import gzip
import json
import random
import re
import subprocess
import unicodedata
from functools import cached_property
from pathlib import Path

from etymology import assertions as A
from etymology import orthography
from etymology.assertions import REPO

WIKTIONARY_EXPORT = (
    REPO
    / "data/artifacts/wiktionary-export/2d98b7aa0c9965e1492db2e242ead832fe7e3c161e1f1a27c1d177135c6df2b7/files"
)
SOURCES = REPO / "data/workspaces/etymology/sources"
ACD = SOURCES / "acd"
PROFILES = {
    ("acd", True): ACD / "etc/orthography.tsv",
    ("acd", False): ACD / "etc/orthography-attested.tsv",
    ("wiktionary", True): SOURCES / "wiktionary/etc/orthography-map.tsv",
    ("wiktionary", False): SOURCES / "wiktionary/etc/orthography-map-attested.tsv",
}
RULE = "agent:wety-match-acd@1"
RELAY_RULE = "agent:wety-relay-acd@1"
RECORD_CODE = re.compile(r"^wiktionary:([^:]+):")
LANG_CODE = re.compile(rb'"lang_code": "([^"]+)"')


def austronesian_codes(export=WIKTIONARY_EXPORT, language_map=ACD / "etc/languages.json"):
    """Wiktionary's codes of the Austronesian family (languages whose family is `map` or
    below it, and its proto-languages), with every code the ACD's language map names."""
    langs = json.loads((export / "languages.json").read_text())
    fams = json.loads((export / "families.json").read_text())
    families = {f for f, v in fams.items() if f == "map" or "map" in v["superfamilies"]}
    codes = {c for c, v in langs.items() if v.get("family") in families}
    codes |= {fams[f]["protoLanguage"] for f in families if fams[f].get("protoLanguage")}
    for row in json.loads(Path(language_map).read_text())["rows"]:
        codes |= {c for c in [row["code"], *row.get("candidates", [])] if c}
    return codes


def record_code(record_id):
    m = RECORD_CODE.match(record_id)
    return m.group(1) if m else None


def lines(path):
    """The lines of a gzipped file, decompressed by gzip(1), which is faster than Python's."""
    with subprocess.Popen(["gzip", "-dc", str(path)], stdout=subprocess.PIPE, bufsize=1 << 20) as p:
        yield from p.stdout


def scan_export(export, codes, out):
    """The Austronesian part of a graph's assertion export, to `out` (JSON Lines, gzipped).

    Kept: source, agents, works, extractions and alternative sets whole; records of an
    Austronesian code; hypotheses naming such a record; assertions whose target is a kept
    hypothesis, or that credit a work (filtered once every hypothesis is known). A second
    pass adds the other records the kept hypotheses name."""
    keep, assertions, kept_hyps, named = [], [], set(), set()
    for line in lines(export):
        o = json.loads(line)
        t = o["type"]
        if t == "record":
            if record_code(o["id"]) in codes:
                keep.append(o)
        elif t == "hypothesis":
            names = {r for link in o["links"] for r in (link["child"], *link["parents"])}
            if any(record_code(r) in codes for r in names):
                keep.append(o)
                kept_hyps.add(o["id"])
                named |= names
        elif t == "assertion":
            if o.get("credits") or record_code(o["locator"].split("/")[0]) in codes:
                assertions.append(o)
        else:
            keep.append(o)
    keep += [a for a in assertions if a["target"] in kept_hyps or a["target"].startswith("alt:")]
    missing = {r for r in named if record_code(r) not in codes}
    for line in lines(export):
        if line.startswith(b'{"form"') or b'"type":"record"' in line:
            o = json.loads(line)
            if o["type"] == "record" and o["id"] in missing:
                keep.append(o)
    with gzip.open(out, "wt", encoding="utf-8") as f:
        f.writelines(json.dumps(o, ensure_ascii=False, sort_keys=True) + "\n" for o in keep)
    return len(keep)


def scan_glosses(wiktextract, codes, out):
    """The glosses of every Austronesian entry in wiktextract's data, keyed by code, page
    title and etymology number, to `out` (JSON Lines, gzipped).

    The graph's own record ids key a word by its canonical form, which for Austronesian
    entries is almost always the page title; the canonical form is kept beside it."""
    wanted = {c.encode() for c in codes}
    n = 0
    with gzip.open(out, "wt", encoding="utf-8") as f:
        for line in lines(wiktextract):
            if not wanted.intersection(LANG_CODE.findall(line)):
                continue
            d = json.loads(line)
            if d.get("lang_code") not in codes:
                continue
            canonical = next(
                (x["form"] for x in d.get("forms", []) if "canonical" in x.get("tags", [])), None
            )
            glosses = [g for s in d.get("senses", []) for g in s.get("glosses", [])[-1:]]
            row = {
                "code": d["lang_code"],
                "word": d.get("word"),
                "canonical": canonical,
                "ety": str(d.get("etymology_number") or ""),
                "pos": d.get("pos"),
                "glosses": glosses,
                "etymology": d.get("etymology_text"),
            }
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
            n += 1
    return n


STOP = {
    "a",
    "an",
    "the",
    "of",
    "to",
    "in",
    "on",
    "for",
    "with",
    "or",
    "and",
    "be",
    "is",
    "as",
    "by",
    "at",
    "from",
    "into",
    "its",
    "it",
    "his",
    "her",
    "one",
    "someone",
    "something",
    "etc",
    "sp",
    "kind",
    "type",
    "used",
    "also",
    "esp",
    "especially",
    "sort",
    "which",
    "that",
    "who",
    "make",
    "made",
    "do",
    "get",
}
ALTERNATIVES = re.compile(r"\s*(?:~|,|//| or )\s*")
GLOTTAL = "ʔ"


class Keys:
    """Comparison keys of forms, from each source's orthography profile: one profile for
    reconstructions, one for attested forms. The loose key also drops glottal stops,
    which one orthography writes and another omits."""

    def __init__(self, profiles=PROFILES):
        self.profiles = {k: orthography.Profile.read(p) for k, p in profiles.items()}
        self.cache = {}

    def __call__(self, source, reconstructed, form):
        k = (source, reconstructed, form)
        if k not in self.cache:
            text = " / ".join(f for f in ALTERNATIVES.split(form.strip()) if f)
            keys = orthography.key(text, self.profiles[(source, reconstructed)]).keys
            self.cache[k] = frozenset(keys)
        return self.cache[k]


def loose(keys):
    return frozenset(k.replace(GLOTTAL, "") for k in keys)


def near(a, b):
    """Two keys of four or more segments one edit apart."""
    if min(len(a), len(b)) < 4 or abs(len(a) - len(b)) > 1:
        return False
    if len(a) == len(b):
        return sum(x != y for x, y in zip(a, b)) == 1
    short, long = sorted((a, b), key=len)
    return any(long[:i] + long[i + 1 :] == short for i in range(len(long)))


def tokens(gloss):
    words = re.findall(r"[a-z]+", unicodedata.normalize("NFKD", gloss.lower()))
    out = set()
    for w in words:
        if w in STOP or len(w) < 2:
            continue
        for end in ("ing", "ed", "es", "s"):
            if w.endswith(end) and len(w) - len(end) >= 3:
                w = w[: -len(end)]
                break
        out.add(w)
    return out


def gloss_similarity(a, b):
    """The share of the shorter gloss's content words the other gloss has, or None when
    either has none."""
    ta, tb = tokens(a or ""), tokens(b or "")
    if not ta or not tb:
        return None
    return round(len(ta & tb) / min(len(ta), len(tb)), 2)


class Acd:
    """The ACD's assertions (acd-2) and its tree."""

    def __init__(self, objects, tree=None):
        self.records, self.sets, self.hyps, self.assertions = {}, {}, {}, []
        self.objects = {}
        if tree is not None:
            self.tree = tree
        for o in objects:
            self.objects[o["id"]] = o
            t = o["type"]
            if t == "record":
                self.records[o["id"]] = o
            elif t == "cognate_set":
                self.sets[o["id"]] = o
            elif t == "hypothesis":
                self.hyps[o["id"]] = o
            elif t == "assertion":
                self.assertions.append(o)
        self.sets_of = collections.defaultdict(list)
        for s in self.sets.values():
            if s["category"].startswith("cognate set"):
                for r in [s["reconstruction"], *s["members"]]:
                    if r:
                        self.sets_of[r].append(s["id"])
        # the ACD's (compiled) parents of each record, and the assertions on its origin
        self.parents = collections.defaultdict(set)
        self.origin = collections.defaultdict(list)
        for a in self.assertions:
            h = self.hyps.get(a["target"])
            if h and a["stance"] == "endorses":
                self.parents[h["word"]].update(h["links"][0]["parents"])
                self.origin[h["word"]].append(a)

    @cached_property
    def tree(self):
        from etymology.convert.acd import Release

        return Release().tree

    @staticmethod
    def language_id(record_id):
        return record_id.split(":", 1)[1].split("-", 1)[0]

    def reflexes(self, record_id):
        """The attested reflexes the ACD cites for a reconstruction: the attested members
        of each cognate set it belongs to that descend, by the ACD's own link, from the
        set's reconstruction, and whose language the tree places below the record's."""
        level = self.language_id(record_id)
        found = set()
        for set_id in self.sets_of[record_id]:
            s = self.sets[set_id]
            for m in s["members"]:
                r = self.records[m]
                if r["reconstructed"] or s["reconstruction"] not in self.parents[m]:
                    continue
                if level in self.tree.above(self.language_id(m)):
                    found.add(m)
        return found


class Wiktionary:
    """The Austronesian part of the graph's assertions (`scan`), with glosses."""

    def __init__(self, objects, glosses):
        self.objects = {}
        self.records, self.hyps, self.assertions = {}, {}, []
        for o in objects:
            self.objects[o["id"]] = o
            t = o["type"]
            if t == "assertion":
                self.assertions.append(o)
            elif t == "record":
                self.records[o["id"]] = o
            elif t == "hypothesis":
                self.hyps[o["id"]] = o
        self.children = collections.defaultdict(set)
        self.by_word = collections.defaultdict(list)
        for a in self.assertions:
            h = self.hyps.get(a["target"])
            if h and a["stance"] == "endorses":
                self.by_word[h["word"]].append(a)
                for link in h["links"]:
                    for p in link["parents"]:
                        self.children[p].add(link["child"])
        self.glosses = collections.defaultdict(list)
        self.etymologies = {}
        for g in glosses:
            n = g["ety"].split(".")[0] or "1"
            for term in {g["word"], g["canonical"]} - {None}:
                key = f"wiktionary:{g['code']}:{term}#{n}"
                self.glosses[key] += g["glosses"]
                if g.get("etymology"):
                    self.etymologies[key] = g["etymology"]

    def origins(self, record_id):
        """The hypotheses on a record's origin that some assertion endorses."""
        return [
            self.hyps[a["target"]]
            for a in self.by_word.get(record_id, ())
            if a["target"] in self.hyps
        ]

    def gloss(self, record_id):
        return "; ".join(self.glosses.get(record_id.rstrip("?"), []))

    def descendants(self, record_id, limit=5000):
        seen, todo = set(), [record_id]
        while todo and len(seen) < limit:
            for c in self.children.get(todo.pop(), ()):
                if c not in seen:
                    seen.add(c)
                    todo.append(c)
        return seen


class Matcher:
    """The rule `wety-match-acd@1`: each ACD record of an `exact` language to a Wiktionary
    record of the same code.

    Candidates are the Wiktionary records of that code whose comparison key equals the
    ACD record's; failing that, for an attested form, whose loose key (glottal stops
    dropped) does; failing that, for a reconstruction, whose key is one edit away.

    - A reconstruction is weighed by its reflexes: of the attested reflexes the ACD
      cites for it (`Acd.reflexes`, one per language and key), how many are among the
      candidate's descendants in wety. A candidate needs a shared reflex or a gloss
      similarity of at least `GLOSS_MIN`; one at a near key needs two shared reflexes.
    - An attested form is weighed by its gloss alone: one candidate is taken unless its
      gloss shares nothing with the ACD's (both known); among several, the one whose gloss
      is closer by `MARGIN`.
    - Ties: a Wiktionary entry is preferred to an imputed record (a word wety knows
      only from citations) with the same evidence; otherwise equal fits are left as
      candidates. Of the ACD's homographs (same language and form, different subscript)
      that fit one Wiktionary record, only the best-fitting is matched; equal fits are
      left unmatched.
    """

    GLOSS_MIN = 0.25
    MARGIN = 0.2

    def __init__(self, acd, wiktionary, keys, language_map=None, exact=None):
        self.acd, self.wikt, self.keys = acd, wiktionary, keys
        self.exact = exact or {
            r["code"]
            for r in json.loads(Path(language_map).read_text())["rows"]
            if r["relation"] == "exact" and r["code"]
        }
        self.index = collections.defaultdict(lambda: collections.defaultdict(set))
        self.loose_index = collections.defaultdict(lambda: collections.defaultdict(set))
        for r in wiktionary.records.values():
            code = r["language"]["code"]
            if code not in self.exact or r["form"] is None:
                continue
            ks = keys("wiktionary", r["reconstructed"], r["form"])
            for k in ks:
                self.index[code][k].add(r["id"])
            for k in loose(ks):
                self.loose_index[code][k].add(r["id"])
        self.descendant_keys = {}

    def wiktionary_reflexes(self, record_id):
        """A Wiktionary record's attested descendants, as (code, loose key) pairs."""
        if record_id not in self.descendant_keys:
            out = set()
            for d in self.wikt.descendants(record_id):
                r = self.wikt.records.get(d)
                if r and not r["reconstructed"] and r["form"]:
                    code = r["language"]["code"]
                    out |= {(code, k) for k in loose(self.keys("wiktionary", False, r["form"]))}
            self.descendant_keys[record_id] = out
        return self.descendant_keys[record_id]

    def cited(self, record_id):
        """The ACD's cited reflexes of a reconstruction, one per language and loose key
        set, in `exact` languages."""
        out = {}
        for m in self.acd.reflexes(record_id):
            r = self.acd.records[m]
            code = r["language"]["code"]
            if r["language"]["relation"] != "exact" or not code or not r["form"]:
                continue
            ks = loose(self.keys("acd", False, r["form"]))
            if ks:
                out[(code, ks)] = m
        return out

    def candidates(self, r):
        code = r["language"]["code"]
        ks = self.keys("acd", r["reconstructed"], r["form"])
        found = set().union(*(self.index[code].get(k, ()) for k in ks)) if ks else set()
        if found:
            return found, "equal"
        if not r["reconstructed"]:
            found = set().union(*(self.loose_index[code].get(k, ()) for k in loose(ks)))
            return found, "near"
        found = {
            w
            for k in ks
            for wk, ws in self.index[code].items()
            if abs(len(wk) - len(k)) <= 1 and near(k, wk)
            for w in ws
        }
        return found, "near"

    def evaluate(self, r):
        """Each candidate's basis, or None for a record no rule considers."""
        lang = r["language"]
        if lang["relation"] != "exact" or lang["code"] not in self.exact or not r["form"]:
            return None
        found, key = self.candidates(r)
        cited = self.cited(r["id"]) if r["reconstructed"] else None
        out = []
        for w in sorted(found):
            basis = {"key": key}
            g = gloss_similarity(r.get("gloss"), self.wikt.gloss(w))
            if g is not None:
                basis["gloss"] = g
            if cited is not None:
                descendants = self.wiktionary_reflexes(w)
                shared = sum(any((c, k) in descendants for k in ks) for c, ks in cited)
                basis["reflexes"] = {"shared": shared, "cited": len(cited)}
            out.append((w, basis))
        return out

    @staticmethod
    def score(basis, reconstructed):
        k = 1.0 if basis["key"] == "equal" else 0.5
        g = basis.get("gloss", 0)
        if reconstructed:
            rx = basis["reflexes"]
            support = min(1.0, rx["shared"] / max(1, min(rx["cited"], 3)))
            return round(0.4 * k + 0.4 * support + 0.2 * g, 2)
        return round(0.6 * k + 0.4 * g, 2)

    def eligible(self, basis, reconstructed):
        g = basis.get("gloss")
        if reconstructed:
            shared = basis["reflexes"]["shared"]
            if basis["key"] == "near":
                return shared >= 2
            return shared >= 1 or (g is not None and g >= self.GLOSS_MIN)
        return g is None or g > 0

    def rank(self, w, basis):
        rx = basis.get("reflexes", {}).get("shared", 0)
        return (rx, basis.get("gloss") or 0, not w.endswith("?"))

    def decide(self, r):
        """("match", target, basis) | ("ambiguous", candidates, basis) | ("unmatched", why, basis)."""
        found = self.evaluate(r)
        if found is None:
            return ("skipped", "language not exact", None)
        if not found:
            return ("unmatched", "no Wiktionary record with this key", None)
        ok = [(w, b) for w, b in found if self.eligible(b, r["reconstructed"])]
        if not ok:
            return ("unmatched", "candidates without support", found[0][1])
        ok.sort(key=lambda x: self.rank(*x), reverse=True)
        if len(ok) == 1:
            return ("match", ok[0][0], ok[0][1])
        (w1, b1), (w2, b2) = ok[:2]
        r1, r2 = self.rank(w1, b1), self.rank(w2, b2)
        if r1[0] > r2[0] or r1[1] - r2[1] >= self.MARGIN:
            return ("match", w1, b1)
        if r1[:2] == r2[:2] and r1[2] != r2[2]:
            return ("match", w1, b1)
        tied = [
            w
            for w, b in ok
            if self.rank(w, b)[0] == r1[0] and r1[1] - self.rank(w, b)[1] < self.MARGIN
        ]
        return ("ambiguous", tied, b1)

    def run(self):
        decisions = {}
        for r in self.acd.records.values():
            decisions[r["id"]] = self.decide(r)
        self.resolve_homographs(decisions)
        return decisions

    def beats(self, a, b):
        """Whether rank `a` is a better fit than rank `b`."""
        return a[0] > b[0] or (a[0] == b[0] and a[1] - b[1] >= self.MARGIN)

    def resolve_homographs(self, decisions):
        """Of the ACD's homographs matched to one Wiktionary record, keep the best fit;
        records with the very same form are one word, and are kept together."""
        by_target = collections.defaultdict(lambda: collections.defaultdict(list))
        for rid, (what, target, basis) in decisions.items():
            if what == "match" and self.acd.records[rid]["reconstructed"]:
                by_target[target][self.acd.records[rid]["form"]].append(rid)
        for target, groups in by_target.items():
            if len(groups) < 2:
                continue
            rank = {
                form: max(self.rank(target, decisions[x][2]) for x in rids)
                for form, rids in groups.items()
            }
            winners = [f for f in groups if not any(self.beats(rank[o], rank[f]) for o in groups)]
            for form, rids in groups.items():
                if form in winners and len(winners) == 1:
                    continue
                why = "homographs fit equally" if form in winners else "a homograph fits better"
                for x in rids:
                    decisions[x] = ("unmatched", why, decisions[x][2])


ACD_WORK = re.compile(r":ACD$|Austronesian Comparative Dictionary")
BORROWINGS = {
    "borrowed",
    "learned borrowing",
    "semi-learned borrowing",
    "unadapted borrowing",
    "orthographic borrowing",
}


def acd_works(objects):
    """Wiktionary's works that are the ACD: its `R:...:ACD` templates, and citations that
    name it or are by Blust and Trussel. Other works by Blust are not the ACD."""
    found = set()
    for o in objects.values():
        if o["type"] != "work":
            continue
        by_them = {"agent:robert-blust", "agent:stephen-trussel"} <= set(o.get("authors", []))
        if ACD_WORK.search(o["id"]) or ACD_WORK.search(o.get("citation", "")) or by_them:
            found.add(o["id"])
    return found


class Relayer:
    """The rule `wety-relay-acd@1`: each Wiktionary assertion that credits the ACD, linked
    to the ACD's own assertion on the origin of the same word, through the matches.

    The ACD links a reflex straight to its set's reconstruction, skipping the levels
    between, where Wiktionary goes one level at a time; so the two are compared by
    ancestry. Wiktionary's ancestry of the word is the assertion's chain, continued by
    Wiktionary's own etymologies of each ancestor (inherited links only).

    - parents agree when every parent the ACD names is matched to a Wiktionary ancestor,
      or, failing a match, Wiktionary has an unmatched ancestor of that parent's language
      with its key (counted apart, as found by key); they differ when Wiktionary's ancestor in that parent's language is another word
      (matched to another ACD record, or with another key);
    - the relation differs when one says inherited and the other a borrowing or a
      derivation within the language; the ACD's `ultimate` (a sub-entry from its main
      entry, steps unstated) is compatible with any derivation;
    - a hedge differs when one is hedged and the other is not.

    Of a word's several ACD origins (one per cognate set it is in), the one with fewest
    differences is relayed, a set of an affix losing ties. A word that is not matched,
    that the ACD gives no origin, or whose comparison is undetermined (Wiktionary's
    ancestry has no word in the ACD parent's language) gets no relay and is counted with
    its reason.
    """

    def __init__(self, acd, wikt, matcher, decisions):
        self.acd, self.wikt, self.matcher = acd, wikt, matcher
        self.works = acd_works(wikt.objects)
        self.to_acd = collections.defaultdict(set)
        self.to_wikt = {}
        for rid, (what, target, _) in decisions.items():
            if what == "match":
                self.to_acd[target].add(rid)
                self.to_wikt[rid] = target
        self.wikt_parents = collections.defaultdict(list)
        for a in wikt.assertions:
            h = wikt.hyps.get(a["target"])
            if h and a["stance"] == "endorses" and a["asserter"] == "agent:wiktionary-editors":
                self.wikt_parents[h["word"]].append(h)

    def crediting(self):
        for a in self.wikt.assertions:
            works = [c["work"] for c in a.get("credits", []) if c["work"] in self.works]
            if works and a["target"] in self.wikt.hyps:
                yield a, works[0]

    def ancestry(self, h):
        """Wiktionary's ancestors of a hypothesis's word: its chain, then each ancestor's
        own inherited etymologies. Each with the modes of the steps that reach it."""
        found = {}
        for link in h["links"]:
            mode = link.get("mode", link["relation"])
            before = found.get(link["child"], ())
            for p in link["parents"]:
                found.setdefault(p, (*before, mode))
        todo = list(found)
        while todo:
            r = todo.pop()
            for h2 in self.wikt_parents.get(r, ()):
                link = h2["links"][0]
                if link.get("mode") != "inherited":
                    continue
                for p in link["parents"]:
                    if p not in found:
                        found[p] = (*found[r], "inherited")
                        todo.append(p)
        return found

    def compare(self, a, h):
        """(the ACD's assertion, differences, how the parents were found) or (None, reason,
        None). Parents are found `matched`, `by key` (an unmatched Wiktionary ancestor of
        the parent's language with its key), as `another word`, or `not named` (the ACD
        names no parent)."""
        word = h["word"]
        acd_words = self.to_acd.get(word)
        if not acd_words:
            return None, "word not matched", None
        origins = [o for w in sorted(acd_words) for o in self.acd.origin.get(w, ())]
        if not origins:
            return None, "the ACD gives the matched word no origin", None
        ancestors = self.ancestry(h)
        hedged = bool(a.get("hedges"))
        results = []
        for o in origins:
            link = self.acd.hyps[o["target"]]["links"][0]
            diffs, undetermined, found, modes = [], None, "not named", None
            for p in link["parents"]:
                pr = self.acd.records[p]
                if pr["form"] is None:
                    continue
                hit = [w for w in ancestors if p in self.to_acd.get(w, ())]
                found = "matched"
                if not hit:
                    code = pr["language"]["code"]
                    same_lang = [w for w in ancestors if record_code(w) == code]
                    pk = self.matcher.keys("acd", True, pr["form"])
                    hit = [
                        w
                        for w in same_lang
                        if w not in self.to_acd
                        and w in self.wikt.records
                        and pk
                        & self.matcher.keys("wiktionary", True, self.wikt.records[w]["form"] or "")
                    ]
                    found = "by key"
                    if not hit:
                        if same_lang:
                            diffs.append("parents")
                            found = "another word"
                        else:
                            undetermined = (
                                "Wiktionary's ancestry has no word in the ACD parent's language"
                            )
                        continue
                modes = ancestors[hit[0]]
            first = h["links"][0]
            wmode = first.get("mode", first["relation"])
            if link["relation"] == "derivation":
                amode = link["mode"]
                if amode in BORROWINGS and wmode not in BORROWINGS:
                    diffs.append("mode")
                elif amode == "inherited":
                    path = modes or (wmode,)
                    if any(m != "inherited" for m in path):
                        diffs.append("mode" if first["relation"] == "derivation" else "relation")
            if hedged != bool(o.get("hedges")):
                diffs.append("hedge")
            if undetermined and "parents" not in diffs:
                results.append((None, undetermined, None))
            else:
                affix = any(
                    (self.acd.records[p]["form"] or "").strip("*").startswith("-")
                    or (self.acd.records[p]["form"] or "").endswith("-")
                    for p in link["parents"]
                )
                results.append((o, list(dict.fromkeys(diffs)), found, affix))
        determined = [r for r in results if r[0] is not None]
        if not determined:
            return results[0]
        determined.sort(key=lambda r: (len(r[1]), r[3], r[2] != "matched"))
        return determined[0][:3]

    def run(self):
        relays, outcomes = [], []
        for a, work in self.crediting():
            h = self.wikt.hyps[a["target"]]
            o, what, found = self.compare(a, h)
            outcomes.append((a, work, o, what, found))
            if o is not None:
                relays.append(A.relay(a["id"], o["id"], work, RELAY_RULE, what))
        return relays, outcomes


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
    """The seeds with every object they refer to, transitively."""
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


def level(acd, record):
    """A record's place in the ACD's tree: a reconstruction's own level, an attested
    form's lowest level above it."""
    lang = Acd.language_id(record["id"])
    if record["reconstructed"]:
        return record["language"]["label"]
    above = acd.tree.above(lang)
    name = acd.tree.languages[above[0]]["Name"] if above else "none"
    return "below " + name


def read_lines(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt", encoding="utf-8") as f:
        for line in f:
            yield json.loads(line)


def run(args):
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    keys = Keys()
    acd = Acd(read_lines(args.acd))
    wikt = Wiktionary(
        read_lines(args.work / "wiktionary.jsonl.gz"), read_lines(args.work / "glosses.jsonl.gz")
    )
    language_map = ACD / "etc/languages.json"
    matcher = Matcher(acd, wikt, keys, language_map)
    decisions = matcher.run()
    extraction = A.extraction(
        "converter",
        "packages/etymology/match_acd.py",
        [args.acd, args.export, args.wiktextract, language_map, *PROFILES.values()],
    )
    new = {extraction["id"]: extraction}
    for agent_id, name in (
        (
            RULE,
            "wety's ACD matcher, v1: ACD records to Wiktionary records by key, gloss and reflexes",
        ),
        (
            RELAY_RULE,
            "wety's ACD relay rule, v1: Wiktionary's assertions crediting the ACD to the ACD's own",
        ),
    ):
        new[agent_id] = {"type": "agent", "id": agent_id, "kind": "rule", "name": name}
    matches = []
    for rid, (what, target, basis) in decisions.items():
        r = acd.records[rid]
        if what == "match":
            m = A.match(
                rid,
                RULE,
                Matcher.score(basis, r["reconstructed"]),
                basis,
                target=target,
                extraction=extraction["id"],
            )
        elif what == "ambiguous":
            m = A.match(
                rid,
                RULE,
                Matcher.score(basis, r["reconstructed"]),
                basis,
                candidates=sorted(target),
                extraction=extraction["id"],
            )
        else:
            continue
        matches.append(m)
    relayer = Relayer(acd, wikt, matcher, decisions)
    relays, outcomes = relayer.run()
    writer = closure(matches + relays, new, acd.objects, wikt.objects)
    n = writer.write(out / "assertions.jsonl")
    print(f"{out / 'assertions.jsonl'}: {n} objects")

    rows = []
    for rid, (what, target, basis) in decisions.items():
        r = acd.records[rid]
        rows.append(
            {
                "record": rid,
                "form": r["form"],
                "gloss": r.get("gloss"),
                "code": r["language"]["code"],
                "kind": "reconstruction" if r["reconstructed"] else "attested",
                "level": level(acd, r),
                "outcome": what,
                "detail": target,
                "basis": basis,
            }
        )
    with gzip.open(out / "decisions.jsonl.gz", "wt", encoding="utf-8") as f:
        f.writelines(json.dumps(x, ensure_ascii=False) + "\n" for x in rows)
    relay_rows = []
    for a, work, o, what, found in outcomes:
        h = wikt.hyps[a["target"]]
        relay_rows.append(
            {
                "assertion": a["id"],
                "locator": a["locator"],
                "work": work,
                "links": [
                    [link["child"], link.get("mode", link["relation"]), link["parents"]]
                    for link in h["links"]
                ],
                "acd_assertion": o and o["id"],
                "acd_link": o and acd.hyps[o["target"]]["links"][0],
                "acd_parents": o
                and [
                    acd.records[p].get("form") for p in acd.hyps[o["target"]]["links"][0]["parents"]
                ],
                "outcome": ("agrees" if not what else "differs") if o else "no relay",
                "detail": what,
                "parents": found,
            }
        )
    (out / "relays.json").write_text(json.dumps(relay_rows, ensure_ascii=False, indent=1) + "\n")
    counts = report(rows, relay_rows)
    counts["objects"] = n
    sample = draw_sample(matches, acd, wikt, args.seed)
    (out / "sample.json").write_text(json.dumps(sample, ensure_ascii=False, indent=1) + "\n")
    (out / "counts.json").write_text(json.dumps(counts, ensure_ascii=False, indent=1) + "\n")


def report(rows, relay_rows):
    c = collections.defaultdict(collections.Counter)
    for x in rows:
        if x["outcome"] == "skipped":
            c["records: language not exact"][x["kind"]] += 1
            continue
        c[f"records: {x['kind']}"][x["outcome"]] += 1
        c[f"records: {x['kind']}, by level"][f"{x['level']}: {x['outcome']}"] += 1
        if x["outcome"] == "unmatched":
            c[f"unmatched {x['kind']}: why"][x["detail"]] += 1
        if x["outcome"] == "match":
            c[f"matched {x['kind']}: key"][x["basis"]["key"]] += 1
            if "reflexes" in x["basis"]:
                rx = x["basis"]["reflexes"]
                c["matched reconstruction: shared reflexes"][
                    "0" if not rx["shared"] else "1-2" if rx["shared"] < 3 else "3+"
                ] += 1
    for r in relay_rows:
        c["relays"][r["outcome"]] += 1
        c["relays by work"][f"{r['work']}: {r['outcome']}"] += 1
        if r["parents"]:
            c["relays: the ACD's parents found"][f"{r['parents']}: {r['outcome']}"] += 1
        if r["outcome"] == "differs":
            c["relays: differences"][", ".join(r["detail"])] += 1
        elif r["outcome"] == "no relay":
            c["no relay: why"][r["detail"]] += 1
    return {k: dict(sorted(v.items())) for k, v in sorted(c.items())}


def draw_sample(matches, acd, wikt, seed, n=100):
    """A random sample of matches with a target, for review."""
    rng = random.Random(seed)
    chosen = rng.sample(sorted((m for m in matches if "target" in m), key=lambda m: m["id"]), n)
    out = []
    for m in chosen:
        r = acd.records[m["record"]]
        w = wikt.records[m["target"]]
        out.append(
            {
                "match": m["id"],
                "acd": {
                    "record": r["id"],
                    "language": r["language"]["label"],
                    "form": r["form"],
                    "gloss": r.get("gloss"),
                    "sets": {
                        s: acd.records[acd.sets[s]["reconstruction"]]["form"]
                        for s in acd.sets_of.get(r["id"], [])
                        if acd.sets[s]["reconstruction"]
                    },
                },
                "wiktionary": {
                    "record": w["id"],
                    "form": w["form"],
                    "gloss": wikt.gloss(w["id"]),
                    "etymology": wikt.etymologies.get(w["id"].rstrip("?")),
                    "parents": sorted(
                        {p for h in wikt.origins(w["id"]) for p in h["links"][0]["parents"]}
                    ),
                },
                "score": m["score"],
                "basis": m["basis"],
                "verdict": None,
                "cause": None,
            }
        )
    return out


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="step", required=True)
    s = sub.add_parser("scan", help="the Austronesian part of the export, and its glosses")
    s.add_argument("--export", type=Path, required=True, help="a graph's assertions.jsonl.gz")
    s.add_argument("--wiktextract", type=Path, required=True, help="raw-wiktextract-data.jsonl.gz")
    s.add_argument("--work", type=Path, required=True, help="the work directory")
    s.add_argument("--only", choices=["export", "glosses"])
    m = sub.add_parser("run", help="match, relay, and write the check's files")
    m.add_argument("--acd", type=Path, required=True, help="the ACD's assertions (acd-2)")
    m.add_argument("--export", type=Path, required=True, help="the graph's assertions.jsonl.gz")
    m.add_argument("--wiktextract", type=Path, required=True, help="raw-wiktextract-data.jsonl.gz")
    m.add_argument("--work", type=Path, required=True, help="the work directory `scan` wrote")
    m.add_argument("--out", type=Path, required=True, help="the check directory")
    m.add_argument("--seed", type=int, default=20261101)
    args = p.parse_args(argv)
    if args.step == "run":
        run(args)
    if args.step == "scan":
        args.work.mkdir(parents=True, exist_ok=True)
        codes = austronesian_codes()
        if args.only != "glosses":
            n = scan_export(args.export, codes, args.work / "wiktionary.jsonl.gz")
            print(f"wiktionary.jsonl.gz: {n} objects")
        if args.only != "export":
            n = scan_glosses(args.wiktextract, codes, args.work / "glosses.jsonl.gz")
            print(f"glosses.jsonl.gz: {n} entries")


if __name__ == "__main__":
    main()
