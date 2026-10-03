"""Verify the pinned embedding snapshot before a reproducible graph run."""
import json,os
from pathlib import Path
from .paths import DATA
from .artifacts import sha

def main():
    m=json.loads((DATA/'registry/embedding-model.json').read_text())
    root=Path(os.environ.get('HF_HOME',Path.home()/'.cache/huggingface'))/'hub'/('models--'+m['model'].replace('/','--'))/'snapshots'/m['revision']
    if not m['files']:raise ValueError('embedding model lock has no file hashes')
    for name,h in m['files'].items():
        p=root/name
        if not p.is_file() or sha(p)!=h:raise ValueError('missing or changed pinned model file: '+str(p))
    print('Pinned embedding model files verified')

if __name__=='__main__':main()
