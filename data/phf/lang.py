# This script reads in several sources of language data and uses it to generate
# phf maps in a rust file.
#
# First, we read the csvs for wiktionary's list of languages and etymology-only
# languages (as they were on 2022-08-24):
# https://en.wiktionary.org/wiki/Wiktionary:List_of_languages,_csv_format
# https://en.wiktionary.org/wiki/Wiktionary:Etymology-only_languages,_csv_format
#
# Then, we get the Lua table from
# https://en.wiktionary.org/wiki/Module:etymology_languages/data. This is
# specifically to get the "parent" data for etymology-only languages, as this is
# generally missing from the csv linked above. This is important for being able
# to resolve templates in etymology sections to appropriate pages rather than
# missing ones, e.g. redirecting from "LL." (Late Latin) to "la" (Latin).
#
# Finally, we read the data from this wikitextprocessor file:
# https://github.com/tatuylonen/wikitextprocessor/blob/main/wikitextprocessor/languages.py,
# which is generated by
# https://github.com/tatuylonen/wikitextprocessor/blob/main/extract_language_codes.py,
# which was last run 2021-09-08. We have split up the resulting data locally
# here into wiktextract_languages.json and wiktextract_language_families.json.
# See extract_language_codes for details on how the data is generated.
# Basically, it fetches it all from Wiktionary:List_of_languages and
# Wiktionary:List_of_families. Therefore, this data will be almost entirely
# redundant with what we already got, but we have to favor wiktextract when
# there are discrepancies (see below). 
#
# It then generate a rust file which defines phf ordered maps (cf.
# https://docs.rs/phf/latest/phf/macro.phf_ordered_map.html) between lang codes
# and names, and a map from etymology-only lang codes to their corresponding
# actual lang codes. The file is saved as src/lang_phf.rs.
#
# This should be called from the base directory of the repo. Usage:
# 
# $ python data/phf/lang.py

#######################

import pandas as pd

etymology_only_languages = pd.read_csv("data/phf/etymology_only_languages.csv", delimiter=";", keep_default_na=False)
# Pandas interprets the lang code 'nan' for Min Nan language as NaN if we don't prevent it!
list_of_languages = pd.read_csv("data/phf/list_of_languages.csv", delimiter=";", keep_default_na=False)

code2name = {}
name2code = {}

reconstructed = set()

def each(string_list_of_items):
    for item in filter(lambda x: x, map(str.strip, string_list_of_items.split(","))):
        yield item

def add(lang):
    # There are multiple codes for various etymology-only languages
    for code in each(lang["code"]):
        canonical_name = lang["canonical name"].strip()
        code2name[code] = canonical_name
        name2code[canonical_name] = code
        for other_name in each(lang["other names"]):
            name2code[other_name] = code
        if "type" in lang and lang["type"].strip() == "reconstructed":
            reconstructed.add(code)

# We add the data from the wiktionary csvs
for _, lang in etymology_only_languages.iterrows():
    add(lang)
for _, lang in list_of_languages.iterrows():
    # We add language family codes as well, as they are not infrequently seen in
    # ety sections. Usually they are in templates like {{der|enm|gmq}},
    # {{der|enm|gmq|-}}, {{m|enm|gmq}}, i.e. where there is no source term and
    # hence wety won't pick it up as a valid ety node. However, on the off
    # chance someone erroneously uses a family code and intends to link to a
    # term, this will allow us a chance to give a reasonable imputation. Also,
    # useful to have the data in case we decide placeholder nodes with no
    # definite source term are ever desired.
    family_code = lang["family code"].strip()
    family = lang["family"].strip()
    if family_code and family:
        code2name[family_code] = family
        name2code[family] = family_code
    add(lang)

#######################

import lupa
from lupa import LuaRuntime
lua = LuaRuntime(unpack_returned_tuples=True)

