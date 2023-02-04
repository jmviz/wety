import json

with (
    open("data/raw-wiktextract-data-descendants.json", "r") as in_file,
    open("data/raw-wiktextract-data-descendants-only.json", "w") as desc_file,
    open("data/descendants_langs.csv", "w") as hist_file
):
    langs = {}
    for line in in_file:
        entry = json.loads(line)
        if "descendants" in entry:
            lang = entry["lang"]
            if lang in langs:
                langs[lang] += 1
            else:
                langs[lang] = 1
            desc_file.write(line)
    
    for lang in sorted(langs, key=langs.get, reverse=True):
        hist_file.write("{}, {}\n".format(lang, langs[lang]))