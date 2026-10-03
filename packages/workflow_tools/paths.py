"""Repository-owned locations. Paths are portable; historical identifiers are retained."""
import json
from functools import lru_cache
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DATA = REPO / 'data'
CATALOG_CODE = REPO / 'packages/catalogs'
CATALOG_PRODUCT = DATA / 'products/catalogs'
CATALOG_WORKSPACE = DATA / 'workspaces/catalogs'
CATALOG_SOURCES = CATALOG_WORKSPACE / 'sources'
CATALOG_GENERATIONS = DATA / 'artifacts/catalog-generations'
DATING_WORKSPACE = DATA / 'workspaces/dating'
DATING_PRODUCT = DATA / 'products/dating'
DATING_GENERATIONS = DATA / 'artifacts/dating-generations'
DATING_HISTORY = DATA / 'archive/dating'
RETRIEVED = DATING_WORKSPACE / 'retrieved'
IECOR_PRODUCT = DATA / 'products/iecor'

@lru_cache(maxsize=1)
def selection():
    return json.loads((DATA/'registry/inputs.json').read_text())

def selected(name):
    """Resolve an explicit selection; never guess the latest directory."""
    return resolve(selection()[name])

def resolve(path):
    p=Path(path)
    if p.is_absolute():
        raise ValueError('repository references must be relative')
    p=(REPO/p).resolve()
    if not p.is_relative_to(REPO):raise ValueError('reference escapes repository')
    return p

@lru_cache(maxsize=1)
def relocations():
    return json.loads((DATA/'registry/migrations/2026-09-reorganization/relocations.json').read_text())['files']

def historical(path, base='dating'):
    """Resolve legacy evidence without rewriting its original payload or claim ID.

    Absolute references are accepted only when their exact recorded repository suffix
    occurs in the migration inventory. Unknown locations fail, never guess by basename.
    """
    raw=str(path)
    if Path(raw).is_absolute():
        if Path(raw).is_file():return Path(raw)  # explicit caller-supplied local input
        # Historical records used this checkout. Match an inventoried complete suffix.
        matches=[v for k,v in relocations().items() if raw.endswith('/'+k)]
        if len(matches)!=1:raise ValueError('unresolved historical location: '+raw)
        return resolve(matches[0])
    key=base.rstrip('/')+'/'+raw if base else raw
    if key in relocations():return resolve(relocations()[key])
    return resolve(raw if raw.startswith(('data/','packages/','crates/')) else 'packages/'+key)
