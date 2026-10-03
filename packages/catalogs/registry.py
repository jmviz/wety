"""Versioned catalog assertions. A lookup correspondence is never entity identity.

All relation directions are source -> target. No mapping relation is composed
transitively. Reviewed equivalence is scoped to this snapshot and purpose.
"""
from __future__ import annotations
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
import fcntl
from collections import Counter, defaultdict

from workflow_tools.paths import (REPO, CATALOG_CODE, CATALOG_PRODUCT, CATALOG_WORKSPACE,
    CATALOG_SOURCES, CATALOG_GENERATIONS, selected, resolve)
ROOT=CATALOG_PRODUCT
POLICY='catalog-links-2.0'
RELATIONS={'catalog_correspondence','equivalent','source_narrower','source_broader','overlap','unresolved','family_context','parent_context','retired_successor'}
STATES={'catalog_link','proposed','context','reviewed','rejected'}
NON_GENEALOGICAL={'Bookkeeping','Sign_Language','Unclassifiable','Unattested','Artificial_Language','Pidgin','Mixed_Language','Speech_Register'}


def digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()).hexdigest()


def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def atomic(path, data):
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    fd,tmp=tempfile.mkstemp(dir=path.parent)
    with os.fdopen(fd,'wb') as f: f.write(data);f.flush();os.fsync(f.fileno())
    os.replace(tmp,path)


def json_bytes(data):return (json.dumps(data,ensure_ascii=False,sort_keys=True,indent=1)+'\n').encode()


def source_snapshot(cldf, verify=True):
    from . import build
    declared=json.loads((CATALOG_SOURCES/'manifest.json').read_text())
    files={'source_manifest':{'sha256':sha(CATALOG_SOURCES/'manifest.json'),'provenance':{'path':os.path.relpath(CATALOG_SOURCES/'manifest.json',ROOT)}}}
    for name,meta in declared['files'].items():
        p=CATALOG_SOURCES/name
        if not p.is_file(): raise ValueError('missing pinned source '+name)
        if verify and sha(p)!=meta['sha256']: raise ValueError('source hash mismatch '+name)
        files['sources/'+name]={'sha256':sha(p),'provenance':meta}
    for label,p in [('wiktionary_languages',build.LANGUAGES_JSON),('wiktionary_families',build.FAMILIES_JSON)]:
        files[label]={'sha256':sha(p),'provenance':{'path':os.path.relpath(p,ROOT),'upstream_revision':'not recorded in legacy export'}}
    receipt=selected('wiktionary_languages').parent/'wiktionary_export_receipt.json'
    if receipt.exists():
        received=json.loads(receipt.read_text())
        for p in (build.LANGUAGES_JSON,build.FAMILIES_JSON):
            if received['outputs'].get(Path(p).name)!=sha(p):raise ValueError('stale Wiktionary export receipt')
        files['wiktionary_export_receipt']={'sha256':sha(receipt),'provenance':{'path':os.path.relpath(receipt,ROOT)}}
    for name in ('languages.csv','values.csv','names.csv','cldf-metadata.json'):
        p=Path(cldf)/name
        files['glottolog/'+name]={'sha256':sha(p),'provenance':{'path':os.path.relpath(p,ROOT)}}
    meta=Path(cldf).parent/'metadata.json'
    if meta.exists(): files['glottolog/metadata.json']={'sha256':sha(meta),'provenance':{'path':os.path.relpath(meta,ROOT),'metadata':json.loads(meta.read_text())}}
    target=declared['files']['wikidata_items.json'].get('inputs',{})
    for key,label in [('languages.json','wiktionary_languages'),('families.json','wiktionary_families')]:
        if target.get(key)!=files[label]['sha256']: raise ValueError('Wikidata target snapshot stale for '+key)
    return {'files':files,'digest':digest({name:info['sha256'] for name,info in files.items()}),'identity_policy':'source-file-hashes-v1','limitations':['Source agreement is not independent corroboration.','Legacy Wikidata truthy extracts omit statement ranks, qualifiers and references.',('Wiktionary dump receipt available; individual module revisions not indexed.' if receipt.exists() else 'Legacy Wiktionary export lacks a dump revision receipt.')]}


