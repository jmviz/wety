"""The records of a word-centric source (WOLD, IE-CoR, TuLeD) matched to Wiktionary's,
and Wiktionary's assertions that credit it relayed to its own (cl-4, cl-5, cl-6).

A word-centric source gives words of attested languages and, for each, the words it
comes from (WOLD's source words, IE-CoR's roots and loan sources), or only cognate
sets (TuLeD). Steps, each writing into a work directory so that later steps rerun
quickly:

- `scan`: from a graph's assertion export, the Wiktionary records of a code the
  source's maps call `exact` whose key is some record's key, every hypothesis on their
  origin with the records it names and the assertions endorsing it, every work, and the
  assertions that credit the source; then the glosses of those records' entries, from
  the entries the graph was built from (`wiktionary_entries`);
- `run`: the source's records matched to Wiktionary's by its matcher rule (`Matcher`),
  and the assertions crediting it relayed to its own by its relay rule (`Relayer`); the
  check's files as `match_etymdict` writes them, with a sample of matches to review.

An attested form's key is the form without diacritics, in lower case, underscores and
hyphens read as nothing, each of its alternants (`~`, `,`) apart: WOLD writes Latin
without macrons where Wiktionary's records keep them. A reconstruction's key, where the
source has an orthography profile (IE-CoR's roots, in the Indo-European notation), is
read through it and Wiktionary's (`Keyer`). Scripts WOLD romanises (Thai, Russian,
Arabic) find no candidates; transliteration is not attempted.
"""

import argparse
import collections
import gzip
import json
import random
import re
import unicodedata
from dataclasses import dataclass
from pathlib import Path

from etymology import assertions as A
from etymology import orthography
from etymology.assertions import REPO
from etymology.match_etymdict import BORROWINGS, drop_labels, gloss_similarity, read_lines
from etymology.matching import Judgements, closure, lines

SOURCES = REPO / "data/workspaces/etymology/sources"
WIKTIONARY_INE = SOURCES / "wiktionary/etc/orthography-ine-itc.tsv"
ALTERNANTS = re.compile(r"\s*(?:~|,|;|/)\s*")
RECORD = re.compile(r"^wiktionary:([^:]+):(.*)#")
CODE = re.compile(rb'"code":"[^"]+"')
LANG_CODE = re.compile(rb'"lang":"([^"]+)"')
KEPT_WHOLE = ("work", "agent", "source", "extraction", "alternative_set")
GLOSS_MIN = 0.25
LINEAGE = ("source named", "source language", "none")


def keys(form):
    """A form's keys: each alternant without diacritics, in lower case."""
    out = set()
    for part in ALTERNANTS.split(form or ""):
        k = unicodedata.normalize("NFD", part.strip().lstrip("*").lower())
        k = "".join(c for c in k if not unicodedata.combining(c))
        k = re.sub(r"[_\-\s]+", "", re.sub(r"\(\d+\)$", "", k))
        if k:
            out.add(unicodedata.normalize("NFC", k))
    return out


@dataclass(frozen=True)
class Spec:
    """What the matcher needs to know of one source."""

    name: str
    label: str
    rule: str
    rule_name: str
    relay_rule: str
    work: re.Pattern  # a Wiktionary work's id or citation that names the source

    @property
    def judgements(self):
        return SOURCES / self.name / "etc/judgements.jsonl"

    def credited(self, work):
        return bool(self.work.search(work["id"]) or self.work.search(work.get("citation", "")))


SPECS = {
    "wold": Spec(
        "wold",
        "WOLD",
        "agent:wety-match-wold@1",
        "wety's WOLD matcher, v1: WOLD's records to Wiktionary's by key, source and gloss",
        "agent:wety-relay-wold@1",
        re.compile(r"WOLD$|:WLD$|World Loanword Database"),
    ),
    "iecor": Spec(
        "iecor",
        "IE-CoR",
        "agent:wety-match-iecor@1",
        "wety's IE-CoR matcher, v1: IE-CoR's records to Wiktionary's by key, root and gloss",
        "agent:wety-relay-iecor@1",
        re.compile(r"IE-?CoR|Indo-European Cognate Relationships"),
    ),
    "tuled": Spec(
        "tuled",
        "TuLeD",
        "agent:wety-match-tuled@1",
        "wety's TuLeD matcher, v1: TuLeD's records to Wiktionary's by key and gloss",
        "agent:wety-relay-tuled@1",
        re.compile(r"TuLeD|Tupían Lexical Database"),
    ),
}


