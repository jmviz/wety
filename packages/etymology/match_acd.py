"""ACD records matched to Wiktionary's, and Wiktionary's assertions that credit the ACD
relayed to the ACD's own (acd-3).

Steps, each writing into a work directory so that later steps rerun quickly:

- `scan`: the Austronesian part of a graph's assertion export (records of an Austronesian
  language, the hypotheses that name one, the assertions on those hypotheses or crediting
  a work, every work and agent), and the glosses of Austronesian entries from the
  entries the graph was built from (`wiktionary_entries`);
- `run`: each ACD record of a language the ACD's language map calls `exact` matched to
  a Wiktionary record of that code, by the rule `agent:wety-match-acd@4` (`Matcher`:
  comparison keys and shape, gloss, the reflexes the ACD cites and the record's
  ancestors); then each Wiktionary assertion that credits the ACD linked to the ACD's
  own assertion on the same word, read through the matches, by `agent:wety-relay-acd@1`
  (`Relayer`). It writes the check's files and a sample of matches to review, or, given
  an earlier run's check directory (`--previous`), what changed against it: a sample of
  the changed matches. Every run is scored against the judgements of all earlier
  reviews (`etc/judgements.jsonl`, `etymology.matching`);
- `review`: adds a check's judged `review.json` to those judgements.

v4 decides as v3 does, and adds what the shared module gives: reviewers' judgements
decide the pairs they cover (a pair judged another word is no candidate, one judged the
same word is the match); the score is the probability learnt weights give the match
(`matching.Linkage`, fitted to every eligible pair, judged pairs held at their label);
each check is scored on judgements held out of the fit; and `hard.json` lists ties and
the least probable targets for a reviewer or a model. Two trials that let the weights
decide (`checks/acd-3-v4-trial/`) did worse than v3's ranking: fitted on these pairs,
EM splits them by gloss rather than by word, so a probability threshold turned away
4,395 attested matches, and without one, ranking by the weights took homographs' other
senses (Malay batu 'stone' for 'mile'): 37 of 44 changes right, against v3's 32 of 32.

v1 of the matcher (acd-3) weighed no ancestors, compared affixes by key alone and let a
near-key doublet tie an equal-key record; its reviewed outputs stay in `checks/acd-3/`.
v2 (`checks/acd-3-ancestry/`) let a cited spelling's shared ancestor outrank its entry's
gloss, took a Wiktionary ancestor of another form as another word whatever its gloss,
and kept a reconstruction from Wiktionary's bound stem (*saŋa-) when a citation of it as
a word (*saŋa) had the same shape.
"""

import argparse
import collections
import gzip
import json
import random
import re
import unicodedata
from functools import cached_property
from pathlib import Path

from etymology import assertions as A
from etymology import orthography
from etymology.assertions import REPO
from etymology.matching import (
    Judgements,
    Linkage,
    closure,
    harvest_into,
    lines,
    pooled,
)

SOURCES = REPO / "data/workspaces/etymology/sources"
ACD = SOURCES / "acd"
PROFILES = {
    ("acd", True): ACD / "etc/orthography.tsv",
    ("acd", False): ACD / "etc/orthography-attested.tsv",
    ("wiktionary", True): SOURCES / "wiktionary/etc/orthography-map.tsv",
    ("wiktionary", False): SOURCES / "wiktionary/etc/orthography-map-attested.tsv",
}
JUDGEMENTS = ACD / "etc/judgements.jsonl"
RULE = "agent:wety-match-acd@4"
# What a pair's evidence may be, strongest first (`matching.Linkage`). Reflexes and
# ancestors both count matched lineage, so they are one field.
FIELDS = {
    "key": ("equal", "near"),
    "lineage": (
        "reflexes shared",
        "a reflex shared",
        "an ancestor shared",
        "cited, none shared",
        "none cited",
    ),
    "gloss": ("close", "fits", "some", "none", "unknown"),
    "shape": ("same", "other"),
    "entry": ("entry", "imputed"),
}
MISSING = {"lineage": "none cited", "gloss": "unknown"}
FIT_ROUNDS = 3  # rounds that refit the weights
LABEL_WEIGHT = 100  # how many unjudged pairs a judged pair counts as, in the fit
HARD = 0.8  # a target below this probability is a hard case
FOLDS = 5
HARD_LIMIT = 200
RELAY_RULE = "agent:wety-relay-acd@1"
RECORD_CODE = re.compile(r"^wiktionary:([^:]+):")
LANG_CODE = re.compile(rb'"lang":"([^"]+)"')


