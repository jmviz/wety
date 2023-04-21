import json

with open('data/phf/wiktextract_language_families.json', 'r') as f:
    wiktextract_data = json.load(f)
    wiktextract_families = {}
    for w_family in wiktextract_data:
        wiktextract_families[w_family['code']] = {
            'name': w_family['name'],
            'parent_code': w_family['parent_code'],
            'url': w_family['url'],
        }

def get_ancestors(family):
    ancestors = []
    parent_code = family['parent_code']
    while parent_code:
        # qfa-not is its own parent
        # see https://en.wiktionary.org/wiki/Wiktionary:Language_families#Non-genetic_categories_and_isolates
        if parent_code in ancestors:
            print(w_family['name'], ancestors, parent_code)
            break
        ancestors.append(parent_code)
        parent = wiktextract_families[parent_code]
        parent_code = parent['parent_code']
    return ancestors

families = []
for code, w_family in wiktextract_families.items():
    family = {
        code: {
            'name': w_family['name'],
            'ancestors': get_ancestors(w_family),
            'url': w_family['url'],
        }
    }
    families.append(family)

with open('data/phf/lang_families.json', 'w') as f:
    json.dump(families, f, indent=4, ensure_ascii=False)
