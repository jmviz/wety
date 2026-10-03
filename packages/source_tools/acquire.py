"""Acquire exact bytes. Refresh is explicit; expected hashes enable replay of a lock."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import tempfile
import urllib.request
import uuid
import zipfile
from workflow_tools.artifacts import atomic_json, manifest, now, sha, verify
from workflow_tools.paths import DATA, REPO

USER_AGENT='wety-sources/1.0 (https://github.com/jmviz/wety)'

def safe_extract_zip(archive,destination):
    destination=Path(destination).resolve()
    with zipfile.ZipFile(archive) as z:
        for info in z.infolist():
            target=(destination/info.filename).resolve()
            if not target.is_relative_to(destination) or info.filename.startswith('/') or '\\' in info.filename or (info.external_attr>>16)&0o170000==0o120000:
                raise ValueError('unsafe archive member '+info.filename)
        z.extractall(destination)

def acquire(url,collection,*,expected_sha256=None,version=None,license=None,headers=None):
    if not re.fullmatch(r'[a-z0-9][a-z0-9_-]*',collection):raise ValueError('invalid collection name')
    if not url.startswith('https://'):raise ValueError('HTTPS source URL required')
    cache=DATA/'cache/downloads';cache.mkdir(parents=True,exist_ok=True)
    started=now()
    with tempfile.TemporaryDirectory(dir=cache) as tmp:
        tmp=Path(tmp);(tmp/'files').mkdir();p=tmp/'files/source'
        request=urllib.request.Request(url,headers={'User-Agent':USER_AGENT,**(headers or {})})
        with urllib.request.urlopen(request,timeout=300) as response,p.open('wb') as f:
            shutil.copyfileobj(response,f,1024*1024)
            metadata={'requested_url':url,'final_url':response.geturl(),'started_at':started,'completed_at':now(),
                'http_status':response.status,'response_headers':{k:v for k,v in response.headers.items() if k.lower() in ('etag','last-modified','content-type','content-length')},
                'upstream_version':version,'license':license,'completeness':'complete HTTP response; source coverage unassessed',
                'request_headers':{k:v for k,v in request.header_items() if k.lower() in ('accept','user-agent')}}
        checksum=sha(p)
        if expected_sha256 and checksum!=expected_sha256:raise ValueError('download checksum does not match lock')
        m=manifest(tmp,collection,'source',metadata)
        dest=DATA/'artifacts'/collection/checksum
        if dest.exists():
            if verify(dest):raise ValueError('existing artifact corrupted')
        else:
            atomic_json(tmp/'manifest.json',m);dest.parent.mkdir(parents=True,exist_ok=True)
            os.rename(tmp,dest)
        # Every acquisition has its own receipt, even repeated downloads of identical bytes.
        receipt=DATA/'registry/acquisitions'/(uuid.uuid4().hex+'.json')
        atomic_json(receipt,{'schema_version':1,'artifact':str(dest.relative_to(REPO)),'artifact_id':m['artifact_id'],'sha256':checksum,**metadata,'acquirer_sha256':sha(__file__)})
        return dest

def fetch_bytes(url,collection='http-responses',**kwargs):
    return (acquire(url,collection,**kwargs)/'files/source').read_bytes()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('collection');parser.add_argument('url')
    parser.add_argument('--sha256');parser.add_argument('--version');parser.add_argument('--license')
    parser.add_argument('--extract-zip',type=Path,help='Explicit separate work directory; original archive is retained')
    args=parser.parse_args()
    dest=acquire(args.url,args.collection,expected_sha256=args.sha256,version=args.version,license=args.license)
    if args.extract_zip:safe_extract_zip(dest/'files/source',args.extract_zip)
    print(dest.relative_to(REPO))

if __name__=='__main__':main()
