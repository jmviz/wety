"""Plan bounded cross-backend semantic review of compact section proposals."""
import argparse
import json
import os
from pathlib import Path
import shutil
from .io_utils import atomic_json,digest,locked
from .section_extraction import load_results


def prepare_reviews(root,target,max_batches=4,reviewers=None):
    if max_batches<1:raise ValueError('positive batch limit required')
    reviewers=reviewers or {'openrouter':'claude-agent','claude-agent':'codex','codex':'openrouter'}
    backends={'openrouter','claude-agent','codex'}
    if set(reviewers)!=backends or any(v not in backends or k==v for k,v in reviewers.items()):
        raise ValueError('review routing must assign every backend to a different supported backend')
    with locked(root/'review-reservations.json'):
        plan=json.loads((root/'plan.json').read_text());results=load_results(root)
        if any(t['kind'] not in ('compact_extract','compact_repair') for t in plan['tasks'].values()):raise ValueError('review planner requires compact extraction or native repair tasks only')
        if target.exists():raise ValueError('review directory already exists')
        reservations=root/'review-reservations.json';reserved=json.loads(reservations.read_text()) if reservations.exists() else {}
        tasks={};cells=[];counts={};coverage=[]
        for cell in plan['cells']:
            result=results.get(cell['id'],{});reviewer=reviewers[cell['backend']]
            recovery=result.get('native_repair',{}) if plan['tasks'][cell['task']]['kind']=='compact_repair' else result.get('recovery',{})
            claims=[r for r in recovery.get('claims',[]) if r['schema_valid']]
            if 'native_repair' in result and not recovery.get('aligned'):claims=[]
            status='scheduled'
            if cell['id'] in reserved:status='already_reserved'
            elif not claims:status='no_valid_proposals'
            elif counts.get(reviewer,0)>=max_batches:status='review_budget_omission'
            coverage.append({'cell':cell['id'],'reviewer':reviewer,'valid_proposals':len(claims),'status':status,'extraction_status':result.get('status','not_attempted')})
            if status!='scheduled':continue
            # Cap is explicit even when a provider returned >12 assertions.
            selected=claims[:12]
            coverage[-1]['omitted_assertions']=len(claims)-len(selected)
            task=plan['tasks'][cell['task']]
            targets=[{'assertion_id':digest({'plan':digest(plan),'cell':cell['id'],'result':digest(result),'index':r['index'],'raw_digest':r['raw_digest']}),
                      'index':r['index'],'raw_digest':r['raw_digest'],'assertion':r['proposal'],
                      'mechanically_ready':r.get('mechanically_ready',False),'primary_evidence':r.get('primary_evidence',False),
                      **({'repair_target_id':r['repair_target_id'],'original_raw_digest':r['original_raw_digest']} if 'repair_target_id' in r else {})} for r in selected]
            key=cell['id'];tasks[key]={'kind':'semantic_review','family':task['family'],'packet':task['packet'],'targets':targets,
                                     'review_of':{'plan_sha256':digest(plan),'cell':key,'result_sha256':digest(result)}}
            cells.append({'id':reviewer+'-review-'+key,'backend':reviewer,'config':reviewer+'.json','task':key,
                          'role':'synthesize','phase':'reviews','tier':'cross-backend-review'})
            counts[reviewer]=counts.get(reviewer,0)+1
        target.mkdir(parents=True);shutil.copytree(root/'configs',target/'configs');shutil.copyfile(root/plan['reference_file'],target/'reference.json')
        new={**plan,'id':target.name,'tasks':tasks,'cells':cells,'reference_file':'reference.json',
             'budget_ledger':os.path.relpath((root/plan['budget_ledger']).resolve(),target.resolve()),
             'scope':'Single bounded cross-backend review of extracted proposals. Different reviewers see different proposals; not a controlled reviewer-quality ranking.'}
        atomic_json(target/'plan.json',new);atomic_json(target/'coverage.json',coverage)
        for cell in cells:reserved[cell['task']]={'plan_sha256':digest(new),'target':str(target.resolve())}
        atomic_json(reservations,reserved)
        return {'review_cells':len(cells),'assertions':sum(len(t['targets']) for t in tasks.values()),'coverage':coverage}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('directory',type=Path);p.add_argument('target',type=Path);p.add_argument('--max-batches',type=int,default=4)
    p.add_argument('--routing',type=Path,help='Explicit backend-to-reviewer JSON mapping; does not reset budget or failure circuit')
    a=p.parse_args();print(json.dumps(prepare_reviews(a.directory,a.target,a.max_batches,json.loads(a.routing.read_text()) if a.routing else None),indent=2))

if __name__=='__main__':main()
