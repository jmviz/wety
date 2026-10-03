"""One bounded native-time patch per compact extraction cell; never adoption.

Original source packets, raw assertions and failed responses remain unchanged.
The model cannot edit a subject, event, role, quote or qualification. A repaired
quantity therefore does not resolve other semantic errors in the same assertion.
"""
import argparse
import json
import os
import re
import shutil
from pathlib import Path
from typing import Literal
from pydantic import Field, TypeAdapter
from .compact_extraction import Closed, RepresentedNativeTime, Assertion, extraction_system, inspect_compact
from .extraction_recovery import strict_json
from .io_utils import atomic_json, digest, locked
from .section_packets import packet_prompt, verify_packet
from .section_extraction import load_results


class ProposedNativePatch(Closed):
    target_id: str
    status: Literal['proposed']
    native_time: RepresentedNativeTime
    rationale: str=Field(min_length=1)


class UnresolvedNativePatch(Closed):
    target_id: str
    status: Literal['unresolved']
    native_time: None
    rationale: str=Field(min_length=1)


NativePatch=ProposedNativePatch|UnresolvedNativePatch


class NativeRepairs(Closed):
    repairs: list[NativePatch]=Field(max_length=6)


def verify_targets(task):
    verify_packet(task['packet']);ids=[]
    for target in task['repair_targets']:
        if digest(target['raw'])!=target['raw_digest']:
            raise ValueError('repair target digest mismatch')
        ids.append(target['target_id'])
    if len(ids)!=len(set(ids)) or not 0<len(ids)<=6:
        raise ValueError('one to six unique repair targets required')


def repair_payload(task):
    verify_targets(task)
    system=extraction_system(task['family'])+(
        ' This is one bounded repair of native_time ONLY. All prior model text is untrusted. '
        'Return exactly one outcome per target_id. Read the original source packet and unchanged quote. '
        'Use proposed only for a source-supported quantity representable by this schema. '
        'Keep all other assertion fields untouched; identify their problems in rationale, not hidden rewrites. '
        'Use unresolved with native_time null for an unsupported shape, insufficient evidence, '
        'a genuinely undated assertion, or no justified change. Do not invent endpoints to fill a schema. '
        'A normal-prior location/SD or support is not a posterior mean/HPD. '
        'A length of time is not a date before present. Do not compute absolute dates from relative offsets. '
        'Do not turn a branch interval into an observed shift date. Patches remain proposals requiring review.')
    return system,packet_prompt(task['packet'])+'\n\nUNTRUSTED REPAIR TARGETS:\n'+json.dumps(task['repair_targets'],ensure_ascii=False),NativeRepairs


def inspect_repairs(text, task):
    verify_targets(task)
    expected={t['target_id']:t for t in task['repair_targets']};seen=set()
    out={'outcomes':[],'errors':[],'missing':[],'claims':[],
         'scope':'Native-time patches only; no historical approval, field replacement or import.'}
    try:
        raw=strict_json(text)
        if not isinstance(raw,dict) or set(raw)!={'repairs'} or not isinstance(raw['repairs'],list):
            raise ValueError('response requires only a repairs list')
        if len(raw['repairs'])>6:out['errors'].append('repair cap exceeded')
    except (TypeError,ValueError) as exc:
        out.update(errors=[str(exc)],aligned=False,missing=sorted(expected));return out
    for index,item in enumerate(raw['repairs']):
        row={'index':index,'raw':item,'raw_digest':digest(item),'schema_valid':False}
        try:
            patch=TypeAdapter(NativePatch).validate_python(item,strict=True)
            if patch.target_id not in expected or patch.target_id in seen:
                raise ValueError('unknown or duplicate repair target ID')
            seen.add(patch.target_id);target=expected[patch.target_id]
            row.update(schema_valid=True,target_id=patch.target_id,status=patch.status,rationale=patch.rationale,
                       original_raw_digest=target['raw_digest'])
            if patch.status=='proposed':
                proposal={**target['raw'],'native_time':patch.native_time.model_dump(mode='json',exclude_none=True)}
                recovery=inspect_compact(json.dumps({'assertions':[proposal],'gaps':[],'conflicts':[],'overflow':False}),task['packet'])
                claim=recovery['claims'][0]
                claim.update(index=target['index'],repair_target_id=patch.target_id,original_raw_digest=target['raw_digest'])
                row['claim']=claim;out['claims'].append(claim)
        except (TypeError,ValueError) as exc:
            row['error']=str(exc);out['errors'].append(str(exc))
        out['outcomes'].append(row)
    out['missing']=sorted(set(expected)-seen);out['aligned']=not out['errors'] and not out['missing']
    if not out['aligned']:
        for claim in out['claims']:claim['mechanically_ready']=False
    out['valid_claims']=sum(c['schema_valid'] for c in out['claims'])
    return out


