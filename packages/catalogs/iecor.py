#!/usr/bin/env python3
"""Propose catalog context for IE-CoR samples; never merge sample identities.

Names and codes describe different scopes. No token, sole-regular-code or
nearest-ancestor heuristic can establish a sample's temporal or lexical scope.
All proposals require review. Historical priors remain on iecor:sample:<ID>.
"""
from __future__ import annotations
import csv
import json
from pathlib import Path
import re
import sys
import unicodedata
from collections import defaultdict, Counter
from workflow_tools.paths import REPO, IECOR_PRODUCT, selected
from . import registry as catalog_registry
HERE=IECOR_PRODUCT


def norm(s):
    # Keep non-Latin scripts and diacritics; punctuation normalization is only
    # candidate generation, never evidence of equivalence.
    return ' '.join(''.join(c if c.isalnum() else ' ' for c in unicodedata.normalize('NFC',s).casefold()).split())


def name_variants(name):
    out=[name]
    if ':' in name:
        a,b=name.split(':',1);out.extend([b.strip()+' '+a.strip(),a.strip()+' '+b.strip()])
    return out


def propose(row,bundle,names):
    candidates=defaultdict(set)
    for name in name_variants(row['Name']):
        for c in names.get(norm(name),[]):candidates[c].add('name')
    for field,index in [('Glottocode','index_by_glottocode'),('ISO639P3code','index_by_iso639_3')]:
        for c in bundle[index].get(row.get(field,''),[]):candidates[c].add(field)
    proposals=[]
    for c,reasons in sorted(candidates.items()):
        target=bundle['languages'][c]
        conflicts=[]
        for field,targetfield in [('Glottocode','glottocode'),('ISO639P3code','iso639_3')]:
            if row.get(field) and target.get(targetfield) and row[field]!=target[targetfield]:conflicts.append(field+' differs')
        proposals.append({'target':target['id'],'name':target['name'],'basis':sorted(reasons),'conflicts':conflicts,
            'relation':'unresolved','status':'proposed','temporal_transfer_allowed':False})
    return {'schema_version':2,'sample_id':'iecor:sample:'+row['ID'],'iecor':row,
        'match':{'route':None,'wiktionary_code':None,'status':'needs_review' if proposals else 'unresolved',
                 'relation':'unresolved','proposals':proposals,'temporal_transfer_allowed':False},'wiktionary':None}


def verify_receipt():
    receipt=json.loads((HERE/'iecor_matches_receipt.json').read_text())
    for name,h in receipt['files'].items():
        if catalog_registry.sha(REPO/name)!=h:raise ValueError('stale IE-CoR matching input/output: '+name)
    if catalog_registry.digest(catalog_registry.current_bundle())!=receipt['crossref_generation']:raise ValueError('IE-CoR proposals use stale catalog generation')
    return receipt


def main():
    errors=catalog_registry.validate_current()
    if errors:raise ValueError('\n'.join(errors))
    b=catalog_registry.current_bundle();raw=json.loads((selected('wiktionary_languages')).read_text());names=defaultdict(set)
    for r in raw.values():
        for n in [r['canonicalName'],*r.get('aliases',[])]:names[norm(n)].add(r['mainCode'])
    with (selected('iecor_languages')).open(newline='') as f:rows=[{k:v.strip() for k,v in r.items() if v and v.strip()} for r in csv.DictReader(f)]
    if len({r['ID'] for r in rows})!=len(rows):raise ValueError('duplicate IE-CoR sample ID')
    results=[propose(r,b,names) for r in sorted(rows,key=lambda r:r['ID'])]
    output=HERE/'iecor_wiktionary_matches.json'
    catalog_registry.atomic(output,catalog_registry.json_bytes(results))
    receipt={'schema_version':2,'crossref_generation':catalog_registry.digest(b),'files':{str(p.relative_to(REPO)):catalog_registry.sha(p) for p in (selected('iecor_languages'),selected('wiktionary_languages'),output)},'limitations':['Names and source codes only generate proposals; no sample-to-stage equivalence has been adjudicated.','Original IE-CoR table download/revision receipt unavailable in legacy input.']}
    catalog_registry.atomic(HERE/'iecor_matches_receipt.json',catalog_registry.json_bytes(receipt))
    print(f'{len(results)} source samples retained; '+str(dict(Counter(r['match']['status'] for r in results))))


if __name__=='__main__':main()
