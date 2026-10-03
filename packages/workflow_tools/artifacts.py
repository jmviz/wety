"""Content manifests and immutable adoption. A receipt is evidence, not certification."""
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import tempfile
from .paths import REPO, DATA, resolve

def now():return datetime.now(timezone.utc).isoformat()

def sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as f:
        for chunk in iter(lambda:f.read(8*1024*1024),b''):h.update(chunk)
    return h.hexdigest()

def digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()

def atomic_json(path,value):
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    fd,tmp=tempfile.mkstemp(dir=path.parent)
    try:
        with os.fdopen(fd,'w') as f:
            json.dump(value,f,indent=2,ensure_ascii=False,sort_keys=True);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,path)
    finally:
        if os.path.exists(tmp):os.unlink(tmp)

def file_records(folder):
    folder=Path(folder)
    result={}
    for p in sorted(folder.rglob('*')):
        if p.is_symlink():raise ValueError('artifact may not contain symlinks: '+str(p))
        if p.is_file():result[str(p.relative_to(folder))]={'sha256':sha(p),'bytes':p.stat().st_size}
    if not result:raise ValueError('empty artifact')
    return result

def manifest(folder,collection,role,provenance,parents=()):
    body={'schema_version':1,'collection':collection,'role':role,'files':file_records(Path(folder)/'files'),'parents':list(parents)}
    return {**body,'artifact_id':'sha256:'+digest(body),'provenance':provenance}

def verify(folder):
    folder=Path(folder);m=json.loads((folder/'manifest.json').read_text())
    body={k:m[k] for k in ('schema_version','collection','role','files','parents')}
    errors=[]
    if m['artifact_id']!='sha256:'+digest(body):errors.append('artifact identity mismatch')
    if file_records(folder/'files')!=m['files']:errors.append('artifact contents differ')
    return errors

def code_snapshot():
    paths=[]
    for base in ('packages','crates','workflows'):
        paths.extend(p for p in (REPO/base).rglob('*') if p.is_file() and '__pycache__' not in p.parts and p.suffix in ('.py','.rs','.toml','.lock','.json','.lua'))
    paths.extend(REPO/p for p in ('Cargo.toml','Cargo.lock','pyproject.toml','uv.lock','rust-toolchain.toml','.cargo/config.toml','data/registry/inputs.json'))
    return {str(p.relative_to(REPO)):sha(p) for p in sorted(set(paths)) if p.is_file()}

def adopt(collection,source,role,provenance,parents=()):
    """Copy a reviewed local file set into an immutable artifact, preserving originals."""
    import re,shutil
    if not re.fullmatch(r'[a-z0-9][a-z0-9_-]*',collection):raise ValueError('invalid collection name')
    if role not in ('source','transformed','editorial'):raise ValueError('invalid artifact role')
    source=Path(source)
    file_records(source)  # reject symlinks before copying
    stage_root=DATA/'cache/adoption';stage_root.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(dir=stage_root) as tmp:
        tmp=Path(tmp);shutil.copytree(source,tmp/'files')
        m=manifest(tmp,collection,role,provenance,parents)
        dest=DATA/'artifacts'/collection/m['artifact_id'].split(':')[1]
        if dest.exists():
            if verify(dest):raise ValueError('existing artifact corrupted')
        else:
            atomic_json(tmp/'manifest.json',m);dest.parent.mkdir(parents=True,exist_ok=True);os.rename(tmp,dest)
    return dest
