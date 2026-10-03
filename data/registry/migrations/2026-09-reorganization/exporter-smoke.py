"""Optional-extraction environment smoke; does not validate real Wiktionary modules."""
from pathlib import Path
import tempfile
from workflow_tools.paths import REPO
from wikitextprocessor import Wtp
from wiktextract.config import WiktionaryConfig
from wiktextract.wxr_context import WiktextractContext
from source_tools.language_data import extract

with tempfile.TemporaryDirectory(dir=REPO/'data/cache') as directory:
    context=WiktextractContext(Wtp(db_path=Path(directory)/'fixture.db'),WiktionaryConfig())
    try:
        ns=context.wtp.NAMESPACE_DATA['Module']
        body='return {languages=function() return [[{"xx":{"mainCode":"xx"}}]] end, families=function() return [[{"xxf":{"mainCode":"xxf"}}]] end}'
        context.wtp.add_page(ns['name']+':lang-data-export',ns['id'],body=body,model='Scribunto')
        assert extract(context,'languages')=={'xx':{'mainCode':'xx'}}
        assert extract(context,'families')=={'xxf':{'mainCode':'xxf'}}
    finally:context.wtp.close_db_conn()
print('PASS: pinned extraction imports, Lua invocation, and JSON decoding')
