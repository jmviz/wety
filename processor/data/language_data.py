# Export English Wiktionary language and family data to JSON.
#
# This should be run from the root directory of the repo.
#
# Usage:
#
# python processor/data/language_data.py enwiktionary_dump_file [--languages languages_output_file] [--families families_output_file]

import argparse
from wikitextprocessor import Wtp
from wikitextprocessor.dumpparser import process_dump
import json

def export_data(ctx, kind, path):
    ctx.start_page(f"{kind} data export")
    data = ctx.expand(f"{{{{#invoke:lang-data-export|{kind}}}}}")

    data = json.loads(data)

    with open(path, "w") as fout:
        json.dump(data, fout, indent=2, ensure_ascii=False, sort_keys=True)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
    description="Export Wiktionary language and family data to JSON")
    parser.add_argument("dump", type=str,
                        help="Wiktionary xml dump file path")
    parser.add_argument("--languages", type=str, default="languages.json",
                            help="Language data output file path")
    parser.add_argument("--families", type=str, default="families.json",
                            help="Family data output file path")
    args = parser.parse_args()

    ctx = Wtp()

    def page_handler(model, title, text):
        if title.startswith("Module:"):
            ctx.add_page(model, title, text)

    process_dump(ctx, args.dump, page_handler=page_handler)

    with open("processor/data/language_data.lua", "r") as fin:
        lua_mod = fin.read()

    ctx.add_page("Scribunto", "Module:lang-data-export", lua_mod, transient=True)

    export_data(ctx, "languages", args.languages)
    export_data(ctx, "families", args.families)

