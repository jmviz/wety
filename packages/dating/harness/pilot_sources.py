"""Targeted repair of the primary sources missed in the pilots; no model calls."""
from workflow_tools.paths import DATING_PRODUCT, DATING_GENERATIONS, DATING_HISTORY

import json
from pathlib import Path
from .config import DATA_DIR
from .retrieve import fetch_text, TEXTS, RETRIEVAL_VERSION, source_id_for, cached_text
from .io_utils import atomic_json
from .store import upsert_source
from .schema import Source

TARGETS=[
 ('ural1272','10.1093/jole/lzz009',[
 'https://uu.diva-portal.org/smash/get/diva2%3A1477043/FULLTEXT01',
 'https://www.utupub.fi/server/api/core/bitstreams/e96250a0-7b60-4e44-aec3-a0bed433016c/content',
 'https://academic.oup.com/jole/article/5/1/17/5688948',
 'https://academic.oup.com/jole/article-pdf/5/1/17/32972859/lzz009.pdf']),
 ('ural1272','10.1111/jeb.12107',['https://bedlan.net/wp-content/uploads/2019/05/honkola_et_al_2013_cultural_and_climatic.pdf']),
 ('ural1272','10.1075/dia.20038.gru',['https://www.mv.helsinki.fi/home/matmies/publications/UralicSpread_Text%26Supplements_Accepted.pdf']),
 ('indo1319','10.1353/lan.2015.0005',['https://linguistics.berkeley.edu/~garrett/ChangEtAl-2015.pdf']),
 ('sino1245','10.1073/pnas.1817972116',['https://www.ebi.ac.uk/europepmc/webservices/rest/PMC6534992/fullTextXML']),
]


def repair():
    reindex_legacy_texts()
    results=[]
    for family,doi,urls in TARGETS:
        path=Path(DATA_DIR)/family/'candidates.json'; candidates=json.loads(path.read_text())
        c=next((c for c in candidates if c.get('doi')==doi),None)
        if not c: results.append({'doi':doi,'error':'candidate absent'}); continue
        old=c.get('retrieved',{})
        if old.get('version')==RETRIEVAL_VERSION and old.get('quality')=='fulltext_candidate' and cached_text(c) is not None:
            results.append({'doi':doi,'status':'already_repaired','sha256':old['sha256']}); continue
        row={'doi':doi,'attempts':[]}
        for url in urls:
            try:
                text,sha,kind=fetch_text(url)
                if not text: continue
                meta=json.loads((Path(TEXTS)/(sha+'.meta.json')).read_text())
                row['attempts'].append({'url':url,'quality':meta['quality'],'sha256':sha})
                if meta['quality']!='fulltext_candidate': continue
                # Identity confirmation, not mere response-size acceptance.
                words=[w.strip('.,:').casefold() for w in c['title'].split() if len(w)>5]
                if sum(w in text.casefold() for w in words)<min(3,len(words)): raise ValueError('document identity mismatch')
                c.setdefault('retrieval_history',[]).append(old)
                c['retrieved']={**meta,'sha256':sha,'kind':kind,'chars':len(text)}
                if c.get('extracted'): c.setdefault('extraction_history',[]).append(c.pop('extracted'))
                sid=c.get('source_id') or source_id_for(c); c['source_id']=sid
                upsert_source(Source(id=sid,type='paper',title=c['title'],doi=doi,year=c.get('year'),url=meta['url'],sha256=sha,raw_sha256=meta['raw_sha256'],retrieval_quality=meta['quality']))
                row['status']='repaired'; break
            except Exception as e:
                row['attempts'].append({'url':url,'error':str(e)[:240]})
        row.setdefault('status','unresolved')
        atomic_json(path,candidates); results.append(row)
        print(doi,row['status'],flush=True)
    atomic_json(Path(DATING_PRODUCT)/'pilot_source_repair.json',results)
    return results


def reindex_legacy_texts():
    """Repair raw-hash-named legacy text without inventing raw provenance."""
    import hashlib
    from .config import CACHE_DIR
    from .io_utils import atomic_text, digest
    from .store import load_sources, save_sources
    targets=json.loads((Path(DATA_DIR)/'_research/pilot_sources.json').read_text())
    sources=load_sources(); changed=[]
    for family in sorted({t['family'] for t in targets}):
        path=Path(DATA_DIR)/family/'candidates.json'; cs=json.loads(path.read_text())
        dois={t['doi'].lower() for t in targets if t['family']==family}
        for c in cs:
            if (c.get('doi') or '').lower() not in dois: continue
            old=c.get('retrieved') or {}; sha=old.get('sha256')
            if not sha or old.get('version')==RETRIEVAL_VERSION: continue
            src=Path(CACHE_DIR)/'texts'/(sha+'.txt')
            if not src.exists(): continue
            raw=src.read_bytes(); actual=hashlib.sha256(raw).hexdigest()
            if actual==sha: continue
            atomic_text(Path(CACHE_DIR)/'texts'/(actual+'.txt'),raw.decode('utf-8'))
            c.setdefault('retrieval_history',[]).append(old)
            c['retrieved']={**old,'sha256':actual,'text_sha256':actual,'raw_sha256':None,
                'quality':'legacy_unverified','hash_repair':'Reindexed existing extracted text; original bytes and parser unverified.'}
            if c.get('extracted'): c.setdefault('extraction_history',[]).append(c.pop('extracted'))
            sid=c.get('source_id')
            if sid in sources:
                sources[sid].update(sha256=actual,raw_sha256=None,retrieval_quality='legacy_unverified')
            changed.append({'family':family,'doi':c['doi'],'old_cache_key':sha,'text_sha256':actual})
        atomic_json(path,cs)
    save_sources(sources)
    if changed: atomic_json(Path(DATING_HISTORY)/'text_reindex'/(digest(changed)+'.json'),changed)
    return changed