def link(source,target,relation,status,basis,snapshot,metadata=None):
    body={'source':source,'target':target,'relation':relation,'status':status,'basis':basis,
          'source_snapshot':snapshot,'target_metadata':metadata or {},'temporal_transfer_allowed':False}
    return {'id':'link:'+digest(body),**body}


def convert_record(rec,raw,is_family,iso,gl,wd,snapshot):
    from . import build
    code=rec['code'];sid=('wikt:family:' if is_family else 'wikt:lang:')+code
    q,d=build.wd_item(wd,raw.get('wikidataItem'))
    kind='family' if is_family else raw['kind']
    result={'schema_version':2,'id':sid,'code':code,'name':raw['canonicalName'],'kind':kind,
        'wiktionary_family':raw.get('family'),'wiktionary_parents':raw.get('parents',[]),
        'wiktionary_ancestors':raw.get('ancestors',[]),'wiktionary_superfamilies':raw.get('superfamilies',[]),
        'wiktionary_non_etymology_only':raw.get('nonEtymologyOnly'),'wiktionary_alias_codes':raw.get('_aliasCodes',[]),
        'proto_language':raw.get('protoLanguage'),'wikidata':raw.get('wikidataItem'),'wikidata_resolved':q,
        'links':[],'issues':list(rec.get('conflicts',[])), 'source_snapshot':snapshot,
        'equivalence_status':'not_adjudicated','temporal_transfer_allowed':False,
        'source_names':{k:raw.get(k,[]) for k in ('aliases','otherNames','varieties')},'source_notes':rec.get('notes',[]),
        'source_assertions':{'wikidata_codes':{k:d.get(k,[]) for k in ('iso639_1','iso639_2','iso639_3','iso639_5','glottocode')},'wikidata_label':d.get('label'),'location':'sources/wikidata_items.json#'+q if q else None}}
    if q:
        result['links'].append(link(sid,'wikidata:'+q,'catalog_correspondence','context',
            {'route':'wiktionary wikidataItem','asserted_item':raw.get('wikidataItem'),'shared_with':rec.get('wikidata_shared_with',[]),'statement_provenance':'legacy truthy values; ranks/qualifiers/references unavailable'},snapshot))
    scopes=[('iso639_3','iso639-3',list(d.get('iso639_3',[]))),('glottocode','glottolog',list(d.get('glottocode',[])))]
    if is_family:scopes=[('iso639_5','iso639-5',list(d.get('iso639_5',[]))),scopes[1]]
    for field,namespace,values in scopes:
        chosen=rec.get(field);route=rec.get(field+'_route') or ''
        candidates=set(values)
        if chosen:candidates.add(chosen)
        for k,v in rec.get('alternatives',{}).items():
            if k.startswith(field): candidates.update(v if isinstance(v,list) else [v])
        conflict=bool(rec.get('conflicts'))
        if namespace=='glottolog' and route.startswith('iso'):
            iso_candidates={rec.get('iso639_3'),*d.get('iso639_3',[])}-{None}
            conflict=conflict or len(iso_candidates)>1
        ambiguous=len(candidates)>1
        for value in sorted(candidates):
            if namespace=='glottolog':
                target=gl['langs'].get(value);valid=bool(target)
                metadata={k:target.get(k) for k in ('Name','Level','category')} if target else {}
                special=bool(target and (target.get('category') in NON_GENEALOGICAL or (not is_family and target['Level']=='family')))
            else:
                valid=value in (iso['iso5'] if is_family else iso['iso3']) or (not is_family and value in iso['retired'])
                metadata=({'name':iso['iso5'].get(value)} if is_family else build.iso3_info(iso,value))
                special=not is_family and value in iso['retired']
            relation='catalog_correspondence';state='catalog_link'
            if kind=='reconstructed' or special:
                relation='family_context' if namespace=='glottolog' and metadata.get('Level')=='family' else 'unresolved';state='context'
            if value!=chosen or ambiguous or conflict or not valid or route in ('name','proto-wikidata-subclass','inherited','') or rec.get('_wd_is_parents') or rec.get('_wd_shared_unrelated'):
                relation='unresolved';state='proposed'
            if namespace=='glottolog' and kind=='reconstructed':relation='family_context';state='context'
            if namespace=='glottolog' and target and target.get('category') in NON_GENEALOGICAL:relation='unresolved';state='context'
            if rec.get('iso639_3_status')=='retired' and namespace=='glottolog':relation='unresolved';state='proposed'
            result['links'].append(link(sid,namespace+':'+value,relation,state,
                {'route':route if value==chosen else 'alternative','wikidata_values':values,
                 'dependency':'catalog assertions may copy one another; independence unassessed','valid_target':valid},snapshot,metadata))
    if not is_family:
        parent=raw.get('nonEtymologyOnly')
        if parent and parent!=code:
            result['links'].append(link(sid,'wikt:lang:'+parent,'parent_context','context',{'route':'Wiktionary nonEtymologyOnly; not necessarily genetic ancestry'},snapshot))
        # Preserve every proto-associated family. No fallback through generic family
        # membership and no inference that P279 means reconstructed ancestor of.
        for fc in rec.get('_proto_families',[]):
            result['links'].append(link(sid,'wikt:family:'+fc,'family_context','context',{'route':'Wiktionary family.protoLanguage'},snapshot))
        for gid in rec.get('_proto_subclass_nodes',[]):
            result['links'].append(link(sid,'glottolog:'+gid,'family_context','context',{'route':'Wikidata P279 association; not a phylogenetic or chronological identity'},snapshot))
    # Catalog retirement is a registry operation, not linguistic descent/extinction.
    for e in list(result['links']):
        code3=e['target'].removeprefix('iso639-3:')
        rt=iso['retired'].get(code3) if e['target'].startswith('iso639-3:') else None
        if rt:
            successors=set(re.findall(r'\[([a-z]{3})\]',rt.get('Ret_Remedy','')))
            if rt.get('Change_To'):successors.add(rt['Change_To'])
            e['target_metadata']['retirement_successors']=sorted(successors)
    # ISO parts are separate namespaces even when their strings coincide.
    for edge in list(result['links']):
        if not edge['target'].startswith(('iso639-3:','iso639-5:')):continue
        code3=edge['target'].split(':',1)[1]
        r=iso['iso3'].get(code3,{})
        r2=iso['iso2'].get(code3,{})
        for ns,value in [('iso639-1',r.get('Part1') or r2.get('iso639_1')),('iso639-2b',r.get('Part2b') or r2.get('bib')),('iso639-2t',r.get('Part2t') or r2.get('term'))]:
            if value:result['links'].append(link(sid,ns+':'+value,'catalog_correspondence','context',{'route':'ISO registration crosswalk','via':edge['target'],'not_independent':True},snapshot))
    for edge in result['links']:
        edge['id']='link:'+digest({k:v for k,v in edge.items() if k!='id'})
    if not result['links']:result['issues'].append('no catalog correspondence established; retain source concept')
    result['links']=sorted({e['id']:e for e in result['links']}.values(),key=lambda e:e['id'])
    return result


