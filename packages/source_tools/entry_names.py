"""Export Wiktionary entry names (page titles) for the terms cited in a Wiktextract dump.

Run from any directory:
    python packages/source_tools/entry_names.py DUMP WIKTEXTRACT LANGUAGES

Etymology templates and Descendants sections cite terms in their display form
(e.g. Latin "mātrīx"), while the page, and hence the Wiktextract entry, is titled by
the language's entry name ("matrix"). Wiktionary computes it with
Language:stripDiacritics, whose rules are per language and script, partly Lua
patterns and partly whole modules. Rather than reimplement them, this runs them from
the dump's own modules over every (language, term) pair the dump cites, cleaned as
the processor cleans template terms, and records those whose title differs.
"""
from __future__ import annotations
import argparse
import gzip
import importlib.metadata
import json
from pathlib import Path
import re
import sys
from workflow_tools.artifacts import now
from workflow_tools.paths import REPO as ROOT
from source_tools.language_data import atomic_json, sha

CHUNK = 100_000  # pairs per #invoke; one invoke may run at most 60 s (~7k pairs/s)
LANG_PREFIX = re.compile(r'^([a-zA-Z][a-zA-Z0-9._-]*):(.*)$', re.S)
ROOT_CATEGORY = re.compile(r'^(.+) terms derived from the (.+) root \*([^ ]+)(?: \((.+)\))?$')


def clean(term):
    """crates/processor clean_template_term: the term a template arg names."""
    if term.startswith('[['):
        term = re.split(r'[|\]]', term[2:], maxsplit=1)[0]
    term = term.removeprefix('*')
    for sep in ('<', '#', ' ('):
        term = term.split(sep, 1)[0]
    return term


def modifiers(arg):
    """The contents of the top-level <...> groups after the term."""
    out, depth, start = [], 0, 0
    for i, c in enumerate(arg):
        if c == '<':
            if depth == 0:
                start = i
            depth += 1
        elif c == '>' and depth:
            depth -= 1
            if depth == 0:
                out.append(arg[start + 1:i])
    return out


def term_pairs(arg, lang, codes):
    """(lang, term) pairs that a term arg may name: as is, with a lang: prefix, a
    <lang:> modifier, and trimmed (as {{etymon}} etymons are). Over-inclusive."""
    head = arg.split('<', 1)[0]
    langs = [lang]
    if m := LANG_PREFIX.match(head):
        if m.group(1) in codes:
            langs.append(m.group(1))
            head = m.group(2)
    for mod in modifiers(arg):
        name, _, value = mod.partition(':')
        if name == 'lang' and value in codes:
            langs.append(value)
        elif name == 'ety':
            yield from inline_ety_pairs(value, langs[-1], codes)
    for term in {clean(head), clean(head.strip())}:
        if term:
            for code in langs:
                yield code, term


def inline_ety_pairs(ety, lang, codes):
    """Etymons in an {{etymon}} inline etymology, keyword<etymon><etymon>..."""
    depth, start = 0, 0
    for i, c in enumerate(ety):
        if c == '<':
            if depth == 0:
                start = i
            depth += 1
        elif c == '>' and depth:
            depth -= 1
            if depth == 0 and not ety[start + 1:].startswith('<'):
                yield from term_pairs(ety[start + 1:i], lang, codes)


def template_pairs(template, codes):
    args = template.get('args') or {}
    lang = args.get('1')
    positional = [args[k] for k in sorted((k for k in args if k.isdigit()), key=int)]
    if template.get('name') in ('etymon', 'ety'):
        for arg in positional[1:]:
            if not arg.startswith(':'):
                yield from term_pairs(arg, lang, codes)
        return
    # The source language and term of derived-kind templates ({{bor|en|fr|mot}}),
    # {{root}} and {{word}} (whose code drops "-pro").
    source = (args.get('2') or '').split(',')[0]
    for code in (source, source + '-pro'):
        if code in codes and args.get('3'):
            yield from term_pairs(args['3'], code, codes)
    # Terms in the entry's language: abbreviation-, compound- and mention-kind
    # templates, with per-part langN= (counted from 1).
    if lang in codes:
        for n, arg in enumerate(positional[1:], start=1):
            yield from term_pairs(arg, args.get(f'lang{n}') if args.get(f'lang{n}') in codes else lang, codes)
    if template.get('name') == 'PIE word' and args.get('2'):
        yield from term_pairs(args['2'], 'ine-pro', codes)


def descendant_pairs(nodes, codes):
    for node in nodes or ():
        if node.get('lang_code') in codes and node.get('word'):
            yield node['lang_code'], clean(node['word'])
        yield from descendant_pairs(node.get('descendants'), codes)


