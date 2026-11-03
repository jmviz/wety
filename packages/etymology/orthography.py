"""Comparison keys for reconstructed forms, from a source's orthography profile.

The same reconstruction is spelled differently from source to source: de Vaan's
*ulkʷo‑ and Wiktionary's *wĺ̥kʷos. A profile maps each source's graphemes to those of
a shared key, so that two spellings of one word get a common key. A key alone never
decides a match; it is one signal beside the gloss and the reflexes.

A profile is a TSV file with the columns Kind, Grapheme, Key and Note:

- `grapheme` rows map a grapheme (one or more characters, compared after NFD) to its
  key; an empty key deletes it. Plain ASCII letters not listed map to themselves in
  lower case; any other character not listed is kept and reported as unknown.
- `ending` rows name an ending (in key graphemes) that may be cut from the end of a
  key: a stem vowel, or a citation form's inflection. Each cut adds a key.
- `include` rows name a convention, a profile in `conventions/` that several sources
  write in (`ine`: the Leiden series' and Wiktionary's Indo-European notation). Its rows
  come first; the source's own rows may replace a grapheme's key.

Notation the key reads in every source: `(x)` marks an optional segment and `a/b`
alternative segments, each expanded both ways and counted as uncertain; ` / ` and
`;` separate whole forms; a `?` marks the form as hedged.
"""

import csv
import unicodedata
from dataclasses import dataclass, field
from pathlib import Path

CONVENTIONS = Path(__file__).parent / "conventions"


@dataclass(frozen=True)
class Profile:
    graphemes: dict
    endings: tuple

    @classmethod
    def read(cls, path):
        graphemes, endings, own = {}, [], set()
        with Path(path).open(newline="") as f:
            for row in csv.DictReader(
                (line for line in f if not line.startswith("#")), delimiter="\t"
            ):
                grapheme = unicodedata.normalize("NFD", row["Grapheme"])
                if row["Kind"] == "grapheme":
                    if grapheme in own:
                        raise ValueError("grapheme listed twice: " + row["Grapheme"])
                    own.add(grapheme)
                    graphemes[grapheme] = row["Key"] or ""
                elif row["Kind"] == "ending":
                    endings.append(grapheme)
                elif row["Kind"] == "include":
                    convention = cls.read(CONVENTIONS / f"{row['Grapheme']}.tsv")
                    graphemes = {**convention.graphemes, **graphemes}
                    endings = [*convention.endings, *endings]
                else:
                    raise ValueError("unknown kind " + row["Kind"])
        return cls(graphemes, tuple(endings))


@dataclass
class Key:
    keys: set = field(default_factory=set)
    uncertain: int = 0
    hedged: bool = False
    unknown: set = field(default_factory=set)


SYNTAX = {"(", ")", "/"}


def segments(form, profile):
    """Split a form into segments: syntax marks, and each letter with the combining
    marks after it, as a tuple of profile graphemes (longest first)."""
    longest = max(map(len, profile.graphemes), default=1)
    graphemes, i = [], 0
    while i < len(form):
        for n in range(min(longest, len(form) - i), 0, -1):
            if form[i : i + n] in profile.graphemes:
                graphemes.append(form[i : i + n])
                i += n
                break
        else:
            if not form[i].isspace():
                graphemes.append(form[i])
            i += 1
    out = []
    for g in graphemes:
        marks = all(unicodedata.combining(c) for c in g)
        if marks and out and out[-1] not in SYNTAX:
            out[-1] = out[-1] + (g,)
        else:
            out.append(g if g in SYNTAX else (g,))
    return out


def expand(segs):
    """Every reading of a segment list, and how many uncertain places it has."""
    groups, uncertain, i = [], 0, 0
    while i < len(segs):
        s = segs[i]
        if s == "(":
            depth, j = 1, i + 1
            while j < len(segs) and depth:
                depth += {"(": 1, ")": -1}.get(segs[j], 0)
                j += 1
            inner, n = expand(segs[i + 1 : j - 1])
            groups.append(inner + [[]])
            uncertain += n + 1
            i = j
        elif s == "/" and groups and i + 1 < len(segs):
            right, n = expand(segs[i + 1 : i + 2])
            groups[-1] = groups[-1] + right
            uncertain += n + 1
            i += 2
        elif s in SYNTAX:
            i += 1
        else:
            groups.append([[s]])
            i += 1
    readings = [[]]
    for choices in groups:
        readings = [r + c for r in readings for c in choices]
    return readings, uncertain


def key(text, profile):
    """The comparison keys of a form as a source writes it."""
    text = unicodedata.normalize("NFD", text)
    result = Key(hedged="?" in text)
    text = text.replace("?", "")
    forms = [f for part in text.split(";") for f in part.split(" / ")]
    for form in forms:
        readings, uncertain = expand(segments(form.strip(), profile))
        result.uncertain += uncertain
        for reading in readings:
            out = []
            for s in (g for unit in reading for g in unit):
                if s in profile.graphemes:
                    out.append(profile.graphemes[s])
                elif s.isascii() and s.isalpha():
                    out.append(s.lower())
                else:
                    out.append(s)
                    result.unknown.add(s)
            k = "".join(out)
            if not k:
                continue
            result.keys.add(k)
            for ending in profile.endings:
                if k.endswith(ending) and len(k) - len(ending) >= 2:
                    result.keys.add(k[: -len(ending)])
    return result


def same(a, b):
    """Whether two keys share a reading."""
    return bool(a.keys & b.keys)
