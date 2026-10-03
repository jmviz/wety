"""Small, explicitly constructed transport controls; not linguistic evidence."""
import argparse
import copy
import hashlib
import json
from datetime import datetime,timedelta,timezone
from pathlib import Path
from .io_utils import atomic_json
from .evaluation_runtime import create_ledger
from .section_packets import build_packets

TEXT='''CONSTRUCTED SOFTWARE CONTROLS. These sentences are invented and are not historical evidence.
The Alpha inscription is assigned to approximately 1200 BCE; this dates the surviving object, not the language's origin.
For this analysis present is 2000 CE. The sampled Beta clade has a posterior mean crown age of 4500 calendar years ago; no interval is supplied.
The earliest Gamma inscription is assigned to the middle of the fifth century CE; a precise year is not known.
The Delta divergence calibration uses a normal prior centred on 1000 calendar years before 2000, standard deviation 50 years, truncated to 800–1200 years. These limits are prior support, not an HPD interval.
The Epsilon borrowing occurred after the donor innovation and before the recipient sound change; neither event has an absolute date here.'''


def prepare(root,config_dir):
    if root.exists():raise ValueError('use a new control directory; never reset an existing ledger')
    root.mkdir(parents=True);(root/'configs').mkdir()
    for backend in ('openrouter','claude-agent','codex'):
        cfg=json.loads((config_dir/f'{backend}.json').read_text())
        atomic_json(root/'configs'/f'{backend}.json',cfg)
        if backend=='openrouter':
            alt=copy.deepcopy(cfg)
            alt['roles']['extract']['backends']['openrouter']['schema_profile']='minimal'
            atomic_json(root/'configs'/'openrouter-minimal.json',alt)
    reference={'scope':'Invented software controls, exposed development only. Exact labels are specified to avoid entity matching ambiguity.',
               'text':TEXT,'subjects':['Alpha inscription','sampled Beta clade','earliest Gamma inscription','Delta divergence','Epsilon borrowing'],
               'checks':['positive native BCE1200','posterior mean4500 years_ago present2000 calendar','enclosing century_CE5 range with middle qualifier','normal prior centre1000 SD50 support800–1200; not HPD','relative-only contact; no absolute date'],
               'review_expected':{'alpha':'revise','beta':'revise','gamma':'supported','delta':'revise','epsilon':'supported'}}
    atomic_json(root/'reference.json',reference)
    packet=build_packets(TEXT,'constructed-compact-controls')['packets'][0]
    task={'kind':'compact_extract','family':'Constructed controls. Use these exact literal subject labels: '+', '.join(reference['subjects']), 'packet':packet}
    def target(i,subject,event,role,native,quote,**kw):
        a={'subject':subject,'subject_type':'documentary_sample' if i in ('alpha','gamma') else 'sampled_clade' if i in ('beta','delta') else 'language_variety',
           'event':event,'role':role,'method':'Constructed supplied statement','native_time':native,'expression':quote,'quote':quote,'locator':'constructed control',
           'scenario':None,'reported_from':None,'qualifications':[],'relative_constraints':[],'sampled_taxa':[],**kw}
        return {'assertion_id':i,'assertion':a,'mechanically_ready':i!='alpha','primary_evidence':True}
    lines=TEXT.splitlines()
    targets=[target('alpha','Alpha inscription','unspecified','observation',{'form':'point','unit':'BCE','value':-1199},lines[1]),
             target('beta','sampled Beta clade','crown_divergence','posterior',{'form':'unknown'},lines[2]),
             target('gamma','earliest Gamma inscription','first_attestation','observation',{'form':'event_range','unit':'century_CE','lower':5,'upper':5,'precision':'century','age_scale':'calendar'},lines[3],qualifications=['An enclosing fifth-century range; the source narrows this to the middle, without a precise boundary.']),
             # Deliberate scope error: source says divergence, not a sampled crown.
             target('delta','Delta divergence','crown_divergence','calibration_prior',{'form':'distribution','unit':'years_ago','mean':1000,'present':2000,'age_scale':'calendar'},lines[4],qualifications=['Normal prior; standard deviation50; support truncated to800–1200 years; support is not a posterior interval.']),
             target('epsilon','Epsilon borrowing','contact','observation',{'form':'unknown'},lines[5],relative_constraints=['after donor innovation','before recipient sound change'])]
    tasks={'extract':task,'review':{'kind':'semantic_review','family':'Constructed controls','packet':packet,'targets':targets}}
    cells=[{'id':b+'-extract','backend':b,'config':b+'.json','task':'extract','role':'extract','phase':'baseline'} for b in ('openrouter','claude-agent','codex')]
    cells.append({'id':'openrouter-minimal','backend':'openrouter','config':'openrouter-minimal.json','task':'extract','role':'extract','phase':'wire-alternative'})
    cells += [{'id':b+'-review','backend':b,'config':b+'.json','task':'review','role':'synthesize','phase':'reviews'} for b in ('openrouter','claude-agent','codex')]
    create_ledger(root/'budget.json',{'max_calls':7,'cash_limit_usd':2,'reservation_usd':{'openrouter':0.5,'claude-agent':0,'codex':0},'max_consecutive_failures':2,'deadline_utc':(datetime.now(timezone.utc)+timedelta(minutes=25)).isoformat()})
    atomic_json(root/'plan.json',{'id':root.name,'split':'constructed_development','tasks':tasks,'cells':cells,'budget_ledger':'budget.json','budget':{'max_seconds_per_cell':150},'reference_file':'reference.json','reference_sha256':hashlib.sha256((root/'reference.json').read_bytes()).hexdigest(),'scope':'Canonical baseline, one explicit minimal-schema alternative only if canonical OpenRouter fails, then fixed identical reviewer controls. No historical imports.'})