def austronesian_codes(languages, families_path, language_map=ACD / "etc/languages.json"):
    """Wiktionary's codes of the Austronesian family (languages whose family is `map` or
    below it, and its proto-languages), with every code the ACD's language map names, from
    Wiktionary's language and family data (`languages.json`, `families.json`)."""
    langs = json.loads(Path(languages).read_text())
    fams = json.loads(Path(families_path).read_text())
    families = {f for f, v in fams.items() if f == "map" or "map" in v["superfamilies"]}
    codes = {c for c, v in langs.items() if v.get("family") in families}
    codes |= {fams[f]["protoLanguage"] for f in families if fams[f].get("protoLanguage")}
    for row in json.loads(Path(language_map).read_text())["rows"]:
        codes |= {c for c in [row["code"], *row.get("candidates", [])] if c}
    return codes


def record_code(record_id):
    m = RECORD_CODE.match(record_id)
    return m.group(1) if m else None


def scan_export(export, codes, out):
    """The Austronesian part of a graph's assertion export, to `out` (JSON Lines, gzipped).

    Kept: source, agents, works, extractions and alternative sets whole; records of an
    Austronesian code; hypotheses naming such a record; assertions whose target is a kept
    hypothesis, or that credit a work (filtered once every hypothesis is known). A second
    pass adds the other records the kept hypotheses name."""
    keep, assertions, kept_hyps, named = [], [], set(), set()
    for line in lines(export, binary=True):
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
    for line in lines(export, binary=True):
        if line.startswith(b'{"form"') or b'"type":"record"' in line:
            o = json.loads(line)
            if o["type"] == "record" and o["id"] in missing:
                keep.append(o)
    with gzip.open(out, "wt", encoding="utf-8") as f:
        f.writelines(json.dumps(o, ensure_ascii=False, sort_keys=True) + "\n" for o in keep)
    return len(keep)


def scan_glosses(entries, codes, out):
    """The glosses of every Austronesian entry, one row per part of speech, keyed by code,
    page title and etymology label, to `out` (JSON Lines, gzipped).

    The graph's own record ids key a word by its canonical form, which for Austronesian
    entries is almost always the page title; the first rendered headword is kept beside
    it. A reconstruction's page is named by its part after the language."""
    wanted = {c.encode() for c in codes}
    n = 0
    with gzip.open(out, "wt", encoding="utf-8") as f:
        for line in lines(entries, binary=True):
            if not wanted.intersection(LANG_CODE.findall(line[:2000])):
                continue
            e = json.loads(line)
            if e.get("lang") not in codes:
                continue
            page = e["page"]
            if e.get("ns") == 118 and "/" in page:
                page = page.split("/", 1)[1]
            blocks = (e.get("etymology") or {}).get("blocks", [])
            etymology = "\n".join(b["text"] for b in blocks if b.get("text")) or None
            for pos in e.get("pos", []):
                headword = next(iter(pos.get("headword", [])), None)
                row = {
                    "code": e["lang"],
                    "word": page,
                    "canonical": headword.lstrip("*") if headword else None,
                    "ety": e.get("ety_label") or "",
                    "pos": pos.get("pos"),
                    "glosses": [
                        drop_labels(s["gloss"]) for s in pos.get("senses", []) if s.get("gloss")
                    ],
                    "etymology": etymology,
                }
                f.write(json.dumps(row, ensure_ascii=False) + "\n")
                n += 1
    return n


LABELS = re.compile(r"^(\([^)]*\)\s*)+")


def drop_labels(gloss):
    """A gloss without the labels it opens with: "(zoology) a wolf" is "a wolf"."""
    return LABELS.sub("", gloss.strip())


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


def shape(form):
    """Whether a form is a prefix (or bound stem), suffix, infix or circumfix, or a
    word: its edge hyphens, which comparison keys drop. The ACD writes a lone infix
    `<um>`."""
    f = form.strip().lstrip("*").strip()
    if f.startswith("<") and f.endswith(">"):
        return (True, True)
    return (f.startswith("-"), f.endswith("-"))


WORD = (False, False)


def twin(record_id):
    """A Wiktionary record and the imputed record for the same entry (`?`) are one word
    for ancestry."""
    return record_id.rstrip("?")