import re


def apply_decisions(records, decisions, snapshot, gl, iso):
    allowed={'target','relation','reviewer','rationale','references','source_snapshot','purpose'}
    for sid,d in decisions.items():
        if sid not in records:raise ValueError('unknown override subject '+sid)
        if set(d)!=allowed:raise ValueError('override fields must be '+str(sorted(allowed)))
        if d['source_snapshot']!=snapshot:raise ValueError('stale mapping decision '+sid)
        if d['relation'] not in {'equivalent','source_narrower','source_broader','overlap','catalog_correspondence','unresolved','family_context'}:raise ValueError('invalid reviewed relation')
        if not all(isinstance(d[k],str) and d[k].strip() for k in ('reviewer','rationale','purpose')) or not d['references']:raise ValueError('decision needs reviewer, rationale, purpose and references')
        namespace,code=d['target'].split(':',1)
        if namespace=='glottolog' and code not in gl['langs']:raise ValueError('unknown override glottocode')
        if namespace=='iso639-3' and code not in iso['iso3'] and code not in iso['retired']:raise ValueError('unknown override ISO code')
        if namespace not in ('glottolog','iso639-3','iso639-5'):raise ValueError('unsupported override namespace')
        if namespace=='iso639-5' and code not in iso['iso5']:raise ValueError('unknown override ISO family')
        rec=records[sid]
        if d['relation']=='equivalent' and (rec['kind']=='reconstructed' or namespace=='glottolog' and gl['langs'][code].get('category') in NON_GENEALOGICAL):
            raise ValueError('reconstructed stages and bookkeeping need contextual relations')
        if namespace=='glottolog' and gl['langs'][code].get('category') in NON_GENEALOGICAL and d['relation']=='catalog_correspondence':raise ValueError('non-genealogical lookup requires contextual relation')
        metadata=next((e['target_metadata'] for e in rec['links'] if e['target']==d['target']),{})
        edge=link(sid,d['target'],d['relation'],'reviewed',d,snapshot,metadata)
        rec['links'].append(edge)
        rec['equivalence_status']='reviewed_for_purpose' if d['relation']=='equivalent' else 'not_adjudicated'


