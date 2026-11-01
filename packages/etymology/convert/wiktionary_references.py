"""Wiktionary's References sections, read from page wikitext: entry-scope credits (core-5).

wiktextract has no field for References, so they are read from the pages dump that
matches the wiktextract release (`etc/source.json`, artifact `pages`). Three steps:

- `extract` reads the dump once and keeps every References and Further reading section
  of the entry and reconstruction namespaces, with the language and etymology it sits
  under, and the wikitext of every `Template:R:...` page;
- `entries` lists wiktextract's entries (page, language, etymology label), the keys
  wety's items are made from, and whether each has an etymology;
- `convert` turns them into a bibliography and credits, and checks both (`check`). Every `Template:R:...` page that
  is not a redirect becomes a work, read from the citation template it calls (authors,
  editors when it names no author, year, title); a `{{cite-...}}` line of a References
  section becomes a work too. Each entry's References then give its entry-scope
  credits, role `follows`: a section beside the etymologies covers every etymology of
  its language, one under "Etymology N" only that one. Further reading lists what a
  reader may consult, not what the editors did, and gives no credits; it is counted.

A credit's locator is the page the template's arguments give, else the headword
("s.v. lupus"): which argument is which is read from the template's own citation call.
"""

import argparse
import collections
import gzip
import io
import json
import re
import subprocess
import unicodedata
from pathlib import Path

from etymology import assertions as A
from etymology import wikitext
from etymology.convert import agent, source, work

NAME = "wiktionary"
EDITORS = "agent:wiktionary-editors"
KEPT = {"References": "references", "Further reading": "further"}
BIBLIOGRAPHIC = re.compile(r"refer|reading|biblio|sources?$|literat", re.IGNORECASE)


def open_dump(path):
    """The dump's lines; bzip2 decompresses in its own process, beside the parse."""
    if str(path).endswith(".bz2") or Path(path).name == "source":
        p = subprocess.Popen(["bzip2", "-dc", str(path)], stdout=subprocess.PIPE)
        return io.TextIOWrapper(p.stdout, encoding="utf-8")
    return open(path, encoding="utf-8")


def extract(dump, out):
    counts = collections.Counter()
    headings = collections.Counter()
    with gzip.open(out, "wt", encoding="utf-8") as f:
        for title, ns, text in wikitext.pages(open_dump(dump)):
            counts[f"pages in namespace {ns}"] += 1
            if ns == wikitext.TEMPLATE:
                if title.startswith("Template:R:"):
                    name = wikitext.normalize_name(title)
                    row = {"template": name, "text": text}
                    target = wikitext.redirect(text)
                    if target:
                        row["redirect"] = wikitext.normalize_name(target)
                    f.write(json.dumps(row, ensure_ascii=False) + "\n")
                    counts["R: templates"] += 1
                continue
            if wikitext.redirect(text):
                continue
            langs = set()
            for lang, ety, level, name, body in wikitext.sections(text):
                langs.add(lang)
                if BIBLIOGRAPHIC.search(name):
                    headings[name] += 1
                kind = KEPT.get(name)
                if kind and body.strip():
                    row = {"page": title, "lang": lang, "ety": ety, "level": level}
                    f.write(
                        json.dumps(row | {"kind": kind, "text": body}, ensure_ascii=False) + "\n"
                    )
                    counts[f"{kind} sections"] += 1
            counts["language sections"] += len(langs)
    return {
        "dump": str(dump),
        "counts": dict(sorted(counts.items())),
        "headings": dict(headings.most_common(40)),
    }


def page_title(d):
    """The page a wiktextract entry comes from: reconstructions live in their namespace."""
    word = d["word"]
    reconstructed = any("reconstruction" in (s.get("tags") or []) for s in d.get("senses") or [])
    if reconstructed:
        return f"Reconstruction:{d['lang']}/{word.removeprefix('*')}"
    return word


def ety_label(d):
    """wiktextract's etymology_number as the processor reads it (`get_ety_label`)."""
    v = d.get("etymology_number")
    label = str(v).strip() if v is not None else ""
    return label or None


def canonical_term(d):
    """The form the processor names an entry by (`get_canonical_term`): its canonical
    form ("vocō" on page voco) when it has as many words as the page title, else the
    page's word."""
    word = d["word"]
    for form in d.get("forms") or []:
        f = form.get("form")
        if "canonical" in (form.get("tags") or []) and f and "," not in f:
            if len(f.split()) == len(word.split()):
                return f
    return word