class Keyer:
    """A record's keys: an attested form's `keys`; a reconstruction's, where the source
    has an orthography profile (`etc/orthography.tsv`), through it on the source's side
    and through Wiktionary's Indo-European profile on Wiktionary's."""

    def __init__(self, spec):
        profile = SOURCES / spec.name / "etc/orthography.tsv"
        self.profiles = None
        if profile.exists():
            self.profiles = {
                "source": orthography.Profile.read(profile),
                "wiktionary": orthography.Profile.read(WIKTIONARY_INE),
            }
        self.cache = {}

    def __call__(self, form, reconstructed, side):
        if not form:
            return frozenset()
        if not (reconstructed and self.profiles):
            return keys(form)
        k = (form, side)
        if k not in self.cache:
            self.cache[k] = frozenset(orthography.key(form, self.profiles[side]).keys)
        return self.cache[k]


def codes_and_keys(src, keyer):
    """The source's exact codes, and for each the keys of its records."""
    wanted = collections.defaultdict(set)
    for r in src.records.values():
        lang = r["language"]
        if lang["relation"] == "exact" and lang["code"] and r["form"]:
            wanted[lang["code"]] |= keyer(r["form"], r["reconstructed"], "source")
    return wanted


def record_code(record_id):
    m = RECORD.match(record_id)
    return m.group(1) if m else None


def scan_export(export, wanted, out, keyer, spec):
    """The part of a graph's export the matcher reads (module docstring), to `out`, in
    three passes over it: records and works; the hypotheses on kept records' origin;
    the assertions on those and those crediting the source, and the records the hypotheses
    name. A line's type is read from its bytes before it is parsed."""
    kept, works, credited, alternatives = {}, [], set(), {}
    codes = {f'"code":"{c}"'.encode() for c in wanted}
    for line in lines(export, binary=True):
        if line.startswith(b'{"form"'):
            m = CODE.search(line)
            if m and m.group(0) in codes:
                o = json.loads(line)
                if (
                    keyer(o["form"], o["reconstructed"], "wiktionary")
                    & wanted[o["language"]["code"]]
                ):
                    kept[o["id"]] = o
        elif any(f'"type":"{t}"'.encode() in line for t in KEPT_WHOLE):
            o = json.loads(line)
            if o["type"] == "alternative_set":
                alternatives[o["id"]] = o
            elif o["type"] in KEPT_WHOLE:
                works.append(o)
                if o["type"] == "work" and spec.credited(o):
                    credited.add(o["id"])
    hyps = {}
    for line in lines(export, binary=True):
        if line.startswith(b'{"id":"hyp:'):
            o = json.loads(line)
            if o["word"] in kept:
                hyps[o["id"]] = o
    named = {
        r for h in hyps.values() for link in h["links"] for r in (link["child"], *link["parents"])
    }
    missing = named - set(kept)
    assertions = []
    for line in lines(export, binary=True):
        if line.startswith(b'{"asserter"'):
            o = json.loads(line)
            if o["target"] in hyps or any(c["work"] in credited for c in o.get("credits", ())):
                assertions.append(o)
        elif line.startswith(b'{"form"') and missing:
            o = json.loads(line)
            if o["id"] in missing:
                kept[o["id"]] = o
    named_sets = {i for a in assertions for i in a.get("in_sets", ())}
    named_sets |= {a["target"] for a in assertions if a["target"].startswith("alt:")}
    objs = works + list(kept.values()) + list(hyps.values()) + assertions
    objs += [alternatives[i] for i in sorted(named_sets) if i in alternatives]
    with gzip.open(out, "wt", encoding="utf-8") as f:
        f.writelines(json.dumps(o, ensure_ascii=False, sort_keys=True) + "\n" for o in objs)
    return len(objs)