def compatible(a, b):
    """Whether two keys may name one ancestor: equal, near, or one inside the other (an
    affixed or compound form and its base: *ma-sakit and *sakit, *C<um>aŋis and *Caŋis)."""
    if a == b or near(a, b):
        return True
    short, long = sorted((a, b), key=len)
    if short in long:
        return len(short) >= 2
    it = iter(long)
    return len(short) >= 3 and all(c in it for c in short)


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

    def ancestors(self, record_id):
        """The records the ACD makes a record descend from, at any remove (its set's
        reconstruction, a sub-entry's main entry, and theirs), that have a form."""
        seen, todo = set(), [record_id]
        while todo:
            for p in self.parents.get(todo.pop(), ()):
                if p not in seen:
                    seen.add(p)
                    todo.append(p)
        return {p for p in seen if self.records.get(p, {}).get("form")}

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
        self.parents = collections.defaultdict(set)
        for a in self.assertions:
            h = self.hyps.get(a["target"])
            if h and a["stance"] == "endorses":
                self.by_word[h["word"]].append(a)
                for link in h["links"]:
                    self.parents[link["child"]].update(link["parents"])
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

    def ancestors(self, record_id):
        """A record's ancestors in wety, at any remove, through every endorsed link."""
        seen, todo = set(), [record_id]
        while todo:
            for p in self.parents.get(todo.pop(), ()):
                if p not in seen:
                    seen.add(p)
                    todo.append(p)
        return seen

    def descendants(self, record_id, limit=5000):
        seen, todo = set(), [record_id]
        while todo and len(seen) < limit:
            for c in self.children.get(todo.pop(), ()):
                if c not in seen:
                    seen.add(c)
                    todo.append(c)
        return seen