def ety_num(label):
    """The processor's etymology number for a label (Wiktionary's number, else 1). It
    raises the number of an entry whose term and number another page already took."""
    return int(label) if label and label.isdigit() and int(label) <= 255 else 1


def entries(wiktextract, out):
    """One row per entry key (page, language, etymology label), deduplicated as the
    processor and the core-2 scan do, with the entry's language code and canonical term;
    the last field says whether any of its lines has an etymology (text or templates)."""
    seen = {}
    with gzip.open(wiktextract, "rt", encoding="utf-8") as f:
        for line in f:
            d = json.loads(line)
            if not d.get("word") or not d.get("lang_code") or "pos" not in d:
                continue
            if "," in d["word"] or "phrase" in d["pos"]:
                continue  # the processor ignores these (`should_ignore_term`, `_pos`)
            key = (page_title(d), d["lang"], ety_label(d))
            has = bool(d.get("etymology_text") or d.get("etymology_templates"))
            row = seen.get(key)
            if row is None:
                seen[key] = [d["lang_code"], canonical_term(d), has]
            elif has:
                row[2] = True
    with gzip.open(out, "wt", encoding="utf-8") as f:
        for (page, lang, ety), (code, term, has) in seen.items():
            f.write(json.dumps([page, lang, ety, code, term, has], ensure_ascii=False) + "\n")
    with_ety = sum(1 for r in seen.values() if r[2])
    return {"entries": len(seen), "with an etymology": with_ety}


# The citation call of an R: template page.

CITE = re.compile(r"^cite[- ](book|journal|web|text|av|newsgroup|video)$")
SUBPAGE = re.compile(r"/(documentation|doc|testcases|sandbox)\b", re.IGNORECASE)
PARAM = re.compile(r"\{\{\{\s*([^{}|]*?)\s*(?:\|([^{}]*))?\}\}\}")
LINK = re.compile(r"\[\[(?:[^\]|]*\|)?([^\]]*)\]\]")
EXTLINK = re.compile(r"\[(?:https?:)?//[^\s\]]*\s*([^\]]*)\]")
TAG = re.compile(r"<[^>]*>")
BOLD = re.compile(r"'{2,}")
YEAR = re.compile(r"\b(1[0-9]{3}|20[0-9]{2})\b")
INTERWIKI = re.compile(
    r"(?<![\w:])(?:w:(?:[a-z]{2,3}(?:-[a-z]{2,8})*:)?|[a-z]{2,3}(?:-[a-z]{2,8})*:)(?=\w)"
)
INITIALS = re.compile(r"(?:\w\.\s*)+\w?\.?")
ETAL = re.compile(r",?\s*\bet\.? al(?:ii|\.)?", re.IGNORECASE)
EMPTY_QUOTES = re.compile(r"\"\"|“”|«»|''")
NAMED = re.compile(r"\s*[\w -]+?\s*")
PAGE_FIELDS = {"page", "pages", "p", "pp"}
ENTRY_FIELDS = {"entry", "head", "w", "lemma", "word"}
TERM_TEMPLATES = {"lang", "m", "l", "mention", "link"}
TEXT_TEMPLATES = {"nowrap", "small", "smallcaps", "sc"}


def expand(text):
    """A template argument as a reader sees it when no arguments are given: parameters
    take their defaults, #if its else branch, #switch its default, {{w}} and term
    templates their text; other templates are dropped."""
    text = wikitext.COMMENT.sub("", text)
    for _ in range(10):
        new = PARAM.sub(lambda m: m.group(2) or "", text)
        if new == text:
            break
        text = new
    out, i = [], 0
    for start, end, name, args in template_spans(text):
        out.append(text[i:start])
        out.append(render(name, args))
        i = end
    out.append(text[i:])
    text = LINK.sub(lambda m: m.group(1), "".join(out))
    text = EXTLINK.sub(lambda m: m.group(1), text)
    text = BOLD.sub("", TAG.sub("", text))
    return re.sub(r"\s+", " ", text).strip()