def scan_glosses(entries, wanted, out, keyer):
    """The glosses of the entries whose page has a wanted key, one row per part of
    speech, keyed by code, page title (a reconstruction's after its language) and
    etymology label."""
    codes = {c.encode() for c in wanted}
    n = 0
    with gzip.open(out, "wt", encoding="utf-8") as f:
        for line in lines(entries, binary=True):
            if not codes.intersection(LANG_CODE.findall(line[:2000])):
                continue
            e = json.loads(line)
            page, reconstructed = e["page"], e.get("ns") == 118
            if reconstructed and "/" in page:
                page = page.split("/", 1)[1]
            if e.get("lang") not in wanted:
                continue
            if not keyer(page, reconstructed, "wiktionary") & wanted[e["lang"]]:
                continue
            for pos in e.get("pos", []):
                headword = next(iter(pos.get("headword", [])), None)
                row = {
                    "code": e["lang"],
                    "word": page,
                    "canonical": headword,
                    "ety": e.get("ety_label") or "",
                    "glosses": [
                        drop_labels(s["gloss"]) for s in pos.get("senses", []) if s.get("gloss")
                    ],
                }
                f.write(json.dumps(row, ensure_ascii=False) + "\n")
                n += 1
    return n


class Source:
    """A source's assertions (its converter's output): each word's source records (the
    parents of its hypotheses), and which records are such parents."""

    def __init__(self, objects, spec):
        self.spec = spec
        self.objects, self.records, self.hyps, self.origin = (
            {},
            {},
            {},
            collections.defaultdict(list),
        )
        assertions = []
        for o in objects:
            self.objects[o["id"]] = o
            if o["type"] == "record":
                self.records[o["id"]] = o
            elif o["type"] == "hypothesis":
                self.hyps[o["id"]] = o
            elif o["type"] == "assertion":
                assertions.append(o)
        self.sources = collections.defaultdict(list)  # a word's source records
        for a in assertions:
            h = self.hyps.get(a["target"])
            if h:
                self.origin[h["word"]].append(a)
                self.sources[h["word"]] += h["links"][0]["parents"]
        self.parent_ids = {p for ps in self.sources.values() for p in ps}


class Wiktionary:
    """The part of the export `scan` kept, with glosses."""

    def __init__(self, objects, glosses, keyer):
        self.objects, self.records, self.hyps = {}, {}, {}
        self.by_key = collections.defaultdict(set)
        self.origins = collections.defaultdict(list)
        self.assertions = []
        for o in objects:
            self.objects[o["id"]] = o
            t = o["type"]
            if t == "record":
                self.records[o["id"]] = o
                for k in keyer(o["form"], o["reconstructed"], "wiktionary"):
                    self.by_key[(o["language"]["code"], k)].add(o["id"])
            elif t == "hypothesis":
                self.hyps[o["id"]] = o
            elif t == "assertion":
                self.assertions.append(o)
        for a in self.assertions:
            h = self.hyps.get(a["target"])
            if h and a["stance"] == "endorses":
                self.origins[h["word"]].append(h)
        self.glosses = collections.defaultdict(list)
        for g in glosses:
            n = g["ety"].split(".")[0] or "1"
            for term in {g["word"], g["canonical"]} - {None}:
                self.glosses[f"wiktionary:{g['code']}:{term}#{n}"] += g["glosses"]

    def gloss(self, record_id):
        return "; ".join(self.glosses.get(record_id.rstrip("?"), []))

    def named(self, record_id):
        """The records a word's endorsed etymologies name, at any link of their chains."""
        return {
            p
            for h in self.origins.get(record_id, ())
            for link in h["links"]
            for p in link["parents"]
        }


