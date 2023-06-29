# Export English Wiktionary language and family data to JSON.
#
# This should be run from the root directory of the repo.
#
# Usage:
#
# python processor/data/language_data.py enwiktionary_dump_file [--languages languages_output_file] [--families families_output_file]

import argparse
from wikitextprocessor import Wtp
from wiktextract.config import WiktionaryConfig
from wiktextract.wxr_context import WiktextractContext
from wikitextprocessor.dumpparser import process_dump
import json


def export_data(wxr: WiktextractContext, kind: str, path: str) -> None:
    wxr.wtp.start_page(f"{kind} data export")
    data = wxr.wtp.expand(f"{{{{#invoke:lang-data-export|{kind}}}}}")
    data = json.loads(data)
    with open(path, "w", encoding="utf-8") as fout:
        json.dump(data, fout, indent=2, ensure_ascii=False, sort_keys=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Export Wiktionary language and family data to JSON"
    )
    parser.add_argument("dump", type=str, help="Wiktionary xml dump file path")
    parser.add_argument(
        "--languages",
        type=str,
        default="languages.json",
        help="Language data output file path",
    )
    parser.add_argument(
        "--families",
        type=str,
        default="families.json",
        help="Family data output file path",
    )
    args = parser.parse_args()
    wxr = WiktextractContext(Wtp(), WiktionaryConfig())
    module_ns_id = wxr.wtp.NAMESPACE_DATA["Module"]["id"]
    module_ns_name = wxr.wtp.NAMESPACE_DATA["Module"]["name"]
    process_dump(wxr.wtp, args.dump, {module_ns_id})
    with open("processor/data/language_data.lua", "r") as fin:
        lua_mod = fin.read()
    wxr.wtp.add_page(
        f"{module_ns_name}:lang-data-export",
        module_ns_id,
        body=lua_mod,
        model="Scribunto",
    )
    wxr.wtp.db_conn.commit()
    export_data(wxr, "languages", args.languages)
    export_data(wxr, "families", args.families)
    wxr.wtp.close_db_conn()