def render(name, args):
    a = wikitext.named_args(args)
    low = name.lower()
    if low.startswith("#if:"):
        return expand(a.get("1", "") if expand(name[4:]) else a.get("2", ""))
    if low.startswith("#switch:"):
        for raw in args:
            k, eq, v = raw.partition("=")
            if eq and k.strip() == "#default":
                return expand(v)
        return ""
    if low in ("w", "lw"):
        return expand(a.get("2") or a.get("1", ""))
    if low in TERM_TEMPLATES:
        return expand(a.get("2", ""))
    if low in TEXT_TEMPLATES:
        return expand(a.get("1", ""))
    return ""


def template_spans(text):
    """(start, end, name, args) of each outermost template in text."""
    i = 0
    while True:
        i = text.find("{{", i)
        if i < 0:
            return
        end = wikitext._close(text, i)
        if end is None:
            return
        parts = wikitext._split(text[i + 2 : end - 2])
        yield i, end, wikitext.normalize_name(parts[0]), parts[1:]
        i = end


def raw_args(args):
    """Arguments by name or position, values as written (parameters unexpanded)."""
    out, n = {}, 0
    for a in args:
        k, eq, v = a.partition("=")
        if eq and NAMED.fullmatch(k):
            out[k.strip()] = v.strip()
        else:
            n += 1
            out[str(n)] = a.strip()
    return out


def citation_call(text):
    """The citation template a template page calls, as (name, raw arguments), or None.
    A call through Module:quote names its template in `template=`."""
    text = re.sub(r"<noinclude>.*?(?:</noinclude>|$)", "", text, flags=re.DOTALL)
    text = re.sub(r"</?(includeonly|onlyinclude)>", "", wikitext.COMMENT.sub("", text))
    for name, args in wikitext.templates(text):
        raw = raw_args(args)
        if name.startswith("#invoke:quote") and CITE.match(raw.get("template", "")):
            return raw["template"], raw
        if CITE.match(name.lower()) or name == "R:Reference-meta":
            return name, raw
    return None


def unprefix(text):
    """Text without Module:quote's link prefixes: "w:gl:Antón Santamarina" (a Wikipedia,
    here Galician) or "ru:Андреев"."""
    return INTERWIKI.sub("", text)


def person_name(name):
    """ "Vasmer, Max" and "Max Vasmer" alike as "Max Vasmer"; link prefixes and a
    Wikipedia title's disambiguator ("(linguist)") are dropped."""
    name = re.sub(r"\s*\([^)]*\)\s*$", "", unprefix(name.replace("_", " "))).strip(" ,;:")
    if name.count(",") == 1:
        last, first = (x.strip() for x in name.split(","))
        if last and first and len(first.split()) <= 4 and not any(c.isdigit() for c in name):
            name = f"{first} {last}"
    return re.sub(r"\s+", " ", name).strip()


def split_people(value):
    """Names in one field: separated by semicolons, "and" (in a few languages), or commas
    between full names."""
    value = ETAL.sub("", value)
    parts = re.split(r"\s*;\s*|\s+(?:and|und|et|и|&)\s+", value)
    out = []
    for p in parts:
        bits = [b.strip() for b in p.split(",")]
        if len(bits) > 2 and all(" " in b for b in bits if b):
            out += bits
        elif (
            len(bits) >= 4 and len(bits) % 2 == 0 and all(INITIALS.fullmatch(b) for b in bits[1::2])
        ):
            # "Lytkin, V. I., Gulyaev, E. S.": surnames and initials in turn.
            out += [f"{bits[i]}, {bits[i + 1]}" for i in range(0, len(bits), 2)]
        else:
            out.append(p)
    names = (person_name(p) for p in out)
    return [n for n in names if len(n) > 1 and not n.lower().startswith(("et al", "others"))]


def people(call, role):
    """The authors (role "author") or editors a citation call names, in order; a
    Wikipedia link (`authorlink`) gives a lone name's spelling."""
    found = []
    for n in [""] + [str(i) for i in range(1, 10)]:
        link = expand(call.get(f"{role}link{n}", "") or call.get(f"{role}-link{n}", ""))
        if role == "author":
            last = expand(call.get(f"last{n}", ""))
            if last:
                first = expand(call.get(f"first{n}", ""))
                found.append(person_name(link) if link else f"{first} {last}".strip())
                continue
        names = split_people(expand(call.get(f"{role}{n}", "")))
        if link and len(names) == 1:
            names = [person_name(link)]
        found += names
    if not found:
        found = split_people(expand(call.get(f"{role}s", "")))
    if role == "author" and not found:
        found = split_people(expand(call.get("coauthors", "")))
    return list(dict.fromkeys(found))