class Matcher:
    """The matcher rule, written for WOLD (`wety-match-wold@1`) and applied as it is to
    IE-CoR and TuLeD under their names (`SPECS`; module docstring for keys). "WOLD"
    below stands for the source, "source word" for a word's parent there (IE-CoR's root).

    A WOLD word's candidates are the Wiktionary records of its language's code with one
    of its keys; an entry's etymologies are records apart (English bay#1 to bay#5). Each
    is weighed by its lineage: whether its etymologies in wety name a word in the
    language of one of WOLD's sources for it with that source word's key ("source
    named"), or another word in such a language ("source language"), or neither; and by
    its gloss against WOLD's meaning. It is eligible when it names the source or its
    gloss fits (similarity `GLOSS_MIN`), or, failing any such, when it is the only
    candidate and no gloss shares nothing with WOLD's. The best by lineage, then gloss
    (closer by `GLOSS_MARGIN`), is matched; equal ones are left as candidates.

    A source word is matched to the record its word's match names in its language with
    its key; failing that, as a word without sources: by gloss, the only candidate taken
    when no gloss contradicts.
    """

    GLOSS_MARGIN = 0.2

    def __init__(self, src, wikt, keyer, judgements=None):
        self.src, self.wikt, self.keyer = src, wikt, keyer
        self.judgements = judgements or Judgements()

    def candidates(self, r):
        code = r["language"]["code"]
        if r["language"]["relation"] != "exact" or not code or not r["form"]:
            return None
        ks = self.keyer(r["form"], r["reconstructed"], "source")
        return sorted({w for k in ks for w in self.wikt.by_key.get((code, k), ())})

    def wikt_keys(self, record_id):
        x = self.wikt.records.get(record_id)
        return self.keyer(x["form"], x["reconstructed"], "wiktionary") if x else frozenset()

    def lineage(self, r, w):
        named = self.wikt.named(w)
        sources = [self.src.records[s] for s in self.src.sources.get(r["id"], ())]
        best = 2
        for s in sources:
            code = s["language"]["code"]
            if not code:
                continue
            same_lang = [x for x in named if record_code(x) == code]
            if not same_lang:
                continue
            best = min(best, 1)
            sk = self.keyer(s["form"], s["reconstructed"], "source")
            if sk and any(self.wikt_keys(x) & sk for x in same_lang):
                best = 0
        return LINEAGE[best]

    def decide_word(self, r):
        found = self.candidates(r)
        if found is None:
            return ("skipped", "language not exact", None)
        if not found:
            return ("unmatched", "no Wiktionary record with this key", None)
        found = [w for w in found if self.judgements.label(r["id"], w) is not False]
        bases = {}
        for w in found:
            b = {"key": "equal", "lineage": self.lineage(r, w)}
            g = gloss_similarity(r.get("gloss"), self.wikt.gloss(w))
            if g is not None:
                b["gloss"] = g
            bases[w] = b
        ok = [
            w
            for w, b in bases.items()
            if b["lineage"] == "source named"
            or b.get("gloss", 0) >= GLOSS_MIN
            or self.judgements.label(r["id"], w)
        ]
        if not ok and len(found) == 1 and bases[found[0]].get("gloss") != 0:
            ok = found
        if not ok:
            return ("unmatched", "candidates without support", bases[found[0]])
        judged = self.judgements.choose(r["id"], ok)
        if judged:
            ok = judged

        def rank(w):
            return (
                -LINEAGE.index(bases[w]["lineage"]),
                bases[w].get("gloss", 0),
                not w.endswith("?"),
            )

        top = max(rank(w) for w in ok)
        kept = [
            w
            for w in ok
            if rank(w)[0] == top[0]
            and top[1] - rank(w)[1] < self.GLOSS_MARGIN
            and rank(w)[2] == top[2]
        ]
        if len(kept) == 1:
            return ("match", kept[0], bases[kept[0]])
        return ("ambiguous", sorted(kept), bases[kept[0]])

    def decide_source(self, s, word_match):
        found = self.candidates(s)
        if found is None:
            return ("skipped", "language not exact", None)
        if word_match:
            named = self.wikt.named(word_match)
            hit = [w for w in found if w in named]
            if len(hit) == 1:
                return ("match", hit[0], {"key": "equal", "lineage": "named by its word's match"})
        if not found:
            return ("unmatched", "no Wiktionary record with this key", None)
        bases = {w: {"key": "equal"} for w in found}
        for w in found:
            g = gloss_similarity(s.get("gloss"), self.wikt.gloss(w))
            if g is not None:
                bases[w]["gloss"] = g
        ok = [w for w in found if bases[w].get("gloss", 0) >= GLOSS_MIN]
        if not ok and len(found) == 1 and bases[found[0]].get("gloss") != 0:
            ok = found
        if len(ok) == 1:
            return ("match", ok[0], bases[ok[0]])
        if ok:
            best = max(bases[w].get("gloss", 0) for w in ok)
            kept = sorted(w for w in ok if best - bases[w].get("gloss", 0) < self.GLOSS_MARGIN)
            if len(kept) == 1:
                return ("match", kept[0], bases[kept[0]])
            return ("ambiguous", kept, bases[kept[0]])
        return ("unmatched", "candidates without support", bases[found[0]])

    def run(self):
        decisions = {}
        words = [r for r in self.src.records.values() if r["id"] not in self.src.parent_ids]
        for r in words:
            decisions[r["id"]] = self.decide_word(r)
        for r in words:
            what, target, _ = decisions[r["id"]]
            for s in dict.fromkeys(self.src.sources.get(r["id"], ())):
                decisions[s] = self.decide_source(
                    self.src.records[s], target if what == "match" else None
                )
        return decisions