def project(rec):
    """Compatibility scalars are unambiguous catalog lookups, never exact joins."""
    for field in ('glottocode','iso639_3','iso639_5','glottocode_route','iso639_3_route','iso639_5_route'):rec.pop(field,None)
    for field,namespace in [('glottocode','glottolog'),('iso639_3','iso639-3'),('iso639_5','iso639-5')]:
        es=[e for e in rec['links'] if e['target'].startswith(namespace+':')]
        reviewed=[e for e in es if e['status']=='reviewed' and e['relation'] in ('equivalent','catalog_correspondence')]
        eligible=reviewed if any(e['status']=='reviewed' for e in es) else [e for e in es if e['status']=='catalog_link' and e['relation']=='catalog_correspondence']
        targets={e['target'].split(':',1)[1] for e in eligible}
        if len(targets)==1:
            rec[field]=targets.pop();rec[field+'_route']='reviewed' if reviewed else 'catalog_link'
    rec['review_state']='needs_review' if rec['issues'] or any(e['status']=='proposed' for e in rec['links']) or not rec.get('glottocode') else 'catalog_links_only'
    rec['links'].sort(key=lambda e:e['id'])
    return rec


def reverse_indexes(records, families=None):
    all_links=defaultdict(list);iso=defaultdict(list);glotto=defaultdict(list)
    for code,rec in sorted(records.items()):
        for e in rec['links']:all_links[e['target']].append({'source':rec['id'],'link_id':e['id'],'relation':e['relation'],'status':e['status']})
        if rec.get('iso639_3'):iso[rec['iso639_3']].append(code)
        if rec.get('glottocode'):glotto[rec['glottocode']].append(code)
    for rec in sorted((families or {}).values(),key=lambda r:r['id']):
        for e in rec['links']:all_links[e['target']].append({'source':rec['id'],'link_id':e['id'],'relation':e['relation'],'status':e['status']})
    return dict(all_links),dict(iso),dict(glotto)


def alias_index(languages):
    out={}
    for c,r in sorted(languages.items()):
        for alias in [c,*r['wiktionary_alias_codes']]:
            if alias in out and out[alias]!=r['id']:raise ValueError('ambiguous Wiktionary alias '+alias)
            out[alias]=r['id']
    return out


