"""Small language-neutral subprocess runner. No algorithm or scholarly policy lives here."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import uuid
from .artifacts import atomic_json, code_snapshot, digest, file_records, manifest, now, sha, verify
from .paths import DATA, REPO, resolve, selected

def run(definition):
    spec=json.loads(Path(definition).read_text())
    if spec.get('schema_version')!=1 or spec.get('mode') not in ('artifact','managed'):raise ValueError('unsupported workflow contract')
    run_id=uuid.uuid4().hex;run_dir=DATA/'runs'/run_id;run_dir.mkdir(parents=True)
    staging=run_dir/'staging';staging.mkdir()
    inputs={name:resolve(path) for name,path in spec.get('inputs',{}).items()}
    # Fingerprint full actual bytes, including dirty code, before invoking any process.
    input_hashes={k:sha(p) for k,p in inputs.items()}
    receipt={'schema_version':1,'run_id':run_id,'workflow':spec,'started_at':now(),'status':'running',
        'inputs':{k:{'path':str(v.relative_to(REPO)),'sha256':input_hashes[k]} for k,v in inputs.items()},
        'implementation':code_snapshot(),'python':sys.version,'platform':sys.platform,'commands':[]}
    try:
        receipt['git_revision']=subprocess.check_output(['git','rev-parse','HEAD'],cwd=REPO,text=True).strip()
        receipt['git_dirty']=bool(subprocess.check_output(['git','status','--porcelain'],cwd=REPO,text=True).strip())
    except subprocess.CalledProcessError:receipt['git_revision']=None
    for command in (['rustc','--version','--verbose'], ['cargo','--version']):
        result=subprocess.run(command,cwd=REPO,text=True,capture_output=True)
        receipt[command[0]]=result.stdout.strip() if result.returncode==0 else {'unavailable':True}
    atomic_json(run_dir/'receipt.json',receipt)
    variables={'repo':str(REPO),'python':sys.executable,'output':str(staging),**{'input_'+k:str(p) for k,p in inputs.items()}}
    try:
        # Only explicit argv arrays; never shell evaluation or implicit network acquisition.
        for i,command in enumerate(spec['commands']+spec.get('validation',[])):
            argv=[v.format_map(variables) for v in command]
            with (run_dir/f'{i:02}.log').open('wb') as log:
                proc=subprocess.run(argv,cwd=REPO,stdout=log,stderr=subprocess.STDOUT)
            receipt['commands'].append({'argv':argv,'exit_code':proc.returncode,'log':f'{i:02}.log'})
            if proc.returncode:raise RuntimeError(f'command {i} failed ({proc.returncode}); see {run_dir}/{i:02}.log')
        if input_hashes!={k:sha(p) for k,p in inputs.items()}:raise ValueError('inputs changed during execution')
        if receipt['implementation']!=code_snapshot():raise ValueError('implementation changed during execution')
        if spec['mode']=='artifact':
            for name in spec['required_outputs']:
                p=(staging/name).resolve()
                if not p.is_relative_to(staging.resolve()) or not p.is_file() or p.stat().st_size==0:raise ValueError('required output missing: '+name)
            artifact=run_dir/'artifact';artifact.mkdir();staging.rename(artifact/'files')
            m=manifest(artifact,spec['collection'],'transformed',{'run':str((run_dir/'receipt.json').relative_to(REPO))},parents=list(input_hashes.values()))
            atomic_json(artifact/'manifest.json',m)
            dest=DATA/'artifacts'/spec['collection']/m['artifact_id'].split(':')[1]
            dest.parent.mkdir(parents=True,exist_ok=True)
            if dest.exists():
                if verify(dest):raise ValueError('existing artifact corrupted')
            else:artifact.rename(dest)
            receipt['artifact']=str(dest.relative_to(REPO))
        else:
            # Domain registries own their validated atomic CURRENT transaction.
            receipt['outputs']={p:sha(resolve(p)) for p in spec['required_outputs']}
        receipt['status']='complete'
        if spec.get('product'):
            product=spec['product']
            if not __import__('re').fullmatch(r'[a-z0-9_-]+',product):raise ValueError('invalid product name')
            atomic_json(DATA/'products'/product/'CURRENT.json',{'schema_version':1,'artifact':receipt['artifact'],'run_id':run_id,'entrypoint':spec.get('entrypoint')})
    except Exception as exc:
        receipt['status']='failed';receipt['error']=str(exc);raise
    finally:
        receipt['completed_at']=now();atomic_json(run_dir/'receipt.json',receipt)
    return receipt