class Relayer:
    """The relay rule, written for WOLD (`wety-relay-wold@1`) and applied as it is to the
    others under their names (`SPECS`): each Wiktionary assertion that credits WOLD, linked to
    WOLD's own assertion on the origin of the same word, through the matches. They agree
    when WOLD's source word is matched to a word the assertion's chain names, or, failing
    a match, the chain names a word of the source's language with its key; they differ in
    `parents` when the chain names another word of that language, and in `mode` when
    WOLD says borrowed and the first link does not. Of WOLD's several origins for a word
    the one with fewest differences is relayed."""

    def __init__(self, src, wikt, decisions, matcher):
        self.src, self.wikt, self.matcher = src, wikt, matcher
        self.to_source = collections.defaultdict(set)
        self.to_wikt = {}
        for rid, (what, target, _) in decisions.items():
            if what == "match":
                self.to_source[target].add(rid)
                self.to_wikt[rid] = target
        self.works = {
            o["id"] for o in wikt.objects.values() if o["type"] == "work" and src.spec.credited(o)
        }

    def compare(self, a, h):
        words = self.to_source.get(h["word"])
        if not words:
            return None, "word not matched"
        origins = [o for w in sorted(words) for o in self.src.origin.get(w, ())]
        if not origins:
            return None, f"{self.src.spec.label} gives the matched word no origin"
        named = {p for link in h["links"] for p in link["parents"]}
        first = h["links"][0]
        results = []
        for o in origins:
            link = self.src.hyps.get(o["target"], {}).get("links", [None])[0]
            if link is None:
                continue
            diffs, determined = [], False
            for p in link["parents"]:
                s = self.src.records[p]
                if self.to_wikt.get(p) in named:
                    determined = True
                    continue
                code = s["language"]["code"]
                same = [x for x in named if record_code(x) == code]
                if not same:
                    continue
                determined = True
                if not any(
                    self.matcher.wikt_keys(x)
                    & self.matcher.keyer(s["form"], s["reconstructed"], "source")
                    for x in same
                ):
                    diffs.append("parents")
            if link.get("mode") == "borrowed" and first.get("mode") not in BORROWINGS:
                diffs.append("mode")
            if determined:
                results.append((len(diffs), o, diffs))
        if not results:
            return (
                None,
                f"Wiktionary names no word in the language of {self.src.spec.label}'s source",
            )
        _, o, diffs = min(results, key=lambda x: x[0])
        return o, diffs

    def run(self):
        relays, outcomes = [], []
        for a in self.wikt.assertions:
            works = [c["work"] for c in a.get("credits", []) if c["work"] in self.works]
            h = self.wikt.hyps.get(a["target"])
            if not works or not h:
                continue
            o, what = self.compare(a, h)
            outcomes.append((a, works[0], o, what))
            if o is not None:
                relays.append(A.relay(a["id"], o["id"], works[0], self.src.spec.relay_rule, what))
        return relays, outcomes


