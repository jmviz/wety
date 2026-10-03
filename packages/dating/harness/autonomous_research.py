"""Bounded, resumable discovery-to-proposal queue. No scientific ingress.

Each network action is reserved before execution. An interrupted action is uncertain
and is never silently retried. Model phases share one locked budget ledger.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sys
from urllib.parse import urlencode
from itertools import zip_longest

from .io_utils import atomic_json,digest,locked
from .evaluation_runtime import create_ledger,remaining_seconds,run_process_group
from .section_packets import build_packets
from .document_packet import parse_article
from .compact_review import prepare_reviews
from workflow_tools.paths import REPO


def prepare(brief_path,root):
    brief=json.loads(Path(brief_path).read_text())
    if root.exists():raise ValueError('new research root required; never reset a run')
    if not 1<=len(brief['queries'])<=6:raise ValueError('one to six queries required')
    limits=brief['limits']
    for k,cap in [('candidates',12),('sources',2),('followups',2),('packets',4)]:
        if type(limits[k]) is not int or not 0<limits[k]<=cap:raise ValueError('invalid limit: '+k)
    if type(limits.get('native_repair_cells',0)) is not int or not 0<=limits.get('native_repair_cells',0)<=12:
        raise ValueError('native_repair_cells must be an integer from zero to twelve')
    if brief['selection_backend'] not in brief['configs']:raise ValueError('selection backend missing')
    if not set(brief['extraction_backends'])<=set(brief['configs']):raise ValueError('extraction backend missing')
    root.mkdir(parents=True);(root/'configs').mkdir()
    for b,config in brief['configs'].items():shutil.copyfile(config,root/'configs'/f'{b}.json')
    create_ledger(root/'budget.json',brief['budget_policy'])
    atomic_json(root/'brief.json',brief)
    atomic_json(root/'state.json',{'brief_sha256':digest(brief),'network':{},'stages':{},'limitations':['Discovery sources are a bounded bibliography, not comprehensive family coverage.','No held-out source/semantic reference is available; model quality remains unmeasured.']})
    snap=root/'implementation';snap.mkdir()
    for p in Path(__file__).parent.glob('*.py'):shutil.copyfile(p,snap/p.name)
    atomic_json(root/'config-lock.json',{p.name:digest(json.loads(p.read_text())) for p in (root/'configs').glob('*.json')})


def pin_outputs(root,state):
    """Pin pipeline-owned stage products; annotations and reports stay separate."""
    paths=[root/n for n in ('candidates.json','discovery-omissions.json','source-graph.json',
                            'sources.json','documents.json','packet-omissions.json')]
    for name in ('sources','screen','extract','review','repair','repair_review'):
        paths.extend(p for p in (root/name).rglob('*') if p.is_file() and p.suffix in ('.json','.txt','.py'))
    state['output_hashes']={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths if p.is_file()}


def verify_outputs(root,state):
    for name,expected in state.get('output_hashes',{}).items():
        p=root/name
        if not p.is_file() or hashlib.sha256(p.read_bytes()).hexdigest()!=expected:
            raise ValueError('stage output changed: '+name)


def verify(root,*,implementation=True):
    brief=json.loads((root/'brief.json').read_text());s=json.loads((root/'state.json').read_text())
    if digest(brief)!=s['brief_sha256']:raise ValueError('research brief changed')
    configs={p.name:digest(json.loads(p.read_text())) for p in (root/'configs').glob('*.json')}
    if configs!=json.loads((root/'config-lock.json').read_text()):raise ValueError('configuration changed')
    verify_outputs(root,s)
    if implementation:
        for p in (root/'implementation').glob('*.py'):
            if p.read_bytes()!=Path(__file__).with_name(p.name).read_bytes():raise ValueError('implementation changed; preserve run and start a declared new version')
    return brief,s


def save(root,state):atomic_json(root/'state.json',state)


def network(root,state,key,url):
    if key in state['network']:
        entry=state['network'][key]
        if entry['status']=='complete':
            p=REPO/entry['artifact']/'files/source'
            if not p.is_file() or hashlib.sha256(p.read_bytes()).hexdigest()!=entry['sha256']:
                raise ValueError('retained acquisition changed: '+key)
            return p
        return None  # incomplete/failed reservations cannot become free retries
    if (root/'STOP').exists() or remaining_seconds(root/'budget.json')<=0:return None
    entry={'url':url,'status':'reserved'};state['network'][key]=entry;save(root,state)
    try:
        p=run_process_group([sys.executable,'-m','source_tools.acquire','dating-autonomous-http',url],min(60,remaining_seconds(root/'budget.json')))
        if p.returncode:raise ValueError(p.stderr[-1600:])
        artifact=p.stdout.strip().splitlines()[-1];path=REPO/artifact/'files/source'
        if not path.is_file():raise ValueError('acquisition artifact absent')
        entry.update(status='complete',artifact=artifact,sha256=hashlib.sha256(path.read_bytes()).hexdigest());save(root,state);return path
    except Exception as exc:
        entry.update(status='failed',error=str(exc)[:1600]);save(root,state);return None


def candidates_from(data,origin):
    out=[]
    for x in data.get('resultList',{}).get('result',[]):
        pmc=x.get('pmcid');doi=(x.get('doi') or '').lower();key=doi or x.get('source','')+':'+str(x.get('id'))
        urls=[u['url'] for u in x.get('fullTextUrlList',{}).get('fullTextUrl',[]) if u.get('availabilityCode')=='OA' and u.get('url','').startswith('https://')]
        if pmc:urls=[f'https://pmc.ncbi.nlm.nih.gov/articles/{pmc}/',*urls]
        out.append({'key':key,'doi':doi or None,'title':x.get('title'),'abstract':x.get('abstractText'),'year':x.get('pubYear'),'type':x.get('pubTypeList',{}).get('pubType',[]),'pmcid':pmc,'urls':list(dict.fromkeys(urls)),'found_by':[origin]})
    return out


def discover(root,brief,state):
    if 'discover' in state['stages']:return
    found={};batches=[]
    for i,q in enumerate(brief['queries']):
        url='https://www.ebi.ac.uk/europepmc/webservices/rest/search?'+urlencode({'query':q,'format':'json','resultType':'core','pageSize':12})
        p=network(root,state,'query-'+str(i),url)
        if not p:continue
        try:
            batches.append(candidates_from(json.loads(p.read_text()),str(i)))
        except (ValueError,TypeError,KeyError) as exc:state['network']['query-'+str(i)]['parse_error']=str(exc)
    # Round-robin prevents the first query from consuming the entire screen cap.
    for group in zip_longest(*batches):
        for c in group:
            if c is None:continue
            if c['key'] in found:found[c['key']]['found_by']+=c['found_by']
            else:found[c['key']]=c
    all_candidates=list(found.values());selected=all_candidates[:brief['limits']['candidates']]
    atomic_json(root/'candidates.json',selected);atomic_json(root/'discovery-omissions.json',all_candidates[len(selected):])
    state['stages']['discover']={'candidates':len(selected),'omitted_by_cap':len(all_candidates)-len(selected),'status':'partial' if any(e['status']!='complete' or 'parse_error' in e for k,e in state['network'].items() if k.startswith('query-')) else 'complete'};save(root,state)


def make_plan(root,name,tasks,cells):
    dest=root/name
    if (dest/'plan.json').exists():return dest
    dest.mkdir(exist_ok=True);shutil.copytree(root/'configs',dest/'configs',dirs_exist_ok=True)
    reference={'scope':'Unannotated autonomous proposal run. No held-out reference or semantic accuracy denominator.','brief_sha256':digest(json.loads((root/'brief.json').read_text()))}
    atomic_json(dest/'reference.json',reference)
    atomic_json(dest/'plan.json',{'id':name,'split':'autonomous_development_unscored','tasks':tasks,'cells':cells,'budget_ledger':'../budget.json','budget':{'max_seconds_per_cell':150},'reference_file':'reference.json','reference_sha256':hashlib.sha256((dest/'reference.json').read_bytes()).hexdigest()})
    return dest


def run_phase(root,directory,phase):
    if (root/'STOP').exists() or remaining_seconds(root/'budget.json')<=0:return
    p=run_process_group([sys.executable,'-m','harness.live_evaluation','run',str(directory),'--phase',phase],remaining_seconds(root/'budget.json'))
    (directory/'phase.log').write_text(p.stdout+'\n'+p.stderr)
    if p.returncode:raise ValueError('phase failed; see '+str(directory/'phase.log'))


def aligned_decisions(candidates,result):
    rows=result.get('output',{}).get('decisions',[]);keys=[r.get('key') for r in rows]
    expected=[c['key'] for c in candidates]
    if result.get('status')!='completed' or len(keys)!=len(set(keys)) or set(keys)!=set(expected):return None
    by_key={r['key']:r for r in rows}
    return [by_key[k] for k in expected]


def screen(root,brief,state):
    if 'screen' in state['stages']:return
    cs=json.loads((root/'candidates.json').read_text());tasks={'screen':{'kind':'screen_named','families':[brief['family']],'inputs':cs}}
    cells=[{'id':b+'-screen','backend':b,'config':b+'.json','task':'screen','role':'screen','phase':'screen'} for b in brief['configs']]
    d=make_plan(root,'screen',tasks,cells)
    if cs:run_phase(root,d,'screen')
    p=d/'results'/f'{brief["selection_backend"]}-screen.json';r=json.loads(p.read_text()) if p.exists() else {}
    decisions=aligned_decisions(cs,r)
    aligned=decisions is not None
    # No selector fallback or manual substitution. Candidate IDs must match once.
    graph=[{**c,'screen':decisions[i] if aligned else None,'status':'excluded' if aligned and decisions[i]['decision']=='exclude' else 'screen_unresolved'} for i,c in enumerate(cs)]
    selected=0
    for row in graph:
        if aligned and row['screen']['decision']=='include':
            row['status']='selected' if selected<brief['limits']['sources'] else 'source_cap_omission'
            selected+=1
    atomic_json(root/'source-graph.json',graph)
    state['stages']['screen']={'status':'complete' if aligned else 'unresolved','selector':brief['selection_backend'],'request_ref':r.get('request_ref'),'selected':min(selected,brief['limits']['sources'])};save(root,state)


def acquire_sources(root,brief,state):
    if 'sources' in state['stages']:return
    graph=json.loads((root/'source-graph.json').read_text());sources=[];followups=0
    for index,row in enumerate(graph):
        if row['status']!='selected':continue
        row['acquisitions']=[];usable=None
        # One initial route, then at most two additional routes/linked supplements
        # across the whole run; no manually chosen evidence URLs.
        for i,url in enumerate(row['urls']):
            if i:
                if followups>=brief['limits']['followups']:break
                followups+=1
            p=network(root,state,f'source-{index}-{i}',url)
            attempt={'url':url,'status':'failed'};row['acquisitions'].append(attempt)
            if p:
                attempt['artifact']=str(p.parent.parent.relative_to(REPO))
                try:
                    text,meta=parse_article(p,url);usable=(text,meta);attempt['status']='parsed_article';break
                except (ValueError,UnicodeError) as exc:attempt.update(status='unusable_response',error=str(exc))
        if usable:
            text,meta=usable;row['status']='parsed_article';row['processing_scope']=meta['scope']
            sid=digest(row['key'])[:16];folder=root/'sources';folder.mkdir(exist_ok=True)
            (folder/f'{sid}.txt').write_text(text);atomic_json(folder/f'{sid}.json',meta)
            sources.append({'id':sid,'candidate_key':row['key'],'text':str(folder/f'{sid}.txt'),'metadata':str(folder/f'{sid}.json')})
            row['linked_material']=meta['attachments']
            for j,attachment in enumerate(meta['attachments']):
                if followups>=brief['limits']['followups']:break
                followups+=1;p=network(root,state,f'linked-{index}-{j}',attachment['url'])
                attachment['status']='acquired_unprocessed' if p else 'failed'
                if p:attachment['artifact']=str(p.parent.parent.relative_to(REPO))
            row['images']=meta['images']
        else:row['status']='access_or_parser_gap';row['gap']='No parseable article through bounded discovered routes. No new source substituted.'
    atomic_json(root/'source-graph.json',graph)
    atomic_json(root/'sources.json',sources);state['stages']['sources']={'parsed':len(sources),'followups_used':followups,'status':'complete_with_gaps'};save(root,state)


def extract(root,brief,state):
    if 'extract' in state['stages']:return
    tasks={};omitted=[];documents=[]
    for source in json.loads((root/'sources.json').read_text()):
        text=Path(source['text']).read_text();meta=json.loads(Path(source['metadata']).read_text())
        doc=build_packets(text,source['id'],headings=meta['headings'],omissions=[*meta['attachments'],*meta['images']],max_chars=18000,context_chars=3000,overlap_chars=500)
        documents.append(doc)
    for group in zip_longest(*(d['packets'] for d in documents)):
        for packet in group:
            if packet is None:continue
            if len(tasks)>=brief['limits']['packets']:omitted.append({'source':packet['source_id'],'packet':packet['id'],'reason':'global_packet_cap'});continue
            key=digest(packet)[:16];tasks[key]={'kind':'compact_extract','family':brief['family'],'packet':packet}
    atomic_json(root/'documents.json',documents)
    atomic_json(root/'packet-omissions.json',omitted)
    cells=[{'id':b+'-'+k,'backend':b,'config':b+'.json','task':k,'role':'extract','phase':'extract'} for b in brief['extraction_backends'] for k in tasks]
    d=make_plan(root,'extract',tasks,cells)
    if cells:run_phase(root,d,'extract')
    state['stages']['extract']={'scheduled_packets':len(tasks),'omitted_packets':len(omitted),'status':'proposals_only'};save(root,state)


def review(root,brief,state):
    if 'review' in state['stages']:return
    d=root/'review'
    if not d.exists():prepare_reviews(root/'extract',d,max_batches=brief['limits']['packets'],reviewers=brief['review_routing'])
    run_phase(root,d,'reviews');state['stages']['review']={'status':'proposals_only'};save(root,state)


def repair(root,brief,state):
    if 'repair' in state['stages']:return
    limit=brief['limits'].get('native_repair_cells',0)
    if not limit:
        state['stages']['repair']={'status':'disabled_in_brief'};save(root,state);return
    from .compact_repair import prepare_repairs
    d=root/'repair'
    if not d.exists():prepare_repairs(root/'extract',d,max_cells=limit)
    run_phase(root,d,'repairs')
    state['stages']['repair']={'status':'native_proposals_only'};save(root,state)


def review_repairs(root,brief,state):
    if 'repair_review' in state['stages'] or not brief['limits'].get('native_repair_cells',0):return
    d=root/'repair_review'
    if not d.exists():prepare_reviews(root/'repair',d,max_batches=brief['limits']['native_repair_cells'],reviewers=brief['review_routing'])
    run_phase(root,d,'reviews');state['stages']['repair_review']={'status':'proposals_only'};save(root,state)


def report(root):
    brief,state=verify(root,implementation=False);out={'scope':'Autonomous model-selected bounded research proposals, not historical approval or measured semantic accuracy.','brief_sha256':state['brief_sha256'],'stages':state['stages'],'network_statuses':{k:v['status'] for k,v in state['network'].items()},'models':{},'limitations':state['limitations'],'execution_error':state.get('execution_error')}
    for name in ('screen','extract','review','repair','repair_review'):
        d=root/name
        if not (d/'plan.json').exists():continue
        plan=json.loads((d/'plan.json').read_text());rows=[]
        for c in plan['cells']:
            p=d/'results'/f'{c["id"]}.json';r=json.loads(p.read_text()) if p.exists() else {'status':'not_attempted'}
            rows.append({'cell':c['id'],'status':r['status'],'schema_valid_proposals':r.get('recovery',{}).get('valid_claims'), 'overflow':r.get('recovery',{}).get('overflow'), 'aligned_review':r.get('semantic_review',{}).get('aligned'),
                         'aligned_native_repair':r.get('native_repair',{}).get('aligned'),'repaired_proposals':r.get('native_repair',{}).get('valid_claims')})
        out['models'][name]=rows
    out['scientific_import']=False;out['mass_release']=False
    atomic_json(root/'report.json',out);return out


def run(root):
    with locked(root/'queue-lock'):
        brief,state=verify(root)
        for stage in (discover,screen,acquire_sources,extract,review,repair,review_repairs):
            if (root/'STOP').exists() or remaining_seconds(root/'budget.json')<=0:break
            verify_outputs(root,state)
            try:stage(root,brief,state)
            except Exception as exc:
                state['execution_error']={'stage':stage.__name__,'type':type(exc).__name__,'message':str(exc)[:1600]}
                pin_outputs(root,state);save(root,state)
                return report(root)
            pin_outputs(root,state);save(root,state)
        return report(root)


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['prepare','run','report']);p.add_argument('directory',type=Path);p.add_argument('--brief',type=Path);a=p.parse_args()
    if a.mode=='prepare':prepare(a.brief,a.directory.resolve())
    else:print(json.dumps(run(a.directory.resolve()) if a.mode=='run' else report(a.directory.resolve()),indent=2))

if __name__=='__main__':main()