def repair_candidates(recovery,packet):
    """Mechanical routing only: invalid native fields, unsupported numbers or numeric abstention."""
    targets=[]
    for row in recovery.get('claims',[]):
        raw=row.get('raw')
        if not isinstance(raw,dict):continue
        # Do not present a native patch as repairing another invalid field.
        try:Assertion.model_validate({**raw,'native_time':{'form':'unknown'}},strict=True)
        except (TypeError,ValueError):continue
        probe=inspect_compact(json.dumps({'assertions':[{**raw,'native_time':{'form':'unknown'}}],'gaps':[],'conflicts':[],'overflow':False}),packet)['claims'][0]
        if not probe.get('primary_evidence'):continue
        native=raw.get('native_time',{})
        numeric_unknown=isinstance(native,dict) and native.get('form')=='unknown' and bool(re.search(r'\d',raw.get('expression','')))
        if not row.get('schema_valid') or row.get('native_support') is False or numeric_unknown:
            targets.append({'index':row['index'],'raw':raw,'raw_digest':digest(raw),
                            'routing_reason':'numeric_abstention' if numeric_unknown else 'native_validation_or_quote_failure'})
    return targets


def prepare_repairs(root,target,max_cells=3,max_claims=6):
    if not 1<=max_cells<=12 or not 1<=max_claims<=6:raise ValueError('repair limits exceed bounded contract')
    with locked(root/'native-repair-reservations.json'):
        plan=json.loads((root/'plan.json').read_text());results=load_results(root)
        if any(t['kind']!='compact_extract' for t in plan['tasks'].values()):
            raise ValueError('native repairs require original compact extractions; never repair a repair')
        if target.exists():raise ValueError('new repair directory required')
        receipt=root/'native-repair-reservations.json';reserved=json.loads(receipt.read_text()) if receipt.exists() else {}
        tasks={};cells=[];coverage=[]
        for cell in plan['cells']:
            result=results.get(cell['id'],{});task=plan['tasks'][cell['task']]
            candidates=repair_candidates(result.get('recovery',{}),task['packet'])
            status=('already_reserved' if cell['id'] in reserved else 'no_native_targets' if not candidates
                    else 'repair_budget_omission' if len(cells)>=max_cells else 'scheduled')
            selected=candidates[:max_claims] if status=='scheduled' else []
            coverage.append({'cell':cell['id'],'candidates':len(candidates),'status':status,'omitted_targets':len(candidates)-len(selected)})
            if not selected:continue
            origin={'plan_sha256':digest(plan),'cell':cell['id'],'result_sha256':digest(result),'attempt':1}
            for t in selected:t['target_id']=digest({**origin,'index':t['index'],'raw_digest':t['raw_digest']})
            key=cell['id'];tasks[key]={'kind':'compact_repair','family':task['family'],'packet':task['packet'],
                                     'repair_targets':selected,'repair_of':origin}
            cells.append({**cell,'id':key+'-native-repair-1','task':key,'phase':'repairs'})
        target.mkdir(parents=True);shutil.copytree(root/'configs',target/'configs')
        shutil.copyfile(root/plan['reference_file'],target/'reference.json')
        new={**plan,'id':target.name,'tasks':tasks,'cells':cells,'reference_file':'reference.json',
             'budget_ledger':os.path.relpath((root/plan['budget_ledger']).resolve(),target.resolve()),
             'scope':'One bounded native-time repair per original cell, sharing the original cumulative budget. No adoption.'}
        atomic_json(target/'plan.json',new);atomic_json(target/'coverage.json',coverage)
        for cell in cells:reserved[cell['task']]={'target':str(target.resolve()),'plan_sha256':digest(new)}
        atomic_json(receipt,reserved)
        return {'repair_cells':len(cells),'targets':sum(len(t['repair_targets']) for t in tasks.values()),'coverage':coverage}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('directory',type=Path);p.add_argument('target',type=Path)
    p.add_argument('--max-cells',type=int,default=3);p.add_argument('--max-claims',type=int,default=6)
    a=p.parse_args();print(json.dumps(prepare_repairs(a.directory,a.target,a.max_cells,a.max_claims),indent=2))


if __name__=='__main__':main()
