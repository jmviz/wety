"""Prepare/run/reconcile a bounded source-section proposal experiment.

Run from the repository root. This command never imports scientific claims.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
from .io_utils import atomic_json, digest, locked
from .section_packets import build_packets, reconcile
from .evaluation_runtime import create_ledger


def prepare(manifest_path, root):
    manifest=json.loads(Path(manifest_path).read_text())
    if root.exists():raise ValueError('output directory exists; preserve it and choose a new version')
    root.mkdir(parents=True);(root/'configs').mkdir()
    reference=Path(manifest['reference']);shutil.copyfile(reference,root/'reference.json')
    tasks={};documents=[]
    for source in manifest['sources']:
        text=Path(source['text']).read_text();sha=hashlib.sha256(text.encode()).hexdigest()
        if sha!=source['text_sha256']:raise ValueError('source text hash changed: '+source['id'])
        meta=json.loads(Path(source['metadata']).read_text()) if source.get('metadata') else {}
        if meta.get('text_sha256',sha)!=sha:raise ValueError('source metadata hash mismatch')
        omissions=[*meta.get('attachments',[]),*meta.get('images',[])]
        document=build_packets(text,source['id'],headings=meta.get('headings',[]),
            context_spans=source.get('context_spans',[]),omissions=omissions,**manifest.get('packet_limits',{}))
        documents.append(document)
        for packet in document['packets']:
            key=digest(packet)[:16]
            tasks[key]={'kind':'compact_extract' if manifest.get('transport')=='compact' else 'section_extract','family':manifest['family'],'packet':packet,
                        'source':{'source_id':source['id'],'title':source['title']}}
    cells=[]
    for backend,config in manifest['configs'].items():
        shutil.copyfile(config,root/'configs'/f'{backend}.json')
        for key in tasks:
            cells.append({'id':f'{backend}-{key}','backend':backend,'config':backend+'.json',
                          'task':key,'role':'extract','phase':'sections','tier':'declared-profile'})
    create_ledger(root/'budget.json',manifest['budget_policy'])
    atomic_json(root/'manifest.json',manifest);atomic_json(root/'documents.json',documents)
    plan={'id':root.name,'split':manifest.get('split','exposed_section_development'),'tasks':tasks,'cells':cells,
          'budget_ledger':'budget.json','budget':{'max_seconds_per_cell':manifest.get('worker_timeout',300)},
          'reference_file':'reference.json','reference_sha256':hashlib.sha256(reference.read_bytes()).hexdigest(),
          'scope':'Bounded section proposals, with partial failures retained; no scientific ingestion.'}
    atomic_json(root/'plan.json',plan)
    return {'sections':len(tasks),'cells':len(cells),'planned_primary_chars':sum(d['source_chars'] for d in documents)}


def load_results(root):
    return {p.stem:json.loads(p.read_text()) for p in (root/'results').glob('*.json')}


def report(root):
    plan=json.loads((root/'plan.json').read_text());results=load_results(root)
    reports={}
    for backend in sorted({c['backend'] for c in plan['cells']}):
        reports[backend]=reconcile({**plan,'cells':[c for c in plan['cells'] if c['backend']==backend]},results)
        reports[backend]['repairs']=[{'cell':c['id'],'repair_of':plan['tasks'][c['task']]['repair_of'],
            'status':results.get(c['id'],{}).get('status','not_attempted'),
            'review':results.get(c['id'],{}).get('repair_review')}
            for c in plan['cells'] if c['backend']==backend and plan['tasks'][c['task']]['kind']=='section_repair']
    out={'plan_sha256':digest(plan),'backends':reports,'scope':'Proposals and execution coverage only; no automatic adoption.'}
    atomic_json(root/'section-report.json',out)
    return out


def prepare_repairs(root, target, max_cells=3, max_claims=4):
    # Reserve the one permitted repair durably across new output directories.
    with locked(root/'repair-reservations.json'):
        return _prepare_repairs(root,target,max_cells,max_claims)


def _prepare_repairs(root, target, max_cells, max_claims):
    if max_cells<1 or max_claims<1:raise ValueError('positive repair limits required')
    plan=json.loads((root/'plan.json').read_text());results=load_results(root)
    if any(t['kind']=='section_repair' for t in plan['tasks'].values()):
        raise ValueError('repair attempts cannot generate further repairs')
    if target.exists():raise ValueError('repair directory already exists')
    receipt=root/'repair-reservations.json'
    reserved=json.loads(receipt.read_text()) if receipt.exists() else {}
    tasks={};cells=[]
    # One bounded repair per original cell; no edits to valid proposals.
    for cell in plan['cells']:
        if len(cells)>=max_cells:break
        if cell['id'] in reserved:continue
        result=results.get(cell['id'],{});recovery=result.get('recovery',{})
        invalid=[r for r in recovery.get('claims',[]) if not r['schema_valid']][:max_claims]
        if not invalid:continue
        key=cell['id'];original=plan['tasks'][cell['task']]
        tasks[key]={**original,'kind':'section_repair','repair_targets':invalid,
                    'repair_of':{'plan_sha256':digest(plan),'cell':cell['id'],
                                 'result_sha256':digest(result),'attempt':1}}
        cells.append({**cell,'id':key+'-repair-1','task':key,'phase':'repairs'})
    target.mkdir(parents=True);shutil.copytree(root/'configs',target/'configs')
    shutil.copyfile(root/plan['reference_file'],target/'reference.json')
    import os
    new={**plan,'id':target.name,'tasks':tasks,'cells':cells,'reference_file':'reference.json',
         'budget_ledger':os.path.relpath((root/plan['budget_ledger']).resolve(),target.resolve()),
         'scope':'At most one repair per original section; valid original claims untouched. Still proposals only.'}
    atomic_json(target/'plan.json',new)
    for cell in cells:reserved[cell['task']]={'target':str(target.resolve()),'plan_sha256':digest(new)}
    atomic_json(receipt,reserved)
    return {'repair_cells':len(cells),'targets':sum(len(t['repair_targets']) for t in tasks.values())}


def main():
    p=argparse.ArgumentParser(description=__doc__);sub=p.add_subparsers(dest='command',required=True)
    a=sub.add_parser('prepare');a.add_argument('manifest',type=Path);a.add_argument('directory',type=Path)
    a=sub.add_parser('report');a.add_argument('directory',type=Path)
    a=sub.add_parser('repairs');a.add_argument('directory',type=Path);a.add_argument('target',type=Path)
    a.add_argument('--max-cells',type=int,default=3);a.add_argument('--max-claims',type=int,default=4)
    a=p.parse_args()
    result=prepare(a.manifest,a.directory) if a.command=='prepare' else report(a.directory) if a.command=='report' else prepare_repairs(a.directory,a.target,a.max_cells,a.max_claims)
    if a.command=='report':
        result={b:{'incomplete_cells':len(r['incomplete_cells']),'proposals':len(r['proposals']),
                   'sections':len(r['sections']),'repair_cells':len(r['repairs']),
                   'document_complete':r['document_complete']} for b,r in result['backends'].items()}
    print(json.dumps(result,indent=2))


if __name__=='__main__':main()