def report(root):
    plan=json.loads((root/'plan.json').read_text());rows=[]
    for cell in plan['cells']:
        p=root/'results'/f'{cell["id"]}.json';d=json.loads(p.read_text()) if p.exists() else {'status':'not_attempted'}
        row={'cell':cell['id'],'status':d['status']};checks={}
        if cell['task']=='extract':
            cs=d.get('recovery',{}).get('claims',[])
            def one(label):
                found=[c for c in cs if c.get('proposal',{}).get('subject')==label]
                return found[0] if len(found)==1 else {}
            a=one('Alpha inscription');b=one('sampled Beta clade');g=one('earliest Gamma inscription');de=one('Delta divergence');e=one('Epsilon borrowing')
            def n(c):return c.get('proposal',{}).get('native_time',{})
            checks['positive_BCE']=n(a).get('unit')=='BCE' and n(a).get('value')==1200
            checks['defined_epoch_mean']=all(n(b).get(k)==v for k,v in {'form':'distribution','mean':4500,'unit':'years_ago','present':2000,'age_scale':'calendar'}.items())
            checks['coarse_range']=all(n(g).get(k)==v for k,v in {'form':'event_range','unit':'century_CE','lower':5,'upper':5}.items()) and 'middle' in json.dumps(g.get('proposal',{})).lower()
            checks['prior_centre']=de.get('proposal',{}).get('role')=='calibration_prior' and n(de).get('mean')==1000 and n(de).get('present')==2000 and n(de).get('interval_type')!='hpd'
            checks['relative_only']=e.get('proposal',{}).get('event')=='contact' and n(e).get('form')=='unknown' and len(e.get('proposal',{}).get('relative_constraints',[]))>=2
            checks['source_and_shape']=len(cs)==5 and all(c.get('mechanically_ready') and c.get('primary_evidence') for c in cs) and not d.get('recovery',{}).get('overflow',True)
        else:
            rev=d.get('semantic_review',{});js={j['assertion_id']:j for j in rev.get('judgments',[])}
            checks={k:js.get(k,{}).get('disposition')==v for k,v in json.loads((root/'reference.json').read_text())['review_expected'].items()}
            checks['aligned_source_quotes']=bool(rev.get('aligned')) and all(j['quote_spans'] for j in js.values())
        row.update(checks=checks,passed=d['status']=='completed' and all(checks.values()));rows.append(row)
    out={'scope':'Constructed controls and mechanical scoring only; inspect prior family/support prose and reviewer rationale separately. No full-source accuracy or model ranking.','rows':rows}
    atomic_json(root/'control-report.json',out);return out


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['prepare','report']);p.add_argument('directory',type=Path);p.add_argument('--configs',type=Path)
    a=p.parse_args()
    if a.mode=='prepare':prepare(a.directory,a.configs)
    else:print(json.dumps(report(a.directory),indent=2))

if __name__=='__main__':main()
