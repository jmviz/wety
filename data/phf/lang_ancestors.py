import re

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

                langs[code] = {
                    'name': name,
                    'family': family,
                    'ancestors': ancestors,
                }
    return langs

langs = {}

langs.update(parse_languages_file('data/wiktionary-modules/Module/languages/data/2.txt'))

langs.update(parse_languages_file('data/wiktionary-modules/Module/languages/data/exceptional.txt'))

import os
data3_pattern = re.compile(r'[a-z]\.txt')
for filename in os.listdir('data/wiktionary-modules/Module/languages/data/3'):
    if data3_pattern.match(filename):
        langs.update(parse_languages_file(os.path.join('data/wiktionary-modules/Module/languages/data/3', filename)))

import json
with open('data/phf/lang_ancestors.json', 'w') as f:
    json.dump(langs, f, indent=4, ensure_ascii=False)