def cited_pairs(wiktextract, codes, names):
    pairs = set()
    with gzip.open(wiktextract, 'rt', encoding='utf-8') as lines:
        for line in lines:
            entry = json.loads(line)
            if 'redirect' in entry:
                continue
            for template in entry.get('etymology_templates') or ():
                pairs.update(template_pairs(template, codes))
            pairs.update(descendant_pairs(entry.get('descendants'), codes))
            for category in entry.get('categories') or ():
                if (m := ROOT_CATEGORY.match(category)) and m.group(2) in names:
                    pairs.add((names[m.group(2)], m.group(3)))
    return sorted(p for p in pairs if p[1] and p[1] != '-')


def lua_string(text):
    level = 0
    while f']{"=" * level}]' in text:
        level += 1
    return f'[{"=" * level}[{text}]{"=" * level}]'


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('dump', type=Path, help='enwiktionary pages-articles .xml.bz2')
    parser.add_argument('wiktextract', type=Path, help='Wiktextract raw data .jsonl.gz from the same dump')
    parser.add_argument('languages', type=Path, help='languages.json exported from the same dump')
    parser.add_argument('--output', type=Path, default=ROOT/'data/workspaces/entry-names/entry_names.json')
    parser.add_argument('--receipt', type=Path, default=ROOT/'data/workspaces/entry-names/entry_names_receipt.json')
    parser.add_argument('--dump-url', help='Original dump download URL, when known')
    args = parser.parse_args()
    started_at = now()
    languages = json.loads(args.languages.read_text())
    codes = set(languages)
    names = {v['canonicalName']: k for k, v in languages.items() if v.get('kind') != 'etymology-only'}
    pairs = cited_pairs(args.wiktextract, codes, names)
    print(f'{len(pairs)} cited (language, term) pairs', flush=True)

    from wikitextprocessor import Wtp
    from wiktextract.config import WiktionaryConfig
    from wiktextract.wxr_context import WiktextractContext
    from wikitextprocessor.dumpparser import process_dump
    wxr = WiktextractContext(Wtp(), WiktionaryConfig())
    lua = Path(__file__).with_suffix('.lua')
    titles, failed = {}, 0
    try:
        ns = wxr.wtp.NAMESPACE_DATA['Module']
        process_dump(wxr.wtp, str(args.dump), {ns['id']})
        wxr.wtp.add_page(ns['name']+':wety-entry-names', ns['id'], body=lua.read_text(), model='Scribunto')
        chunks = [pairs[i:i + CHUNK] for i in range(0, len(pairs), CHUNK)]
        for n, chunk in enumerate(chunks):
            body = 'return ' + lua_string(json.dumps(chunk, ensure_ascii=False))
            wxr.wtp.add_page(f"{ns['name']}:wety-entry-names/input/{n}", ns['id'], body=body, model='Scribunto')
        wxr.wtp.db_conn.commit()
        for n in range(len(chunks)):
            wxr.wtp.start_page(f'entry names {n}')
            text = wxr.wtp.expand('{{#invoke:wety-entry-names|entry_names|'+str(n)+'}}')
            try:
                result = json.loads(text)
            except ValueError:
                raise ValueError(f'entry names failed for chunk {n}: {text[:300]}')
            failed += result['failed']
            for code, term, title in result['titles'] or ():
                titles.setdefault(code, {})[term] = title
            print(f'chunk {n + 1}/{len(chunks)}', flush=True)
    finally:
        wxr.wtp.close_db_conn()
    receipt = {'schema_version': 1, 'started_at': started_at, 'completed_at': now(), 'python': sys.version,
               'dependency_lock_sha256': sha(ROOT/'uv.lock'),
               'dump': {'name': args.dump.name, 'url': args.dump_url, 'sha256': sha(args.dump)},
               'wiktextract': {'name': args.wiktextract.name, 'sha256': sha(args.wiktextract)},
               'languages': {'name': args.languages.name, 'sha256': sha(args.languages)},
               'lua_sha256': sha(lua), 'exporter_sha256': sha(__file__),
               'dependencies': {n: {'version': importlib.metadata.version(n), 'origin': json.loads(importlib.metadata.distribution(n).read_text('direct_url.json') or 'null')} for n in ('wikitextprocessor', 'wiktextract')},
               'counts': {'cited_pairs': len(pairs), 'differing_titles': sum(map(len, titles.values())), 'failed': failed},
               'limitations': ['Cited pairs are collected over-inclusively from the Wiktextract dump; a term the processor reads differently has no entry here and is matched as written.',
                               'Dump-level provenance; individual upstream module revisions must be recovered from the dump if needed.']}
    atomic_json(args.output, titles)
    receipt['outputs'] = {args.output.name: sha(args.output)}
    atomic_json(args.receipt, receipt)


if __name__ == '__main__':
    main()