def validate_bundle(b):
    errors=[];ids=set()
    snapshot=b['source_snapshot'];identity=snapshot.get('identity_policy','legacy-paths-v1')
    if identity not in ('source-file-hashes-v1','legacy-paths-v1'):errors.append('unknown source identity policy')
    identity_data={name:info['sha256'] for name,info in snapshot['files'].items()} if identity=='source-file-hashes-v1' else snapshot['files']
    if digest(identity_data)!=snapshot['digest']:errors.append('source snapshot digest mismatch')
    if b.get('schema_version')!=2 or b.get('policy')!=POLICY:errors.append('unsupported bundle policy')
    records={**b['languages'],**{'family:'+k:v for k,v in b['families'].items()}}
    subjects={r['id'] for r in records.values()};gl={r['glottocode'] for r in b['glottolog']}
    for r in records.values():
        if r['id']!=('wikt:family:' if r['kind']=='family' else 'wikt:lang:')+r['code']:errors.append('concept namespace mismatch')
        if r['source_snapshot']!=b['source_snapshot']['digest']:errors.append('stale record')
        if r['temporal_transfer_allowed'] is not False:errors.append('record date transfer forbidden')
        if r.get('schema_version')!=2:errors.append('wrong record schema')
        if 'confidence' in r or any(k in r for k in ('glottolog_first_year','glottolog_last_year','inherited_from')):errors.append('unsafe flattened attributes '+r['id'])
        for e in r['links']:
            if e['id'] in ids:errors.append('duplicate link ID')
            ids.add(e['id'])
            if e['source']!=r['id'] or e['relation'] not in RELATIONS or e['status'] not in STATES:errors.append('invalid link')
            if e['id']!='link:'+digest({k:v for k,v in e.items() if k!='id'}):errors.append('link digest mismatch')
            if e['source_snapshot']!=b['source_snapshot']['digest']:errors.append('stale link')
            if e['temporal_transfer_allowed'] is not False:errors.append('date transfer forbidden')
            if e['target'].startswith('wikt:') and e['target'] not in subjects:errors.append('missing local target '+e['target'])
            if e['status'] in ('catalog_link','reviewed') and e['target'].startswith('glottolog:') and e['target'].split(':',1)[1] not in gl:errors.append('unknown accepted glottocode')
            if e['status']=='reviewed' and not all(e['basis'].get(k) for k in ('reviewer','rationale','references','purpose')):errors.append('undocumented decision')
            if r['kind']=='reconstructed' and e['status']=='catalog_link':errors.append('reconstructed stage catalog identity')
            if e['relation']=='equivalent' and e['status']!='reviewed':errors.append('unreviewed equivalence')
            if e['status']=='catalog_link' and (e['relation']!='catalog_correspondence' or not e['basis'].get('valid_target')):errors.append('invalid catalog link')
            if e['status']=='reviewed' and b['decisions'].get(r['id'])!=e['basis']:errors.append('reviewed assertion lacks matching decision')
            if e['status'] in ('catalog_link','reviewed') and e['target_metadata'].get('category') in NON_GENEALOGICAL and e['relation'] in ('equivalent','catalog_correspondence'):errors.append('non-genealogical target promoted to lookup identity')
        projected=project({k:v for k,v in r.items() if k not in ('glottocode','iso639_3','iso639_5')})
        for key in ('glottocode','iso639_3','iso639_5'):
            if r.get(key)!=projected.get(key):errors.append('unsafe scalar projection '+r['id'])
    if b['index_by_wiktionary_code']!=alias_index(b['languages']):errors.append('alias index mismatch')
    expected=reverse_indexes(b['languages'],b['families'])
    for key,v in zip(('index_by_target','index_by_iso639_3','index_by_glottocode'),expected):
        if b[key]!=v:errors.append('reverse index mismatch '+key)
    return errors


def csv_bytes(rows,fields):
    out=io.StringIO();w=csv.DictWriter(out,fieldnames=fields);w.writeheader()
    for r in rows:w.writerow({k:json.dumps(r[k],ensure_ascii=False,sort_keys=True) if isinstance(r.get(k),(list,dict)) else r.get(k,'') for k in fields})
    return out.getvalue().encode()


def report(b):
    records=list(b['languages'].values())+list(b['families'].values());links=[e for r in records for e in r['links']]
    return ('# Language cross-reference v2\n\n'
        f"Source snapshot: `{b['source_snapshot']['digest']}`. Policy: `{POLICY}`.\n\n"
        f"{len(b['languages'])} Wiktionary language/stage codes; {len(b['families'])} family codes; {len(b['glottolog'])} Glottolog catalog nodes.\n\n"
        'Link states (not accuracy or independent confirmations): '+json.dumps(dict(Counter(e['status'] for e in links)),sort_keys=True)+'.\n\n'
        f"{sum(bool(r.get('glottocode')) for r in b['languages'].values())} unambiguous language-code catalog lookups; {sum(e['relation']=='equivalent' and e['status']=='reviewed' for e in links)} reviewed equivalences.\n\n"
        f"{len(b['review_queue'])} records in the mapping review queue. See `review_queue.json`, `index_by_target.json`, and `critique.md`.\n\n"
        'Catalog correspondences do not transfer dates, status, sampled-clade identity or lexemes. Missing correspondence means unresolved or out of catalog scope, not nonexistence.\n\n'
        'Wikidata statement-level provenance and the original Wiktionary dump receipt remain unavailable for the legacy snapshots; this release does not reconstruct them. Extraction/mapping accuracy on held-out specialist judgments: **not measured**.\n')


