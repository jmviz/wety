"""Audit this relocation against its pre-migration file inventory and scientific products."""
import json
from pathlib import Path
from .artifacts import atomic_json,sha
from .paths import DATA,REPO

LEDGER=DATA/'registry/migrations/2026-09-reorganization'

def normalize(value,key=None):
    # Only provenance-address fields changed, not linguistic links or temporal values.
    if isinstance(value,dict):
        return {k:normalize(v,k) for k,v in value.items() if k not in ('source_snapshot','crossref_generation','link_id','link_ids') and not (k=='id' and isinstance(v,str) and v.startswith('link:'))}
    if isinstance(value,list):
        items=[normalize(x) for x in value]
        return sorted(items,key=lambda x:json.dumps(x,sort_keys=True)) if key=='links' else items
    return value

def semantic_audit():
    from catalogs.registry import current_bundle
    from harness.registry import current_bundle as dating_bundle
    previous=DATA/'archive/reorganization-originals'
    cp=json.loads((previous/'core/data/langcodes/CURRENT.json').read_text())
    dp=json.loads((previous/'dating/data/_derived/CURRENT.json').read_text())
    catalog_old=json.loads((DATA/'artifacts/catalog-generations'/cp['generation']/'bundle.json').read_text())
    dating_old=json.loads((DATA/'artifacts/dating-generations'/dp['generation']/'bundle.json').read_text())
    catalog_new=current_bundle();dating_new=dating_bundle()
    checks={}
    for k in ('languages','families','glottolog','index_by_target','review_queue'):
        # Reverse-index ordering also follows content-derived link IDs.
        a,b=normalize(catalog_old[k]),normalize(catalog_new[k])
        if k=='index_by_target':
            a={k:sorted(v,key=lambda x:json.dumps(x,sort_keys=True)) for k,v in a.items()}
            b={k:sorted(v,key=lambda x:json.dumps(x,sort_keys=True)) for k,v in b.items()}
        checks['catalog_'+k]=a==b
    for k in ('claims','records','research_graph','review_queue','pilot_coverage'):
        checks['dating_'+k]=dating_old[k]==dating_new[k]
    checks['dating_view']=normalize(dating_old['view'])==normalize(dating_new['view'])
    checks['all_spoken_unknown']=all(v['spoken'] is None for v in dating_new['view'].values())
    return {'checks':checks,'passed':all(checks.values()),'counts':{'catalog_languages':len(catalog_new['languages']),'catalog_families':len(catalog_new['families']),'dating_claims':len(dating_new['claims']),'dating_codes':len(dating_new['view'])},'provenance_only_changes':['source_snapshot','crossref_generation','link_id(s)','content-derived link id/order'],'catalog_before':cp['generation'],'dating_before':dp['generation']}

def preservation_audit():
    old=json.loads((LEDGER/'inventory-before.json').read_text())['files']
    mapping=json.loads((LEDGER/'relocations.json').read_text())['files']
    cp=json.loads((DATA/'archive/reorganization-originals/core/data/langcodes/CURRENT.json').read_text())
    unchanged=[];preserved=[];changed=[];missing=[]
    for name,record in old.items():
        current=REPO/mapping[name]
        if not current.is_file():missing.append(name);continue
        if sha(current)==record['sha256']:unchanged.append(name);continue
        candidates=[DATA/'archive/reorganization-originals'/name]
        if name.startswith('core/data/langcodes/'):
            candidates.append(DATA/'artifacts/catalog-generations'/cp['generation']/Path(name).name)
        matched=next((p for p in candidates if p.is_file() and sha(p)==record['sha256']),None)
        if matched:preserved.append({'original':name,'preserved_at':str(matched.relative_to(REPO))})
        else:changed.append(name)
    return {'inventoried':len(old),'bytes':sum(v['bytes'] for v in old.values()),'unchanged':len(unchanged),'original_preserved':preserved,'edited_without_separate_original':changed,'missing':missing}

def main():
    result={'preservation':preservation_audit(),'semantics':semantic_audit()}
    atomic_json(LEDGER/'verification.json',result)
    print(json.dumps(result,indent=2))
    raise SystemExit(bool(result['preservation']['missing']) or not result['semantics']['passed'])

if __name__=='__main__':main()