def slug(name):
    s = unicodedata.normalize("NFKD", name).encode("ascii", "ignore").decode().lower()
    s = re.sub(r"[^a-z0-9]+", "-", s).strip("-")
    return s or "x" + A.digest(name)[:8]


def argument_roles(call):
    """Which of a template's own arguments give the page and which the headword, read
    from the fields of its citation call that use them."""
    roles = {}
    if call.get("pageparam"):
        roles[call["pageparam"].strip()] = "page"
    for field, value in call.items():
        kind = "page" if field in PAGE_FIELDS else "entry" if field in ENTRY_FIELDS else None
        if kind:
            for m in re.finditer(r"\{\{\{\s*([^{}|]+?)\s*[|}]", value):
                name = m.group(1)
                if name.isdigit() or name in PAGE_FIELDS or name in ENTRY_FIELDS:
                    roles.setdefault(name, kind)
    return roles


def describe(call):
    """A citation call's authors (editors when it names none), year, title and citation."""
    authors = people(call, "author")
    edited = not authors
    if edited:
        authors = people(call, "editor")
    title = unprefix(expand(call.get("title", "") or call.get("work", "")))
    part = EMPTY_QUOTES.sub("", expand(call.get("article", "") or call.get("chapter", "")))
    if part.strip(" ,"):
        title = f"{part}, in {title}" if title else part
    year = None
    for field in ("year", "date", "year_published"):
        m = YEAR.search(expand(call.get(field, "")))
        if m:
            year = int(m.group(1))
            break
    where = ": ".join(
        x for x in (expand(call.get("location", "")), expand(call.get("publisher", ""))) if x
    )
    head = "; ".join(authors) + (" (ed.)" if authors and edited else "")
    if year:
        head = f"{head} ({year})".strip()
    out = {"title": title, "year": year, "authors": authors}
    if title:
        out["citation"] = ". ".join(x for x in (head, title, where) if x)
    return out


def plain(text):
    """A template page that calls no citation template: its text is the citation."""
    text = re.sub(r"<noinclude>.*?(?:</noinclude>|$)", "", text, flags=re.DOTALL)
    if "#invoke:" in text:
        return {}
    citation = expand(text)
    if not citation or len(citation) > 400:
        return {}
    m = YEAR.search(citation)
    return {"citation": citation, "year": int(m.group(1)) if m else None}


def locator(roles, args):
    """The page a template's arguments give, else its headword as "s.v. ...", else None."""
    page = headword = None
    for k, v in wikitext.named_args(args).items():
        v = expand(v)
        if not v:
            continue
        kind = roles.get(k) or ("page" if k in PAGE_FIELDS else None)
        if kind == "page" and page is None:
            page = v
        elif kind == "entry" and headword is None:
            headword = v
    if page:
        return page
    return f"s.v. {headword}" if headword else None


class Bibliography:
    """Works seeded from the R: template pages, and the people they name."""

    def __init__(self, rows, out):
        self.out = out
        self.pages = {r["template"]: r for r in rows if not SUBPAGE.search(r["template"])}
        self.works = {}
        self.roles = {}
        self.counts = collections.Counter()

    def canonical(self, name):
        seen = set()
        while name in self.pages and "redirect" in self.pages[name] and name not in seen:
            seen.add(name)
            name = self.pages[name]["redirect"]
        return name

    def person(self, name):
        """One agent per slug: the first spelling seen ("J. P. Mallory") names it."""
        aid = f"agent:{slug(name)}"
        if aid not in self.out.objects:
            self.out.add(agent(aid, "person", name))
        return aid

    def template_work(self, name):
        """The work an R: template cites, under the name its redirects lead to."""
        name = self.canonical(name)
        if name in self.works:
            return self.works[name]
        page = self.pages.get(name)
        call = citation_call(page["text"]) if page else None
        self.roles[name] = argument_roles(call[1]) if call else {}
        if not page:
            self.counts["works with no template page"] += 1
        elif call:
            self.counts[f"works read from {call[0]}"] += 1
        elif page["text"].lstrip().lower().startswith("#redirect"):
            self.counts["works whose redirect leads nowhere"] += 1
        else:
            self.counts["works from a page with no citation call"] += 1
        fields = describe(call[1]) if call else plain(page["text"]) if page else {}
        if call and not fields.get("citation"):
            self.counts["works read with no title"] += 1
        w = self.make(f"work:{NAME}:{name.replace(' ', '_')}", fields, "{{" + name + "}}")
        w["x"] = {NAME: {"template": name}}
        self.works[name] = self.out.add(w)
        return self.works[name]

    def cite_work(self, args):
        """A work cited by a {{cite-...}} line, the same work wherever its authors,
        title and year agree; None when it names no title."""
        fields = describe(raw_args(args))
        if not fields.get("citation"):
            return None
        key = [fields["authors"], fields["title"], fields["year"]]
        wid = f"work:{NAME}:cite:{A.digest(key)}"
        if wid not in self.out.objects:
            # The first citation seen speaks for the others (they differ in place, publisher).
            self.out.add(self.make(wid, fields, None))
        return wid

    def make(self, wid, fields, fallback):
        w = work(wid, fields.get("citation") or fallback)
        if fields.get("year"):
            w["year"] = fields["year"]
        if fields.get("authors"):
            w["authors"] = [self.person(p) for p in fields["authors"]]
        return w

    def seed(self):
        for name, page in self.pages.items():
            if "redirect" not in page:
                self.template_work(name)