def publish(languages,families,glottolog,snapshot,decisions):
    allidx,isoidx,glidx=reverse_indexes(languages,families)
    queue=[{'subject':r['id'],'name':r['name'],'issues':r['issues'],'link_ids':[e['id'] for e in r['links'] if e['status'] in ('proposed','context')],'reason':r['review_state'],'priority':('conflict' if r['issues'] else 'stage_scope' if r['kind'] in ('reconstructed','etymology-only') else 'catalog_scope'),'decision_needed':'Review concept extension for intended use; catalog lookup is not exact equivalence'} for r in [*languages.values(),*families.values()] if r['review_state']=='needs_review' or any(e['status']=='context' for e in r['links'])]
    queue.sort(key=lambda r:({'conflict':0,'stage_scope':1,'catalog_scope':2}[r['priority']],r['subject']))
    b={'schema_version':2,'policy':POLICY,'source_snapshot':snapshot,'languages':languages,'families':families,'glottolog':glottolog,
       'index_by_wiktionary_code':alias_index(languages),'index_by_target':allidx,'index_by_iso639_3':isoidx,'index_by_glottocode':glidx,'review_queue':queue,
       'decisions':decisions,'implementation':implementation_hashes()}
    errors=validate_bundle(b)
    if errors:raise ValueError('\n'.join(errors[:30]))
    gen=digest(b);folder=CATALOG_GENERATIONS/gen
    outputs={'wiktionary_langcodes.json':json_bytes(languages),'wiktionary_families.json':json_bytes(families),
        'index_by_wiktionary_code.json':json_bytes(b['index_by_wiktionary_code']),'index_by_target.json':json_bytes(allidx),'index_by_iso639_3.json':json_bytes(isoidx),'index_by_glottocode.json':json_bytes(glidx),'review_queue.json':json_bytes(queue),'report.md':report(b).encode()}
    for name,table in [('wiktionary_langcodes',languages),('wiktionary_families',families)]:
        outputs[name+'.csv']=csv_bytes(list(table.values()),['id','code','name','kind','review_state','glottocode','iso639_3','iso639_5','links','issues','source_snapshot'])
    outputs['glottolog_langcodes.csv']=csv_bytes(glottolog,list(glottolog[0]))
    with (ROOT/'.publish.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        atomic(folder/'bundle.json',json_bytes(b))
        for name,data in outputs.items():atomic(folder/name,data)
        atomic(folder/'manifest.json',json_bytes({'generation':gen,'files':{n:hashlib.sha256(v).hexdigest() for n,v in outputs.items()}}))
        for name,data in outputs.items():atomic(ROOT/name,data)
        atomic(ROOT/'CURRENT.json',json_bytes({'generation':gen,'bundle':os.path.relpath(folder/'bundle.json',ROOT)}))
    return b


def current_bundle(root=ROOT):
    root=Path(root);p=json.loads((root/'CURRENT.json').read_text());b=json.loads((root/p['bundle']).read_text())
    if digest(b)!=p['generation']:raise ValueError('cross-reference generation corrupted')
    return b


def validate_current(root=ROOT):
    root=Path(root);b=current_bundle(root);errors=validate_bundle(b)
    manifest=json.loads((CATALOG_GENERATIONS/digest(b)/'manifest.json').read_text())
    for name,h in manifest['files'].items():
        if sha(root/name)!=h:errors.append('stale or altered mirror '+name)
    for name,info in b['source_snapshot']['files'].items():
        p=CATALOG_SOURCES/name.removeprefix('sources/') if name.startswith('sources/') else root/info['provenance']['path'] if 'path' in info['provenance'] else None
        if p and sha(p)!=info['sha256']:errors.append('source changed; rebuild '+name)
    for name,h in b['implementation'].items():
        if sha(REPO/name)!=h:errors.append('implementation changed; rebuild '+name)
    if json.loads((CATALOG_WORKSPACE/'overrides.json').read_text()).get('decisions',{})!=b['decisions']:errors.append('decisions changed; rebuild')
    return errors


def implementation_hashes():
    paths=[]
    for package in ('catalogs','source_tools','workflow_tools'):
        paths.extend((REPO/'packages'/package).glob('*.py'))
    paths += [REPO/'pyproject.toml', REPO/'uv.lock']
    return {str(p.relative_to(REPO)):sha(p) for p in sorted(paths)}


if __name__=='__main__':
    errors=validate_current()
    for error in errors:print(error)
    print(f'{len(errors)} validation errors')
    raise SystemExit(bool(errors))