class Matcher:
    """The rule `wety-match-acd@4`: each ACD record of an `exact` language to a Wiktionary
    record of the same code.

    Candidates are the Wiktionary records of that code and shape (word, prefix, suffix,
    infix: `shape`; when none has the same shape, a word may stand for an affix and an
    affix for a word, as one source writes a bound stem *tuRa- or an enclitic -mo that
    the other writes as a word, but two kinds of affix never meet; so may they for a
    reconstruction of four or more segments whose own shape has only imputed records,
    since Wiktionary writes the stem *saŋa- and cites it as *saŋa) whose comparison key
    equals the ACD record's; failing that, for an
    attested form, whose loose key (glottal stops dropped) does; failing that, for a
    reconstruction, whose key is one edit away. Each candidate is weighed by:

    - reflexes, for a reconstruction: of the attested reflexes the ACD cites for it
      (`Acd.reflexes`, one per language and key), how many are among the candidate's
      descendants in wety;
    - ancestors: of the record's ancestors in the ACD (its set's reconstruction, a
      sub-entry's main entry, and theirs) that are matched, how many are the
      candidate's ancestors in wety (an imputed record for the same entry counts as
      it). A candidate whose ancestry in wety has words in an ACD ancestor's language,
      none of them that ancestor's match, of a `compatible` key or of a gloss that fits
      the ancestor's (similarity `GLOSS_MIN`: *bətik 'tattoo' for the ACD's *batek
      'mottled design, as of a tattoo', one etymon reconstructed two ways), contradicts the ACD
      (matched or not, so this reaches further than the basis): it is taken only when
      its gloss (similarity `GLOSS_MIN`) or, for a reconstruction, a shared reflex shows
      it is the same word, the two sources then differing on its origin;
    - the gloss.

    A reconstruction needs a shared reflex, a shared ancestor or a gloss similarity of
    at least `GLOSS_MIN`; one at a near key needs two shared reflexes. An attested form
    is taken unless its gloss shares nothing with the ACD's (both known) and no ancestor
    is shared (a shared ancestor vouches for a gloss that shares nothing with the
    record's only when it fits that ancestor's gloss in the ACD), and is passed over for
    any candidate whose gloss is not a mismatch (an imputed reconstruction such as PMP
    *ba#1? stands for every homograph cited under that title, so a shared ancestor alone
    cannot outweigh the gloss). Candidates rank by shared reflexes, key (equal before
    near), shared ancestors, then gloss, which must be closer by `GLOSS_MARGIN` to decide. A
    fitting gloss stands against a shared ancestor of a candidate without a gloss: an
    imputed record (a word wety knows only from citations) may be Wiktionary's citation
    of an entry under another spelling (bener for bĕnĕr) whose own etymology names no
    ancestor. With the same evidence, the record's shape is preferred, then a Wiktionary
    entry to an imputed record; otherwise equal fits are left as candidates.

    Reconstructions are matched first, in rounds, so that their ancestors' matches can
    be weighed (until no match changes, at most `ROUNDS`); then attested forms. Of
    several ACD records with different forms that fit one Wiktionary record (homographs
    such as *kutu₁ and *kutu₂, or doublets such as *wakaR and *akaR, which share as many
    reflexes, so that the equal key decides), only the best by the same ranking is
    matched; equal fits are left unmatched. Records with the very same
    form are one word, and are kept together.

    Reviewers' judgements decide the pairs they cover, and the score is the learnt
    weights' probability (see the module's docstring).
    """

    GLOSS_MIN = 0.25
    GLOSS_MARGIN = 0.2
    ROUNDS = 6

    def __init__(
        self, acd, wiktionary, keys, language_map=None, exact=None, judgements=None, learn=True
    ):
        self.acd, self.wikt, self.keys = acd, wiktionary, keys
        self.judgements = judgements or Judgements()
        self.learn = learn
        self.models = {k: Linkage(FIELDS, MISSING) for k in ("reconstruction", "attested")}
        self.exact = exact or {
            r["code"]
            for r in json.loads(Path(language_map).read_text())["rows"]
            if r["relation"] == "exact" and r["code"]
        }
        self.index = collections.defaultdict(lambda: collections.defaultdict(set))
        self.loose_index = collections.defaultdict(lambda: collections.defaultdict(set))
        self.shapes = {}
        for r in wiktionary.records.values():
            code = r["language"]["code"]
            if code not in self.exact or r["form"] is None:
                continue
            self.shapes[r["id"]] = shape(r["form"])
            ks = keys("wiktionary", r["reconstructed"], r["form"])
            for k in ks:
                self.index[code][k].add(r["id"])
            for k in loose(ks):
                self.loose_index[code][k].add(r["id"])
        self.descendant_keys = {}
        self.ancestor_cache = {}
        self.matched = {}

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

    def wiktionary_ancestors(self, record_id):
        if record_id not in self.ancestor_cache:
            self.ancestor_cache[record_id] = {twin(a) for a in self.wikt.ancestors(record_id)}
        return self.ancestor_cache[record_id]

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
        form_shape = shape(r["form"])

        def same_shape(ws):
            same = {w for w in ws if self.shapes[w] == form_shape}
            stem = r["reconstructed"] and max(map(len, ks), default=0) >= 4
            if not same or (stem and all(w.endswith("?") for w in same)):
                same |= {w for w in ws if WORD in (form_shape, self.shapes[w])}
            return same, bool(ws)

        found = set().union(*(self.index[code].get(k, ()) for k in ks)) if ks else set()
        if found:
            return *same_shape(found), "equal"
        if not r["reconstructed"]:
            found = set().union(*(self.loose_index[code].get(k, ()) for k in loose(ks)))
            return *same_shape(found), "near"
        found = {
            w
            for k in ks
            for wk, ws in self.index[code].items()
            if abs(len(wk) - len(k)) <= 1 and near(k, wk)
            for w in ws
        }
        return *same_shape(found), "near"

    def ancestry(self, r, w):
        """(shared, cited, contradicted, fit) for a candidate: of the record's matched ACD
        ancestors, how many are the candidate's ancestors in wety; how well the
        candidate's gloss fits the best of those shared ancestors' glosses; and whether the
        candidate's ancestry contradicts the ACD's: it has words in the language of some
        of the record's ACD ancestors, and none of those ancestors finds its match or a
        `compatible` key among them (a record in two of the ACD's sets agrees with
        Wiktionary if one of them does). Affixes are not compared, since a derivative
        names its affix as an ancestor beside its base: none of Wiktionary's, and none of
        the ACD's with a leading hyphen (its trailing-hyphen ancestors are bound stems,
        *tuRa- 'sibling'). A Wiktionary ancestor whose gloss fits the ACD ancestor's
        agrees with it, whatever its form."""
        ancestors = self.acd.ancestors(r["id"])
        targets = {twin(self.matched[a]) for a in ancestors if a in self.matched}
        mine = self.wiktionary_ancestors(w)
        by_code = collections.defaultdict(list)
        for x in mine:
            r = self.wikt.records.get(x) or self.wikt.records.get(x + "?")
            if r and r["form"] and shape(r["form"]) == WORD:
                by_code[record_code(x)].append(r)
        compared = agreed = 0
        for a in ancestors:
            lang = self.acd.records[a]["language"]
            same_lang = by_code.get(lang["code"]) if lang["relation"] == "exact" else None
            if not same_lang or shape(self.acd.records[a]["form"])[0]:
                continue
            compared += 1
            if a in self.matched and twin(self.matched[a]) in mine:
                agreed += 1
                continue
            ka = self.keys("acd", True, self.acd.records[a]["form"])
            agreed += any(
                compatible(k, j)
                for x in same_lang
                for k in ka
                for j in self.keys("wiktionary", True, x["form"])
            ) or any(
                (gloss_similarity(self.acd.records[a].get("gloss"), self.wikt.gloss(x["id"])) or 0)
                >= self.GLOSS_MIN
                for x in same_lang
            )
        gloss = self.wikt.gloss(w)
        fits = [
            gloss_similarity(self.acd.records[a].get("gloss"), gloss)
            for a in ancestors
            if a in self.matched and twin(self.matched[a]) in mine
        ]
        fit = max((f for f in fits if f is not None), default=None)
        return len(targets & mine), len(targets), bool(compared) and not agreed, fit

    def evaluate(self, r):
        """Each candidate's basis and whether its ancestry contradicts the ACD; None for
        a record no rule considers, "shape" when every candidate has another shape."""
        lang = r["language"]
        if lang["relation"] != "exact" or lang["code"] not in self.exact or not r["form"]:
            return None
        found, any_shape, key = self.candidates(r)
        if any_shape and not found:
            return "shape"
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
            shared, n, contradicted, fit = self.ancestry(r, w)
            if n:
                basis["ancestors"] = {"shared": shared, "cited": n}
            out.append((w, basis, {"contradicted": contradicted, "fit": fit}))
        return out

    def eligible(self, basis, reconstructed, ancestry):
        g = basis.get("gloss")
        # a shared ancestor vouches for a candidate whose gloss shares nothing with the
        # record's only when that gloss fits the ancestor's own in the ACD
        mismatch = g == 0 and (ancestry["fit"] or 0) < self.GLOSS_MIN
        anc = basis.get("ancestors", {}).get("shared", 0) >= 1 and not mismatch
        if reconstructed:
            shared = basis["reflexes"]["shared"]
            if basis["key"] == "near":
                ok = shared >= 2
            else:
                ok = shared >= 1 or anc or (g is not None and g >= self.GLOSS_MIN)
        else:
            ok = g is None or g > 0 or anc
        if ok and ancestry["contradicted"]:
            ok = (g is not None and g >= self.GLOSS_MIN) or (
                reconstructed and basis["reflexes"]["shared"] >= 1
            )
        return ok

    def features(self, r, w, basis):
        """A candidate's evidence as `Linkage` levels."""
        rx = basis.get("reflexes", {}).get("shared", 0)
        an = basis.get("ancestors", {}).get("shared", 0)
        cited = sum(basis.get(k, {}).get("cited", 0) for k in ("reflexes", "ancestors"))
        lineage = (
            "reflexes shared"
            if rx >= 2
            else "a reflex shared"
            if rx
            else "an ancestor shared"
            if an
            else "cited, none shared"
            if cited
            else "none cited"
        )
        g = basis.get("gloss")
        gloss = (
            "unknown"
            if g is None
            else "close"
            if g >= 0.5
            else "fits"
            if g >= self.GLOSS_MIN
            else "some"
            if g > 0
            else "none"
        )
        return {
            "key": basis["key"],
            "lineage": lineage,
            "gloss": gloss,
            "shape": "same" if self.shapes[w] == shape(r["form"]) else "other",
            "entry": "imputed" if w.endswith("?") else "entry",
        }

    def options(self, r):
        """A record's eligible candidates as (candidate, basis, features), or the
        outcome when it has none."""
        found = self.evaluate(r)
        if found is None:
            return ("skipped", "language not exact", None)
        if found == "shape":
            return ("unmatched", "Wiktionary's words with this key have another shape", None)
        if not found:
            return ("unmatched", "no Wiktionary record with this key", None)
        # a pair a reviewer judged the same word needs no eligibility
        ok = [
            (w, b)
            for w, b, c in found
            if self.eligible(b, r["reconstructed"], c) or self.judgements.label(r["id"], w)
        ]
        if not r["reconstructed"] and any(b.get("gloss", 1) > 0 for _, b in ok):
            ok = [(w, b) for w, b in ok if b.get("gloss", 1) > 0]
        if not ok:
            if all(c["contradicted"] for _, _, c in found):
                return ("unmatched", "candidates whose ancestry contradicts the ACD", found[0][1])
            return ("unmatched", "candidates without support", found[0][1])
        return [(w, b, self.features(r, w, b)) for w, b in ok]

    def fit(self, model, options):
        """Fits a model to the eligible pairs, judged pairs held at their label."""
        if not self.learn:
            return
        flat = [(rid, w, f) for rid, o in options.items() if isinstance(o, list) for w, _, f in o]
        model.fit(
            [f for _, _, f in flat],
            [self.judgements.label(rid, w) for rid, w, _ in flat],
            label_weight=LABEL_WEIGHT,
        )

    def rank(self, r, w, basis):
        g = basis.get("gloss")
        return (
            basis.get("reflexes", {}).get("shared", 0),
            basis["key"] == "equal",
            basis.get("ancestors", {}).get("shared", 0),
            g or 0,
            self.shapes[w] == shape(r["form"]),
            not w.endswith("?"),
            g is not None,
        )

    def beats(self, a, b):
        """Whether rank `a` is a better fit than rank `b`. A fitting gloss stands against
        a shared ancestor of a candidate without a gloss: Wiktionary cites a spelling
        (bener) of an entry (bĕnĕr) whose own etymology does not name the ancestor."""
        if a[:2] != b[:2]:
            return a[:2] > b[:2]
        lo, hi = sorted((a, b), key=lambda x: x[2])
        offset = lo[2] < hi[2] and lo[3] >= self.GLOSS_MIN and not hi[6]
        if a[2] != b[2] and not offset:
            return a[2] > b[2]
        return a[3] - b[3] >= self.GLOSS_MARGIN

    def decide(self, r, opts, model):
        """("match", target, basis) | ("ambiguous", candidates, basis) | ("unmatched", why,
        basis). Reviewers' judgements decide the pairs they cover; the rest is ranked as
        v3 ranked it. The score is the weights' probability."""
        if isinstance(opts, tuple):
            return opts
        rid = r["id"]
        opts = [o for o in opts if self.judgements.label(rid, o[0]) is not False]
        if not opts:
            return ("unmatched", "candidates judged other words", None)
        bases = {w: b for w, b, _ in opts}
        ranks = {w: self.rank(r, w, b) for w, b, _ in opts}

        def better(a, b):
            return self.beats(a, b) or (a[:4] == b[:4] and a[4:6] > b[4:6])

        kept = next(
            ([w] for w in bases if all(o == w or better(ranks[w], ranks[o]) for o in ranks)),
            None,
        ) or sorted(w for w in ranks if not any(self.beats(ranks[o], ranks[w]) for o in ranks))
        judged = self.judgements.choose(rid, list(bases))
        if judged and judged != kept:
            self.overridden[rid], kept = kept, judged
        probability = {w: model.probability(f) for w, _, f in opts}
        self.scores[rid] = round(max(probability[w] for w in kept), 3)
        if len(kept) == 1:
            return ("match", kept[0], bases[kept[0]])
        return ("ambiguous", kept, bases[kept[0]])

    def run(self):
        reconstructions = [r for r in self.acd.records.values() if r["reconstructed"]]
        decisions = {}
        self.rounds, self.changes = 0, []
        self.scores, self.overridden = {}, {}
        model = self.models["reconstruction"]
        for n in range(self.ROUNDS):
            self.rounds += 1
            options = {r["id"]: self.options(r) for r in reconstructions}
            if n < FIT_ROUNDS:
                self.fit(model, options)
            decisions = {r["id"]: self.decide(r, options[r["id"]], model) for r in reconstructions}
            self.resolve_homographs(decisions)
            matched = {rid: d[1] for rid, d in decisions.items() if d[0] == "match"}
            self.changes.append(len(set(matched.items()) ^ set(self.matched.items())))
            if matched == self.matched and n >= FIT_ROUNDS:
                break
            self.matched = matched
        attested = [r for r in self.acd.records.values() if not r["reconstructed"]]
        options = {r["id"]: self.options(r) for r in attested}
        self.fit(self.models["attested"], options)
        for r in attested:
            decisions[r["id"]] = self.decide(r, options[r["id"]], self.models["attested"])
        return decisions

    def resolve_homographs(self, decisions):
        """Of the ACD's records with different forms matched to one Wiktionary record
        (homographs, doublets), keep the best fit."""
        by_target = collections.defaultdict(lambda: collections.defaultdict(list))
        for rid, (what, target, basis) in decisions.items():
            if what == "match" and self.acd.records[rid]["reconstructed"]:
                by_target[target][self.acd.records[rid]["form"]].append(rid)
        for target, groups in by_target.items():
            if len(groups) < 2:
                continue
            judged = [
                f
                for f, rids in groups.items()
                if any(self.judgements.label(x, target) for x in rids)
            ]
            if judged:
                # a reviewer's judgement decides between the ACD's records too
                for form, rids in groups.items():
                    if form not in judged:
                        for x in rids:
                            decisions[x] = (
                                "unmatched",
                                "another ACD record is judged its match",
                                decisions[x][2],
                            )
                continue
            rank = {
                form: max(self.rank(self.acd.records[x], target, decisions[x][2]) for x in rids)
                for form, rids in groups.items()
            }
            winners = [f for f in groups if not any(self.beats(rank[o], rank[f]) for o in groups)]
            for form, rids in groups.items():
                if form in winners and len(winners) == 1:
                    continue
                why = (
                    "other ACD records fit equally"
                    if form in winners
                    else "another ACD record fits better"
                )
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
    judgements = Judgements.read(JUDGEMENTS)
    matcher = Matcher(acd, wikt, keys, language_map, judgements=judgements)
    decisions = matcher.run()
    extraction = A.extraction(
        "converter",
        "packages/etymology/match_acd.py",
        [args.acd, args.export, args.entries, language_map, *PROFILES.values(), JUDGEMENTS],
    )
    new = {extraction["id"]: extraction}
    for agent_id, name in (
        (
            RULE,
            "wety's ACD matcher, v3: ACD records to Wiktionary records by key, gloss, reflexes and ancestors",
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
                matcher.scores[rid],
                basis,
                target=target,
                extraction=extraction["id"],
            )
        elif what == "ambiguous":
            m = A.match(
                rid,
                RULE,
                matcher.scores[rid],
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
    counts["reconstruction rounds"] = matcher.changes
    if args.previous:
        changes, sample = compare(args.previous, rows, matches, acd, wikt, args.seed)
        counts["changes"] = changes
        (out / "changed-sample.json").write_text(
            json.dumps(sample, ensure_ascii=False, indent=1) + "\n"
        )
    else:
        sample = draw_sample(matches, acd, wikt, args.seed)
        (out / "sample.json").write_text(json.dumps(sample, ensure_ascii=False, indent=1) + "\n")
    found = {x["record"]: x["detail"] for x in rows if x["outcome"] in ("match", "ambiguous")}
    counts["judgements"] = judgements.score(found)
    counts["weights"] = {k: m.table() for k, m in matcher.models.items()}
    counts["decided by judgements"] = {
        rid: {"weights": before, "judged": decisions[rid][1]}
        for rid, before in sorted(matcher.overridden.items())
    }
    counts["held out"] = held_out(acd, wikt, keys, language_map, judgements)
    hard = hard_cases(decisions, matcher, acd, wikt)
    (out / "hard.json").write_text(json.dumps({"hard": hard}, ensure_ascii=False, indent=1) + "\n")
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


def review_entry(m, acd, wikt):
    """A match as a reviewer reads it: both records, the ACD's sets, and Wiktionary's
    gloss, etymology and parents."""
    r = acd.records[m["record"]]
    w = wikt.records[m["target"]]
    return {
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
            "parents": sorted({p for h in wikt.origins(w["id"]) for p in h["links"][0]["parents"]}),
        },
        "score": m["score"],
        "basis": m["basis"],
        "verdict": None,
        "cause": None,
    }


def held_out(acd, wikt, keys, language_map, judgements):
    """The matcher scored on judgements it did not see: in each of `FOLDS` folds it
    learns from the others' judgements and is scored on the fold's."""
    scores = []
    for training, test in judgements.split(FOLDS, 20261101):
        d = Matcher(acd, wikt, keys, language_map, judgements=training).run()
        found = {rid: x[1] for rid, x in d.items() if x[0] in ("match", "ambiguous")}
        scores.append(test.score(found))
    return pooled(scores)


def hard_cases(decisions, matcher, acd, wikt):
    """What the weights barely decide and no judgement decides, for a reviewer or a
    model: each candidate of a tie, then the `HARD_LIMIT` least probable targets below
    `HARD`; judged as a sample is (`verdict` on the Wiktionary record)."""
    ties, weak = [], []
    for rid, (what, found, basis) in sorted(decisions.items()):
        if what not in ("match", "ambiguous") or matcher.judgements.verdict(rid, found):
            continue
        score = matcher.scores[rid]
        if what == "ambiguous":
            for w in found:
                m = {"id": None, "record": rid, "target": w, "score": score, "basis": basis}
                ties.append(review_entry(m, acd, wikt))
        elif score < HARD:
            weak.append((score, rid, found, basis))
    weak = [
        review_entry({"id": None, "record": rid, "target": w, "score": p, "basis": b}, acd, wikt)
        for p, rid, w, b in sorted(weak)[:HARD_LIMIT]
    ]
    return ties + weak


def draw_sample(matches, acd, wikt, seed, n=100):
    """A random sample of matches with a target, for review."""
    rng = random.Random(seed)
    targeted = sorted((m for m in matches if "target" in m), key=lambda m: m["id"])
    return [review_entry(m, acd, wikt) for m in rng.sample(targeted, min(n, len(targeted)))]


def compare(previous, rows, matches, acd, wikt, seed, n=100):
    """What changed against an earlier run's check directory's decisions. Returns the
    counts and a sample of changed matches (gained or retargeted) to review. Earlier
    reviews are checked through the judgements (`JUDGEMENTS`), not here."""
    with gzip.open(previous / "decisions.jsonl.gz", "rt", encoding="utf-8") as f:
        before = {x["record"]: x for x in map(json.loads, f)}
    now = {x["record"]: x for x in rows}
    counts = collections.defaultdict(collections.Counter)
    changed = []
    for rid, x in now.items():
        b = before[rid]
        was, kind = b["outcome"], x["kind"]
        if x["outcome"] == "match" and was != "match":
            what = "gained (was ambiguous)" if was == "ambiguous" else "gained (was unmatched)"
            changed.append(rid)
        elif was == "match" and x["outcome"] != "match":
            what = (
                f"lost ({x['outcome']}: {x['detail']})"
                if x["outcome"] == "unmatched"
                else "lost (now ambiguous)"
            )
        elif was == "match" and b["detail"] != x["detail"]:
            what = "retargeted"
            changed.append(rid)
        elif (
            was == "ambiguous"
            and x["outcome"] == "ambiguous"
            and set(b["detail"]) != set(x["detail"])
        ):
            what = "ambiguous, other candidates"
        elif was == x["outcome"]:
            what = "same"
        else:
            what = f"{was} -> {x['outcome']}"
        counts[kind][what] += 1
    by_record = {m["record"]: m for m in matches if "target" in m}
    rng = random.Random(seed)
    picked = rng.sample(sorted(changed), min(n, len(changed)))
    sample = []
    for rid in picked:
        e = review_entry(by_record[rid], acd, wikt)
        e["before"] = {"outcome": before[rid]["outcome"], "detail": before[rid]["detail"]}
        sample.append(e)
    counts = {k: dict(sorted(v.items())) for k, v in sorted(counts.items())}
    return counts, sample


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="step", required=True)
    s = sub.add_parser("scan", help="the Austronesian part of the export, and its glosses")
    s.add_argument("--export", type=Path, required=True, help="a graph's assertions.jsonl.gz")
    s.add_argument(
        "--entries", type=Path, required=True, help="wety's entries (wiktionary_entries)"
    )
    s.add_argument("--work", type=Path, required=True, help="the work directory")
    s.add_argument("--languages", type=Path, required=True, help="Wiktionary's languages.json")
    s.add_argument("--families", type=Path, required=True, help="Wiktionary's families.json")
    s.add_argument("--only", choices=["export", "glosses"])
    m = sub.add_parser("run", help="match, relay, and write the check's files")
    m.add_argument("--acd", type=Path, required=True, help="the ACD's assertions (acd-2)")
    m.add_argument("--export", type=Path, required=True, help="the graph's assertions.jsonl.gz")
    m.add_argument(
        "--entries", type=Path, required=True, help="wety's entries (wiktionary_entries)"
    )
    m.add_argument("--work", type=Path, required=True, help="the work directory `scan` wrote")
    m.add_argument("--out", type=Path, required=True, help="the check directory")
    m.add_argument("--seed", type=int, default=20261101)
    m.add_argument(
        "--previous",
        type=Path,
        help="an earlier run's check directory: review what changed instead of a fresh sample",
    )
    r = sub.add_parser(
        "review", help="add a check's judged review.json and hard.json to the judgements"
    )
    r.add_argument("--out", type=Path, required=True, help="the check directory")
    args = p.parse_args(argv)
    if args.step == "run":
        run(args)
    if args.step == "review":
        reviews = [args.out / f for f in ("review.json", "hard.json")]
        n = harvest_into(JUDGEMENTS, [r for r in reviews if r.exists()])
        print(f"{JUDGEMENTS}: {n} new judgements")
    if args.step == "scan":
        args.work.mkdir(parents=True, exist_ok=True)
        codes = austronesian_codes(args.languages, args.families)
        if args.only != "glosses":
            n = scan_export(args.export, codes, args.work / "wiktionary.jsonl.gz")
            print(f"wiktionary.jsonl.gz: {n} objects")
        if args.only != "export":
            n = scan_glosses(args.entries, codes, args.work / "glosses.jsonl.gz")
            print(f"glosses.jsonl.gz: {n} entries")


if __name__ == "__main__":
    main()