ety_langs = {}
with open('data/phf/etymology_languages_data.lua') as file:
    data = file.read()
    ety_langs_data = dict(lua.execute(data))
    for lang_code, lang_data in ety_langs_data.items():
        lang_data = dict(lang_data)
        ety_langs[lang_code] = {"name": lang_data["canonicalName"], "parent": lang_data["parent"]}
# For each ety lang, we iterate through successive parents until we find the
# first that is not itself an ety lang. We then define this ety lang's "lang" to
# be this, which will be the code of either a normal language (usually) or a
# language family (sometimes?).
for code, ety_lang in ety_langs.items():
    lang = ety_lang["parent"]
    while lang in ety_langs:
        # print(code, lang)
        lang = ety_langs[lang]["parent"]
    ety_lang["lang"] = lang
etycode2code = {code: ety_lang["lang"] for code, ety_lang in ety_langs.items()}

#######################

# We write the wiktextract lang data last so that we will favor wiktextract data
# over the wiktionary csv data when there are any conflicts. This generally
# happens when the wiktextract data is out of date due to the data collecting
# script not having been run in a while. For example, in the wiktextract data
# from 2021-09-08, Old Khmer has lang_code 'mkh-okm', whereas in the wiktionary
# csv of 2022-08-25, it has lang_code 'okz' (See
# https://iso639-3.sil.org/code/okz, where we can see that the ISO 639-3 code
# was registered 2021-01-15. So wiktionary previously had a custom code, and
# updated it some months after the ISO code became available.) While we would
# like to have the latest data, since we are digesting wiktextract data, we need
# to align with their lang codes when there is mismatch, even though they are
# outdated.

import json

wiktextract_languages = json.load(open("data/phf/wiktextract_languages.json"))
wiktextract_language_families = json.load(open("data/phf/wiktextract_language_families.json"))

for lang in wiktextract_languages:
    for other_name in lang.get("other_names", ()):
            name2code[other_name] = lang["code"]
    for alias in lang.get("aliases", ()):
            name2code[alias] = lang["code"]
# Ensure that the wiktextract canonical name <-> code mapping is the last written
for lang_fam in wiktextract_language_families:
    code2name[lang_fam["code"]] = lang_fam["name"]
    # We don't write name -> code for families here as there may be some
    # conflicting overlap with language names/aliases/other_names. We only care
    # about writing family data so that we have their codes in case we want to
    # impute an item from a template in an ety section and the template
    # erroneously has a family code.
for lang in wiktextract_languages:
    code2name[lang["code"]] = lang["name"]
    name2code[lang["name"]] = lang["code"]

#######################

with open("src/lang_phf.rs", "w") as f:
    f.write("// This file was generated by data/phf/lang.py, see there for details.\n\n")
    f.write("#![allow(clippy::all)]\n\n")
    f.write("use phf::{phf_ordered_map, OrderedMap, phf_set, Set};\n\n")

    f.write("pub(crate) static LANG_CODE2NAME: OrderedMap<&'static str, &'static str> = phf_ordered_map! {\n")
    for code, name in code2name.items():
        f.write(f'    "{code}" => "{name}",\n')
    f.write("};\n\n")

    f.write("pub(crate) static LANG_NAME2CODE: OrderedMap<&'static str, &'static str> = phf_ordered_map! {\n")
    for name, code in name2code.items():
        f.write(f'    "{name}" => "{code}",\n')
    f.write("};\n\n")

    f.write("pub(crate) static LANG_ETYCODE2CODE: OrderedMap<&'static str, &'static str> = phf_ordered_map! {\n")
    for etycode, code in etycode2code.items():
        f.write(f'    "{etycode}" => "{code}",\n')
    f.write("};\n\n")

    f.write("pub(crate) static LANG_RECONSTRUCTED: Set<&'static str> = phf_set! {\n")
    for code in reconstructed:
        f.write(f'    "{code}",\n')
    f.write("};\n")
