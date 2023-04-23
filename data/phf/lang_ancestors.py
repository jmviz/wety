import re

# Parse a Module:languages data file. N.B. a language's "ancestors" in these
# files should really be called the parent(s), as it only traces back one step.
# For example, the "ancestors" of "en" is "enm". Most languages have either zero
# or one "ancestors". The only languages that have multiple are mixture
# languages where multiple parent languages went into forming it.
def parse_languages_file(filename):
    langs = {}

    entry_pattern = re.compile(r'm\["(?P<code>[^"\n]+)"\] = {.*?(?P<content>\n.*?)}\n\n', re.DOTALL)
    name_family_pattern = re.compile(r'^\n\t"(?P<name>[^"\n]+)",.*?\n\t.*?\n\t("(?P<family>[a-z\-]+)",)?', re.DOTALL)
    ancestors_pattern = re.compile(r'\n\tancestors = {?(?P<ancestors>.+?)(}|--|\n)')

    with open(filename, 'r') as f:
        file_content = f.read()
        for match in entry_pattern.finditer(file_content):
            code = match.group('code')
            content = match.group('content')

            info_match = name_family_pattern.match(content)
            if info_match:
                name = info_match.group('name')
                family = info_match.group('family')
                ancestors = []
                ancestors_match = ancestors_pattern.search(content)
                if ancestors_match:
                    ancestors = ancestors_match.group('ancestors')
                    if ancestors:
                        ancestors = [a.strip('" ') for a in ancestors.split(',')]
                        ancestors = [a for a in ancestors if a]
                # We don't bother dealing with mixture languages for
                # now. There are only 19 of these among all languages in
                # Module:languages data (as of 2023-04-22), and all of
                # them are relatively minor AFAIK. Setting this to []
                # here will cause the family-based ancestry tracing
                # below to occur for mixture languages, but this will
                # generally be of no consequence as the family is either
                # "crp" or "qfa-mix" for all of them except one, both of
                # which have parent family "qfa-not". The only exception
                #  currently is Norwegian Bokmål, which has family gmq
                # (both its parents are of this family).
                parent = ancestors[0] if len(ancestors) == 1 else None

                langs[code] = {
                    'name': name,
                    'family': family,
                    'parent': parent,
                    'etymology_only': False,
                }
    return langs

import os

# process the Module:languages files
langs = {}
path = 'data/wiktionary-modules/Module/languages/data/'
langs.update(parse_languages_file(os.path.join(path, '2.txt')))

langs.update(parse_languages_file(os.path.join(path, 'exceptional.txt')))

data3_pattern = re.compile(r'[a-z]\.txt')
for filename in os.listdir(os.path.join(path, '3')):
    if data3_pattern.match(filename):
        langs.update(parse_languages_file(os.path.join(path, '3', filename)))

import json

with open('data/phf/etymology_langs.json') as elf:
    el = json.load(elf)
    for code, data in el.items():
        ancestor = data['parent']
        while ancestor in el:
            ancestor = el[ancestor]['parent']
        family = langs[ancestor]['family'] if ancestor in langs else None
        parent = data['parent']
        langs[code] = {
            'name': data['name'],
            'family': family,
            'parent': parent if parent in el or parent in langs else None,
            'etymology_only': True,
        }

def trace_ancestry(langs, fams, lang):
    parent = langs[lang]['parent']
    ancestors = []
    while parent:
        ancestors.append(parent)
        parent = langs[parent]['parent'] if parent in langs else None
        parent = parent if parent in langs else None
    fam = langs[ancestors[-1]]['family'] if ancestors else langs[lang]['family']
    if not fam or fam not in fams: return ancestors
    proto = fam + '-pro'
    if proto in langs:
        ancestors.append(proto)
    for fam_ancestor in fams[fam]['ancestors']:
        proto = fam_ancestor + '-pro'
        if proto in langs:
            ancestors.append(proto)
        else: return ancestors
    return ancestors


# combine the data 
with open('data/phf/wiktextract_languages.json', 'r') as wl, open('data/phf/lang_families.json', 'r') as wf:
    w_langs = {lang['code']: lang for lang in json.load(wl)}
    fams = json.load(wf)
    for lang in langs:
        langs[lang]['url'] = w_langs[lang]['language_url'] if lang in w_langs and 'language_url' in w_langs[lang] else None
        langs[lang]['ancestors'] = trace_ancestry(langs, fams, lang)


with open('data/phf/lang_ancestors.json', 'w') as f:
    json.dump(langs, f, indent=4, ensure_ascii=False)
