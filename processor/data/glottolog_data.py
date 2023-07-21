# NOT USED CURRENTLY. This script gets language timespan data from Glottolog.
# The timespans included in Glottolog are only for extict languages. I have
# hand-compiled timespans for the primary IE languages elsewhere. In the future
# I may combine the two if there turns out to be a convincing need/want for
# timespan data in wety. If so, need to remember that wikidata id is not unique:
# multiple wiktionary language entries in languages.json can be associated with
# the same wikidata id. So need to do something smarter when matching languages
# between wiktionary and glottolog.

glottolog_path = "../glottolog"

import json
import pyglottolog

glottolog = pyglottolog.Glottolog(glottolog_path)


def get_wikidata_item(lang):
    prefix = "https://www.wikidata.org/entity/"
    for link in lang.links:
        link_str = link.to_string()
        if link_str.startswith(prefix):
            return link_str[len(prefix) :]


def get_timespan(lang):
    if not lang.timespan:
        return None
    return {
        "start": lang.timespan[0],
        "end": lang.timespan[1],
    }


def get_status(lang):
    if not lang.endangerment:
        return None
    return lang.endangerment.status.name


langs = []
for lang in glottolog.languoids():
    timespan = get_timespan(lang)
    if timespan:
        status = get_status(lang)
        if status != "extinct":
            print(lang.name)
        langs.append(
            {
                "name": lang.name,
                "wikidataItem": get_wikidata_item(lang),
                "timespan": timespan,
                "status": status,
            }
        )

json.dump(
    langs, open("glottolog.json", "w"), indent=2, ensure_ascii=False, sort_keys=True
)
