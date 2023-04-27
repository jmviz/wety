# Fetch Json exports of Module:languages data. See
# https://en.wiktionary.org/wiki/Module:JSON_data. The lang exports are missing
# the otherNames, aliases, varieties, which we get from parsing the dumped
# Module:languages extra data directly, see get_wiktionary_extra_lang_data.py.
# Also, we want urls, which we get in fetch_wikidata_lang_data.py. These three
# scripts are meant to be run in the sequence they were just mentioned, as in
# get_all_lang_data.py.

import json
import requests

headers = {
    "Accept-Encoding": "gzip,deflate",
    "User-Agent": "https://github.com/jmviz/wety",
}

# https://github.com/tatuylonen/wiktextract/blob/master/get_languages.py
def expand_template(text: str) -> str:
    # https://www.mediawiki.org/wiki/API:Expandtemplates
    params = {
        "action": "expandtemplates",
        "format": "json",
        "text": text,
        "prop": "wikitext",
        "formatversion": "2",
        "maxlag": 5,
    }
    r = requests.get("https://en.wiktionary.org/w/api.php",
                     params=params, headers=headers)
    data = r.json()
    return data["expandtemplates"]["wikitext"]

lang_data_json = expand_template(
        "{{#invoke:JSON data|export_languages||1|2|3|type|ancestors|wikipedia_article}}"
    )
lang_data = json.loads(lang_data_json)
with open("data/phf/lang_data.json", "w") as fout:
    json.dump(lang_data, fout, indent=4, ensure_ascii=False)

ety_lang_data_json = expand_template(
        "{{#invoke:JSON data|export_etymology_languages}}"
    )
ety_lang_data = json.loads(ety_lang_data_json)
with open("data/phf/ety_lang_data.json", "w") as fout:
    json.dump(ety_lang_data, fout, indent=4, ensure_ascii=False)

lang_family_data_json = expand_template(
        "{{#invoke:JSON data|export_families}}"
    )
lang_family_data = json.loads(lang_family_data_json)
with open("data/phf/lang_family_data.json", "w") as fout:
    json.dump(lang_family_data, fout, indent=4, ensure_ascii=False)