def works_in(bib, out, body):
    """(work id, locator, how) of each work a section's text cites outside footnotes."""
    found = []
    for name, args in wikitext.templates(body):
        if name.startswith("R:"):
            loc = locator(bib.roles.get(bib.canonical(name), {}), args)
            found.append((bib.template_work(name), loc, "R:"))
        elif name == "R" and len(args) >= 2:
            # {{R|<lang>|<key>}}: a reference kept in a language's data module.
            lang, key = args[0].strip(), args[1].strip()
            w = work(f"work:{NAME}:R|{lang}|{key}".replace(" ", "_"), f"{{{{R|{lang}|{key}}}}}")
            w["x"] = {NAME: {"template": f"R|{lang}"}}
            found.append((out.add(w), locator({}, args[2:]), "R"))
        elif CITE.match(name.lower()):
            wid = bib.cite_work(args)
            found.append((wid, None, "cite" if wid else "cite, no title"))
    return found


def record_id(code, term, label):
    """The id the processor gives an entry's word (`graph-assertions`), unless another
    page took its term and number first; (page, code, label) always identify it."""
    return f"{NAME}:{code}:{term}#{ety_num(label)}"


def entry_key(key, code):
    page, _, label = key
    return {"page": page, "lang": code, "label": label, "ety_num": ety_num(label)}


def convert(sections_path, entries_path, out_dir):
    """Write the bibliography (`assertions.jsonl`), each entry's credits
    (`entry_credits.jsonl.gz`) and its Further reading works; return the counts."""
    out = A.Writer()
    out.add(source(NAME))
    tool = "packages/etymology/convert/wiktionary_references.py"
    out.add(A.extraction("parser", tool, [sections_path, entries_path]))
    out.add(agent(EDITORS, "editors", "Wiktionary's editors"))
    templates, sections = [], []
    with gzip.open(sections_path, "rt", encoding="utf-8") as f:
        for line in f:
            row = json.loads(line)
            (templates if "template" in row else sections).append(row)
    bib = Bibliography(templates, out)
    bib.seed()
    counts = collections.Counter()
    wanted = {(s["page"], s["lang"]) for s in sections}
    index = collections.defaultdict(list)
    with gzip.open(entries_path, "rt", encoding="utf-8") as f:
        for line in f:
            page, lang, ety, code, word, has = json.loads(line)
            counts["entries"] += 1
            counts["entries with an etymology"] += has
            if (page, lang) in wanted:
                index[page, lang].append((ety, code, word, has))
    credits = collections.defaultdict(dict)
    further = collections.defaultdict(lambda: (None, set()))
    with_section = {}
    for s in sections:
        kind = s["kind"]
        body, notes = wikitext.strip_refs(s["text"])
        counts[f"{kind}: footnotes left to core-4"] += notes
        found = works_in(bib, out, body)
        targets = [e for e in index.get((s["page"], s["lang"]), []) if s["ety"] in (None, e[0])]
        counts[f"{kind} sections"] += 1
        for _, _, how in found:
            counts[f"{kind}: cited by {how}"] += 1
        found = [f for f in found if f[0]]
        if found:
            counts[f"{kind} sections citing a work"] += 1
            if not targets:
                counts[f"{kind} sections citing a work, with no wiktextract entry"] += 1
        for ety, code, word, has in targets:
            key = (s["page"], s["lang"], ety)
            if kind == "references":
                with_section[key] = has
                for wid, loc, _ in found:
                    credits[key].setdefault((wid, loc), (code, word, has))
            else:
                further[key] = (code, further[key][1] | {wid for wid, _, _ in found})
    counts["entries with a References section"] = len(with_section)
    counts["entries with an etymology and a References section"] = sum(with_section.values())
    counts["objects"] = out.write(out_dir / "assertions.jsonl")
    with gzip.open(out_dir / "entry_credits.jsonl.gz", "wt", encoding="utf-8") as f:
        for key, cs in credits.items():
            code, word, has = next(iter(cs.values()))
            counts["entries credited"] += 1
            counts["entries with an etymology credited"] += has
            row = entry_key(key, code) | {
                "record": record_id(code, word, key[2]),
                "etymology": has,
                "credits": [
                    {"work": wid, "role": "follows", "scope": "entry"}
                    | ({"locator": loc} if loc else {})
                    for wid, loc in cs
                ],
            }
            counts["credits"] += len(row["credits"])
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    with gzip.open(out_dir / "further_reading.jsonl.gz", "wt", encoding="utf-8") as f:
        for key, (code, ws) in further.items():
            row = entry_key(key, code) | {"works": sorted(ws)}
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    counts["entries with Further reading works"] = len(further)
    return {
        "counts": dict(sorted(counts.items())),
        "bibliography": dict(sorted(bib.counts.items())),
        "errors": check(out_dir),
    }


