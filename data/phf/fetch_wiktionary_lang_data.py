import json
import requests

# https://github.com/tatuylonen/wiktextract/blob/master/get_languages.py
def expand_template(text: str) -> str:
    # https://www.mediawiki.org/wiki/API:Expandtemplates
    params = {
        "action": "expandtemplates",
        "format": "json",
        "text": text,
        "prop": "wikitext",
        "formatversion": "2",
    }
    r = requests.get(f"https://en.wiktionary.org/w/api.php",
                     params=params)
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