def run(args):
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    spec = SPECS[args.source]
    keyer = Keyer(spec)
    src = Source(read_lines(args.assertions), spec)
    wikt = Wiktionary(
        read_lines(args.work / "wiktionary.jsonl.gz"),
        read_lines(args.work / "glosses.jsonl.gz"),
        keyer,
    )
    judgements = Judgements.read(spec.judgements)
    matcher = Matcher(src, wikt, keyer, judgements)
    decisions = matcher.run()
    inputs = [args.assertions, args.export, args.entries]
    inputs += [
        p for p in [spec.judgements, SOURCES / spec.name / "etc/orthography.tsv"] if p.exists()
    ]
    extraction = A.extraction("converter", "packages/etymology/match_words.py", inputs)
    new = {extraction["id"]: extraction}
    relay_name = f"wety's {spec.label} relay rule, v1: Wiktionary's assertions crediting {spec.label} to its own"
    for agent_id, name in ((spec.rule, spec.rule_name), (spec.relay_rule, relay_name)):
        new[agent_id] = {"type": "agent", "id": agent_id, "kind": "rule", "name": name}
    matches = []
    for rid, (what, target, found) in decisions.items():
        basis = match_basis(found)
        if what == "match":
            matches.append(
                A.match(rid, spec.rule, 1.0, basis, target=target, extraction=extraction["id"])
            )
        elif what == "ambiguous":
            matches.append(
                A.match(rid, spec.rule, 1.0, basis, candidates=target, extraction=extraction["id"])
            )
    relays, outcomes = Relayer(src, wikt, decisions, matcher).run()
    writer = closure(matches + relays, new, src.objects, wikt.objects)
    n = writer.write(out / "assertions.jsonl")
    print(f"{out / 'assertions.jsonl'}: {n} objects")
    rows = []
    for rid, (what, target, basis) in decisions.items():
        r = src.records[rid]
        rows.append(
            {
                "record": rid,
                "form": r["form"],
                "gloss": r.get("gloss"),
                "code": r["language"]["code"],
                "kind": "source word" if rid in src.parent_ids else "word",
                "outcome": what,
                "detail": target,
                "basis": basis,
            }
        )
    with gzip.open(out / "decisions.jsonl.gz", "wt", encoding="utf-8") as f:
        f.writelines(json.dumps(x, ensure_ascii=False) + "\n" for x in rows)
    relay_rows = [
        {
            "assertion": a["id"],
            "locator": a["locator"],
            "work": work,
            "source_assertion": o and o["id"],
            "outcome": ("agrees" if not what else "differs") if o else "no relay",
            "detail": what,
        }
        for a, work, o, what in outcomes
    ]
    (out / "relays.json").write_text(json.dumps(relay_rows, ensure_ascii=False, indent=1) + "\n")
    c = collections.defaultdict(collections.Counter)
    for x in rows:
        c[f"{x['kind']}s"][x["outcome"]] += 1
        if x["outcome"] == "unmatched":
            c[f"unmatched {x['kind']}s: why"][x["detail"]] += 1
        if x["outcome"] == "match":
            c[f"matched {x['kind']}s: lineage"][x["basis"].get("lineage", "none")] += 1
        if x["outcome"] in ("match", "ambiguous", "unmatched"):
            c[f"{x['kind']}s by code"][f"{x['code']}: {x['outcome']}"] += 1
    for r in relay_rows:
        c["relays"][r["outcome"]] += 1
        if r["outcome"] == "differs":
            c["relays: differences"][", ".join(r["detail"])] += 1
        if r["outcome"] == "no relay":
            c["no relay: why"][r["detail"]] += 1
    counts = {k: dict(sorted(v.items())) for k, v in sorted(c.items())}
    counts["objects"] = n
    found = {x["record"]: x["detail"] for x in rows if x["outcome"] in ("match", "ambiguous")}
    counts["judgements"] = judgements.score(found)
    (out / "counts.json").write_text(json.dumps(counts, ensure_ascii=False, indent=1) + "\n")
    rng = random.Random(args.seed)
    targeted = sorted((m for m in matches if "target" in m), key=lambda m: m["id"])
    sample = [review_entry(m, src, wikt) for m in rng.sample(targeted, min(100, len(targeted)))]
    (out / "sample.json").write_text(json.dumps(sample, ensure_ascii=False, indent=1) + "\n")


