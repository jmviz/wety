# To be run after fetch_wiktionary_lang_data.py

import json
import requests
import re

headers = {
    "Accept-Encoding": "gzip,deflate",
    "User-Agent": "https://github.com/jmviz/wety",
}

date_props = [
    "P585", # "point in time" https://www.wikidata.org/wiki/Property:P585
    "P571", # "inception" https://www.wikidata.org/wiki/Property:P571
    "P580", # "start time", https://www.wikidata.org/wiki/Property:P580
    "P1249", # "time of earliest written record" https://www.wikidata.org/wiki/Property:P1249
]

def get_year(claims, date_prop):
    year_pattern = re.compile(r'(?P<year>-?\d+)')
    try:
        time = claims[date_prop][0]["mainsnak"]["datavalue"]["value"]["time"]
        return int(re.match(year_pattern, time).group("year"))
    except: return None

def get_wikidata(id):
    wikidata = {
        "wikipedia_url": None,
        "wiktionary_url": None,
        "year": None,
    }
    r = requests.get(f"https://www.wikidata.org/wiki/Special:EntityData/Q{id}.json?flavor=simple", 
                         headers=headers)
    wikidata_json = r.json()
    # the general structure is "entities" -> "Qxxxxx", but we don't access the
    # "Qxxxxx" field directly in case there was a redirect to another id
    wikidata_json = next(iter(wikidata_json["entities"].values()))
    if "sitelinks" in wikidata_json:
        sitelinks = wikidata_json["sitelinks"]
        if "enwiktionary" in sitelinks:
            wikidata["wiktionary_url"] = sitelinks["enwiktionary"]["url"]
        if "enwiki" in sitelinks:
            wikidata["wikipedia_url"] = sitelinks["enwiki"]["url"]
    if "claims" in wikidata_json:
        claims = wikidata_json["claims"]
        for date_prop in date_props:
            year = get_year(claims, date_prop)
            if year:
                wikidata["year"] = year
                return wikidata
    return wikidata