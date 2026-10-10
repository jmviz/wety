import argparse,json
from pathlib import Path
from .paths import REPO, resolve

def main():
    parser=argparse.ArgumentParser(description='Wety artifact and workflow tools')
    sub=parser.add_subparsers(dest='command',required=True)
    r=sub.add_parser('run');r.add_argument('workflow',help='workflow name or repository-relative definition')
    v=sub.add_parser('verify');v.add_argument('artifact',help='repository-relative artifact directory')
    sub.add_parser('verify-all')
    s=sub.add_parser('store',help='the private research store (restic snapshots, sync between checkouts)')
    s.add_argument('action',choices=['status','snapshot','restore','sync','check'])
    s.add_argument('paths',nargs='*',help='sync: SRC DST data directories')
    s.add_argument('--data',help="snapshot: default the main checkout's data/")
    s.add_argument('--note');s.add_argument('--target',help='restore: an empty directory');s.add_argument('--snapshot',help='restore: default the pointer\'s')
    s.add_argument('--apply',action='store_true',help='sync: copy (default: report only)')
    a=sub.add_parser('adopt');a.add_argument('collection');a.add_argument('directory');a.add_argument('--role',choices=['source','transformed','editorial'],required=True);a.add_argument('--provenance',required=True,help='JSON file describing origin, limitations, and any input/run references')
    args=parser.parse_args()
    if args.command=='run':
        from .runner import run
        p=resolve(args.workflow) if '/' in args.workflow else REPO/'workflows'/args.workflow/'workflow.json'
        print(json.dumps(run(p),indent=2))
    elif args.command=='store':
        from . import store
        if args.action=='sync':
            if len(args.paths)!=2:parser.error('store sync SRC DST')
            out=store.sync(*args.paths,apply=args.apply)
        elif args.action=='snapshot':out=store.snapshot(args.data,args.note)
        elif args.action=='restore':
            if not args.target:parser.error('store restore --target DIR')
            out=store.restore(args.target,args.snapshot)
        else:out=getattr(store,args.action)()
        print(json.dumps(out,indent=2))
        if args.action=='sync' and out['conflicts']:raise SystemExit(1)
    elif args.command=='adopt':
        from .artifacts import adopt
        print(adopt(args.collection,resolve(args.directory),args.role,json.loads(resolve(args.provenance).read_text())).relative_to(REPO))
    else:
        from .artifacts import verify
        paths=[resolve(args.artifact)] if args.command=='verify' else [p.parent for p in (REPO/'data/artifacts').glob('*/*/manifest.json') if (p.parent/'files').is_dir()]
        errors={str(p.relative_to(REPO)):verify(p) for p in paths};errors={k:v for k,v in errors.items() if v}
        print(json.dumps({'verified':len(paths),'errors':errors},indent=2));raise SystemExit(bool(errors))

if __name__=='__main__':main()