def match_basis(basis):
    """A decision's basis in the schema's terms: a word whose candidate's etymology names
    its source's parent shares an ancestor with it (`ancestors`: one shared, or none of
    one cited when only the parent's language is named); a source word named by its
    word's match shares a descendant (`reflexes`)."""
    if basis is None:
        return None
    out = {k: v for k, v in basis.items() if k in ("key", "gloss")}
    lineage = basis.get("lineage")
    if lineage in ("source named", "source language"):
        out["ancestors"] = {"shared": int(lineage == "source named"), "cited": 1}
    elif lineage == "named by its word's match":
        out["reflexes"] = {"shared": 1, "cited": 1}
    return out


def review_entry(m, src, wikt):
    """A match as a reviewer reads it (`matching.harvest` reads it as the ACD's are)."""
    r = src.records[m["record"]]
    w = wikt.records[m["target"]]
    sources = [src.records[s] for s in src.sources.get(r["id"], ())]
    return {
        "match": m["id"],
        "source": src.spec.name,
        src.spec.name: {
            "record": r["id"],
            "language": r["language"]["label"],
            "form": r["form"],
            "gloss": r.get("gloss"),
            "sources": [f"{s['language']['label']} {s['form'] or ''}".strip() for s in sources],
        },
        "wiktionary": {
            "record": w["id"],
            "form": w["form"],
            "gloss": wikt.gloss(w["id"]),
            "parents": sorted(wikt.named(w["id"])),
        },
        "basis": m["basis"],
        "verdict": None,
        "cause": None,
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="step", required=True)
    s = sub.add_parser("scan", help="the part of the export and the glosses the matcher reads")
    s.add_argument("--source", choices=sorted(SPECS), required=True)
    s.add_argument("--assertions", type=Path, required=True, help="the source's converted release")
    s.add_argument("--export", type=Path, required=True, help="a graph's assertions.jsonl.gz")
    s.add_argument("--entries", type=Path, required=True, help="wety's entries")
    s.add_argument("--work", type=Path, required=True, help="the work directory")
    m = sub.add_parser("run", help="match, relay, and write the check's files")
    m.add_argument("--source", choices=sorted(SPECS), required=True)
    m.add_argument("--assertions", type=Path, required=True)
    m.add_argument("--export", type=Path, required=True)
    m.add_argument("--entries", type=Path, required=True)
    m.add_argument("--work", type=Path, required=True)
    m.add_argument("--out", type=Path, required=True, help="the check directory")
    m.add_argument("--seed", type=int, default=20261110)
    args = p.parse_args(argv)
    if args.step == "scan":
        args.work.mkdir(parents=True, exist_ok=True)
        spec = SPECS[args.source]
        keyer = Keyer(spec)
        wanted = codes_and_keys(Source(read_lines(args.assertions), spec), keyer)
        n = scan_export(args.export, wanted, args.work / "wiktionary.jsonl.gz", keyer, spec)
        print(f"wiktionary.jsonl.gz: {n} objects")
        n = scan_glosses(args.entries, wanted, args.work / "glosses.jsonl.gz", keyer)
        print(f"glosses.jsonl.gz: {n} entries")
    if args.step == "run":
        run(args)


if __name__ == "__main__":
    main()