def check(out_dir):
    """The bibliography's errors (`etymology.assertions`), then the credit table's: each
    credit is a schema `Credit` naming a work of the bibliography, each record id a
    schema `RecordId`. Until core-4 emits Wiktionary's assertions, the credits wait in
    this table; core-4 copies an entry's credits onto each assertion it endorses."""
    objs = A.read(out_dir / "assertions.jsonl")
    errors = A.validate(objs)
    works = {o["id"] for o in objs if o["type"] == "work"}
    credit = A.branch_validator("Credit")
    record = A.branch_validator("RecordId")
    with gzip.open(out_dir / "entry_credits.jsonl.gz", "rt", encoding="utf-8") as f:
        for n, line in enumerate(f, 1):
            d = json.loads(line)
            if not record.is_valid(d["record"]):
                errors.append(f"entry_credits line {n}: record id {d['record']!r}")
            for c in d["credits"]:
                errors += [f"entry_credits line {n}: {e.message}" for e in credit.iter_errors(c)]
                if c["work"] not in works:
                    errors.append(f"entry_credits line {n}: {c['work']} is not in the bibliography")
    return errors


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="step", required=True)
    e = sub.add_parser("extract", help="the dump's References sections and R: templates")
    e.add_argument("dump", type=Path, help="pages-articles XML (.bz2 or plain)")
    e.add_argument("--out", type=Path, required=True, help="sections, .jsonl.gz")
    e.add_argument("--counts", type=Path, help="where to write what was read")
    n = sub.add_parser("entries", help="wiktextract's entry keys")
    n.add_argument("wiktextract", type=Path, help="raw wiktextract data, .jsonl.gz")
    n.add_argument("--out", type=Path, required=True, help="entries, .jsonl.gz")
    c = sub.add_parser("convert", help="the bibliography and the entry-scope credits")
    c.add_argument("sections", type=Path, help="extract's output")
    c.add_argument("entries", type=Path, help="entries' output")
    c.add_argument("--out", type=Path, required=True, help="directory to write into")
    args = p.parse_args(argv)
    if args.step == "convert":
        result = convert(args.sections, args.entries, args.out)
        (args.out / "convert_counts.json").write_text(json.dumps(result, indent=1) + "\n")
        print(json.dumps(result, indent=1))
    elif args.step == "entries":
        print(json.dumps(entries(args.wiktextract, args.out)))
    else:
        result = extract(args.dump, args.out)
        text = json.dumps(result, ensure_ascii=False, indent=1)
        if args.counts:
            args.counts.write_text(text + "\n")
        print(text)


if __name__ == "__main__":
    main()
