"""Export Wiktionary source concepts, with a verifiable dump/export receipt.

Run from any directory: python packages/source_tools/language_data.py DUMP
The receipt identifies the complete dump, Lua export code and dependencies.
A dump hash does not by itself supply missing upstream revision IDs.
"""
from __future__ import annotations
import argparse
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import tempfile
import sys
from workflow_tools.artifacts import now
from workflow_tools.paths import REPO as ROOT


def sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for b in iter(lambda:f.read(1<<20),b''):h.update(b)
    return h.hexdigest()


def atomic_json(path,data):
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    fd,tmp=tempfile.mkstemp(dir=path.parent)
    with os.fdopen(fd,'w') as f:json.dump(data,f,indent=2,ensure_ascii=False,sort_keys=True);f.write('\n');f.flush();os.fsync(f.fileno())
    os.replace(tmp,path)


def extract(wxr,kind):
    wxr.wtp.start_page(f'{kind} data export')
    return json.loads(wxr.wtp.expand('{{#invoke:lang-data-export|'+kind+'}}'))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('dump',type=Path)
    parser.add_argument('--languages',type=Path,default=ROOT/'data/workspaces/catalogs/wiktionary-export/languages.json')
    parser.add_argument('--families',type=Path,default=ROOT/'data/workspaces/catalogs/wiktionary-export/families.json')
    parser.add_argument('--receipt',type=Path,default=ROOT/'data/workspaces/catalogs/wiktionary-export/wiktionary_export_receipt.json')
    parser.add_argument('--dump-url',help='Original dump download URL, when known')
    args=parser.parse_args()
    started_at=now()
    from wikitextprocessor import Wtp
    from wiktextract.config import WiktionaryConfig
    from wiktextract.wxr_context import WiktextractContext
    from wikitextprocessor.dumpparser import process_dump
    wxr=WiktextractContext(Wtp(),WiktionaryConfig())
    lua=Path(__file__).with_suffix('.lua')
    try:
        ns=wxr.wtp.NAMESPACE_DATA['Module']
        process_dump(wxr.wtp,str(args.dump),{ns['id']})
        wxr.wtp.add_page(ns['name']+':lang-data-export',ns['id'],body=lua.read_text(),model='Scribunto')
        wxr.wtp.db_conn.commit()
        # Complete both extractions before replacing either output. Consumers
        # verify both hashes against the final receipt, committed last.
        langs=extract(wxr,'languages');families=extract(wxr,'families')
        receipt={'schema_version':1,'started_at':started_at,'completed_at':now(),'python':sys.version,'dependency_lock_sha256':sha(ROOT/'uv.lock'),'dump':{'name':args.dump.name,'url':args.dump_url,'sha256':sha(args.dump)},'lua_sha256':sha(lua),'exporter_sha256':sha(__file__),
                 'dependencies':{n:{'version':importlib.metadata.version(n),'origin':json.loads(importlib.metadata.distribution(n).read_text('direct_url.json') or 'null')} for n in ('wikitextprocessor','wiktextract')},'limitations':['Dump-level provenance; individual upstream module revisions must be recovered from the dump if needed.']}
        atomic_json(args.languages,langs);atomic_json(args.families,families)
        receipt['outputs']={p.name:sha(p) for p in (args.languages,args.families)}
        atomic_json(args.receipt,receipt)
    finally:wxr.wtp.close_db_conn()


if __name__=='__main__':main()
