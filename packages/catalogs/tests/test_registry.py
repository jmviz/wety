"""Offline safety and regression checks, not an accuracy benchmark."""
import copy
import csv
import importlib.util
import io
import json
from pathlib import Path
import sys
import unittest
from workflow_tools.paths import REPO, CATALOG_CODE, CATALOG_PRODUCT
ROOT=CATALOG_PRODUCT
from catalogs import build, registry as r, iecor


def inputs():
    iso={'iso3':{},'retired':{},'iso5':{},'iso2':{},'macro_members':{},'member_of':{},'iso1_to_3':{}}
    gl={'langs':{'aaaa1234':{'Name':'A','Level':'language','category':'Spoken_L1_Language'},'bbbb1234':{'Name':'B','Level':'language','category':'Spoken_L1_Language'}}}
    wd={'items':{'Q1':{'glottocode':['aaaa1234']} }}
    raw={'canonicalName':'A','kind':'regular','wikidataItem':'Q1'}
    rec={'code':'aaa','glottocode':'aaaa1234','glottocode_route':'iso+wikidata'}
    return rec,raw,iso,gl,wd


def converted(mutate=lambda *x:None):
    values=inputs();mutate(*values)
    rec,raw,iso,gl,wd=values
    return r.project(r.convert_record(rec,raw,False,iso,gl,wd,'snapshot'))


class MappingPolicy(unittest.TestCase):
    def test_catalog_agreement_is_not_equivalence(self):
        x=converted();self.assertEqual(x['glottocode'],'aaaa1234');self.assertNotIn('confidence',x)
        self.assertTrue(all(e['relation']!='equivalent' for e in x['links']))
    def test_conflicting_identifiers_abstain(self):
        x=converted(lambda rec,raw,iso,gl,wd:wd['items']['Q1'].update(glottocode=['bbbb1234']))
        self.assertNotIn('glottocode',x);self.assertEqual(len([e for e in x['links'] if e['status']=='proposed']),2)
    def test_upstream_iso_conflict_cannot_create_glottocode(self):
        x=converted(lambda rec,*_:rec.update(conflicts=['iso639_3: contradictory scope']))
        self.assertNotIn('glottocode',x)
    def test_multiple_values_abstain_even_with_agreement(self):
        x=converted(lambda rec,raw,iso,gl,wd:wd['items']['Q1'].update(glottocode=['aaaa1234','bbbb1234']))
        self.assertNotIn('glottocode',x)
    def test_name_only_is_proposal(self):
        x=converted(lambda rec,*_:rec.update(glottocode_route='name'))
        self.assertNotIn('glottocode',x)
    def test_bookkeeping_never_projects(self):
        x=converted(lambda rec,raw,iso,gl,wd:gl['langs']['aaaa1234'].update(category='Bookkeeping'))
        self.assertNotIn('glottocode',x)
    def test_proto_not_family_identity(self):
        def mutate(rec,raw,iso,gl,wd):raw['kind']='reconstructed';rec['_proto_families']=['fam1','fam2']
        x=converted(mutate);self.assertNotIn('glottocode',x)
        self.assertEqual({e['target'] for e in x['links'] if e['target'].startswith('wikt:family:')},{'wikt:family:fam1','wikt:family:fam2'})
    def test_parent_scope_cannot_supply_child_identifier(self):
        x=converted(lambda rec,raw,*_: (rec.update(_wd_is_parents=True),raw.update(nonEtymologyOnly='parent')))
        self.assertNotIn('glottocode',x);self.assertIn('wikt:lang:parent',[e['target'] for e in x['links']])
    def test_family_target_is_context_for_language(self):
        x=converted(lambda rec,raw,iso,gl,wd:gl['langs']['aaaa1234'].update(Level='family'))
        self.assertNotIn('glottocode',x)
    def test_invalid_target_preserved_as_proposal(self):
        x=converted(lambda rec,raw,iso,gl,wd:gl['langs'].pop('aaaa1234'))
        self.assertNotIn('glottocode',x);self.assertTrue(any(e['target']=='glottolog:aaaa1234' for e in x['links']))
    def test_retirement_is_not_extinction_or_identity(self):
        def mutate(rec,raw,iso,gl,wd):
            rec.update(iso639_3='aaa',iso639_3_route='code',iso639_3_status='retired')
            iso['retired']['aaa']={'Ref_Name':'A','Ret_Reason':'S','Change_To':'','Ret_Remedy':'Split into [bbb], [ccc]','Effective':'2000-01-01'}
        x=converted(mutate);self.assertNotIn('iso639_3',x);self.assertNotIn('glottocode',x)
        edge=next(e for e in x['links'] if e['target']=='iso639-3:aaa')
        self.assertEqual(edge['target_metadata']['retirement_successors'],['bbb','ccc'])
        self.assertEqual(edge['id'],'link:'+r.digest({k:v for k,v in edge.items() if k!='id'}))
    def test_classification_cycles_and_inconsistent_prefixes_fail(self):
        build.validate_classification({'a':{},'b':{'classification':'a'}})
        with self.assertRaisesRegex(ValueError,'classification'):
            build.validate_classification({'a':{'classification':'b'},'b':{'classification':'a'}})
        with self.assertRaisesRegex(ValueError,'classification'):
            build.validate_classification({'a':{},'b':{'classification':'missing'}})
    def test_redirect_chain_and_cycle(self):
        wd={'items':{'Q1':{'redirects_to':'Q2'},'Q2':{'redirects_to':'Q3'},'Q3':{'label':'A'}}}
        self.assertEqual(build.wd_item(wd,'Q1'),('Q3',{'label':'A'}))
        wd['items']['Q3']={'redirects_to':'Q1'}
        with self.assertRaisesRegex(ValueError,'cycle'):build.wd_item(wd,'Q1')
    def test_stale_and_unrestricted_overrides_rejected(self):
        x=converted();_,_,iso,gl,_=inputs()
        decision={'target':'glottolog:aaaa1234','relation':'catalog_correspondence','reviewer':'test','rationale':'bounded test','references':['fixture'],'source_snapshot':'old','purpose':'catalog lookup'}
        with self.assertRaisesRegex(ValueError,'stale'):r.apply_decisions({x['id']:x},{x['id']:decision},'snapshot',gl,iso)
        decision.update(source_snapshot='snapshot',confidence='high')
        with self.assertRaisesRegex(ValueError,'fields'):r.apply_decisions({x['id']:x},{x['id']:decision},'snapshot',gl,iso)
    def test_reviewed_abstention_overrides_automatic_lookup(self):
        x=converted();_,_,iso,gl,_=inputs()
        decision={'target':'glottolog:aaaa1234','relation':'unresolved','reviewer':'test','rationale':'scope unclear','references':['fixture'],'source_snapshot':'snapshot','purpose':'historical stage lookup'}
        r.apply_decisions({x['id']:x},{x['id']:decision},'snapshot',gl,iso)
        self.assertNotIn('glottocode',r.project(x))
    def test_scope_reviews_keep_lookup_and_allow_several_targets(self):
        x=converted();_,_,iso,gl,_=inputs();iso['iso3']['aaa']={}
        base={'reviewer':'test','rationale':'bounded test','references':['fixture'],'source_snapshot':'snapshot','purpose':'dating stage scope'}
        decisions=[{**base,'target':'glottolog:aaaa1234','relation':'source_broader'},{**base,'target':'iso639-3:aaa','relation':'equivalent'}]
        r.apply_decisions({x['id']:x},{x['id']:decisions},'snapshot',gl,iso)
        r.project(x)
        # The broader-scope review documents extension without withdrawing the lookup.
        self.assertEqual((x['glottocode'],x['glottocode_route']),('aaaa1234','catalog_link'))
        self.assertEqual((x['iso639_3'],x['iso639_3_route']),('aaa','reviewed'))
        self.assertEqual({e['relation'] for e in x['links'] if e['status']=='reviewed'},{'source_broader','equivalent'})
        self.assertEqual(x['equivalence_status'],'reviewed_for_purpose')
        with self.assertRaisesRegex(ValueError,'duplicate'):r.apply_decisions({x['id']:x},{x['id']:[decisions[0],decisions[0]]},'snapshot',gl,iso)
    def test_family_disagreement_does_not_delete_name_candidate(self):
        _,raw,iso,gl,wd=inputs()
        raw.update(nonEtymologyOnly='x-stage',family='fam',wikidataItem=None)
        gl.update(iso_to_glotto={},names_lang={build.norm_name('A'):{'aaaa1234'}})
        gl['langs']['aaaa1234'].update({k:'' for k in ('Family_ID','Language_ID','ISO639P3code','Closest_ISO369P3code','First_Year_Of_Documentation','Last_Year_Of_Documentation')})
        for family in ('aaaa1234','bbbb1234'):
            result=build.match_language('x-stage',raw,{},iso,gl,wd,{}, {'fam':{'glottocode':family}})
            self.assertEqual(result['alternatives']['glottocode_name_candidates'],['aaaa1234'])
            self.assertEqual(result['glottocode_route'],'name')
    def test_projection_clears_stale_scalar(self):
        x=converted();x['links']=[];r.project(x);self.assertNotIn('glottocode',x)
    def test_unicode_names_do_not_collapse_to_empty(self):
        self.assertNotEqual(iecor.norm('汉语'),iecor.norm('日本語'))
        self.assertEqual(iecor.norm('é'),iecor.norm('e\u0301'))
    def test_unique_code_does_not_override_sample_name(self):
        b={'languages':{'oco':{'id':'wikt:lang:oco','name':'Old Cornish','glottocode':'oldc1252'}},'index_by_glottocode':{'oldc1252':['oco']},'index_by_iso639_3':{}}
        result=iecor.propose({'ID':'230','Name':'Late Cornish','Glottocode':'oldc1252'},b,{})
        self.assertIsNone(result['match']['wiktionary_code']);self.assertEqual(result['sample_id'],'iecor:sample:230')
    def test_alias_namespace_collision_rejected(self):
        with self.assertRaisesRegex(ValueError,'ambiguous'):r.alias_index({'a':{'id':'wikt:lang:a','wiktionary_alias_codes':['b']},'b':{'id':'wikt:lang:b','wiktionary_alias_codes':[]}})


class PublishedRegression(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.bundle=r.current_bundle()
    def test_committed_bundle_and_mirrors(self):self.assertEqual(r.validate_current(),[])
    def test_coverage_is_not_confidence(self):
        import json
        from workflow_tools.paths import selected
        languages=json.loads(selected('wiktionary_languages').read_text())
        # One catalog row per Wiktionary main code (aliases resolve to their main code).
        self.assertEqual(len(self.bundle['languages']),sum(1 for c,e in languages.items() if e['mainCode']==c))
        for row in self.bundle['languages'].values():
            self.assertNotIn('confidence',row);self.assertNotIn('glottolog_first_year',row);self.assertNotIn('inherited_from',row)
            self.assertFalse(row['temporal_transfer_allowed'])
    def test_aramaic_conflict_not_preferred(self):
        row=self.bundle['languages']['arc'];self.assertNotIn('glottocode',row)
        self.assertTrue({'glottolog:impe1235','glottolog:aram1259'}<={e['target'] for e in row['links']})
    def test_uralic_aliases_not_proto_dates(self):
        for code in ('urj-ugr-pro','urj-fpr-pro','urj-pro'):
            row=self.bundle['languages'][code];self.assertNotIn('glottocode',row);self.assertNotIn('glottolog_family_glottocode',row)
    def test_umbrella_codes_not_forced_into_language_nodes(self):
        self.assertNotIn('glottocode',self.bundle['languages']['ar'])
        self.assertNotIn('glottocode',self.bundle['languages']['zh'])
        self.assertEqual(self.bundle['languages']['sh']['glottocode'],'sout1528')
    def test_low_german_review_is_bounded(self):
        x=self.bundle['languages']['nds'];self.assertEqual(x['glottocode'],'lowg1239')
        e=next(e for e in x['links'] if e['status']=='reviewed');self.assertEqual(e['relation'],'catalog_correspondence');self.assertFalse(e['temporal_transfer_allowed'])
    def test_csv_preserves_link_assertions(self):
        with (ROOT/'wiktionary_langcodes.csv').open() as f:rows={x['code']:x for x in csv.DictReader(f)}
        self.assertEqual(json.loads(rows['arc']['links']),self.bundle['languages']['arc']['links'])
    def test_reverse_index_includes_family_proposals(self):
        entries=[e for es in self.bundle['index_by_target'].values() for e in es]
        self.assertTrue(any(e['source'].startswith('wikt:family:') and e['status']=='proposed' for e in entries))
    def test_iso_parts_are_distinct_namespaces(self):
        targets={e['target'] for e in self.bundle['languages']['de']['links']}
        self.assertTrue({'iso639-1:de','iso639-2b:ger','iso639-2t:deu','iso639-3:deu'}<=targets)
    def test_resolution_ledger_covers_every_finding(self):
        ledger=json.loads((CATALOG_CODE/'review-ledger.json').read_text())
        self.assertEqual([x['id'] for x in ledger],[f'C{i:02d}' for i in range(1,21)])
        for row in ledger:
            self.assertIn(row['status'],('resolved','resolved_with_explicit_limits'))
            self.assertTrue(row['resolution']);self.assertTrue(row['checks'])
            for check in row['checks']:
                source=(REPO/check['suite']).read_text()
                self.assertIn('def '+check['test']+'(',source)
    def test_all_context_or_proposal_records_are_queued(self):
        queued={r['subject'] for r in self.bundle['review_queue']}
        for row in list(self.bundle['languages'].values())+list(self.bundle['families'].values()):
            if row['review_state']=='needs_review' or any(e['status'] in ('proposed','context') for e in row['links']):self.assertIn(row['id'],queued)
    def test_deterministic_link_order_and_fingerprint(self):
        self.assertEqual(r.digest(self.bundle),r.digest(json.loads(r.json_bytes(self.bundle))))
        for x in self.bundle['languages'].values():self.assertEqual([e['id'] for e in x['links']],sorted(e['id'] for e in x['links']))



class ProvenanceSafety(unittest.TestCase):
    def test_source_tampering_fails_before_mapping(self):
        from unittest.mock import patch
        real=r.sha
        def altered(path):return '0'*64 if Path(path).name=='iso-639-3.tab' else real(path)
        with patch.object(r,'sha',side_effect=altered):
            with self.assertRaisesRegex(ValueError,'source hash mismatch'):r.source_snapshot(build.find_glottolog_cldf())
    def test_source_identity_survives_path_relocation(self):
        from unittest.mock import patch
        before=r.source_snapshot(build.find_glottolog_cldf())
        with patch.object(r.os.path,'relpath',return_value='different/physical/location'):
            after=r.source_snapshot(build.find_glottolog_cldf())
        self.assertEqual(before['digest'],after['digest'])
        self.assertNotEqual(before['files'],after['files'])
    def test_multiple_releases_require_explicit_selection(self):
        from unittest.mock import patch
        with patch.object(build,'selected',return_value=Path('/missing-pinned-release')):
            with self.assertRaisesRegex(ValueError,'selected Glottolog'):build.find_glottolog_cldf()
    def test_full_statement_fetch_retains_evidence(self):
        from unittest.mock import patch
        import tempfile
        from source_tools import catalog_sources as fetch_sources
        raw={'entities':{'Q1':{'lastrevid':123,'claims':{'P1394':[{'id':'Q1$abc','rank':'deprecated','qualifiers':{'P580':[{'snaktype':'novalue'}]},'references':[{'hash':'source'}],'mainsnak':{'snaktype':'value','datavalue':{'value':'aaaa1234'}}}]}}},'redirects':[{'from':'Q0','to':'Q1'}]}
        with tempfile.TemporaryDirectory() as tmp,patch.object(fetch_sources,'SOURCES',tmp),patch.object(fetch_sources,'wiktionary_qids',return_value=['Q0']),patch.object(fetch_sources,'http_get',return_value=json.dumps(raw).encode()):
            m={'files':{}};fetch_sources.fetch_statements(m)
            saved=json.loads((Path(tmp)/'wikidata_statements.json').read_text())
            self.assertEqual(saved['responses'],[raw]);self.assertEqual(saved['entities']['Q1']['claims']['P1394'][0]['rank'],'deprecated')
    def test_exporter_paths_are_repository_relative(self):
        spec=importlib.util.spec_from_file_location('wety_exporter',REPO/'packages/source_tools/language_data.py')
        exporter=importlib.util.module_from_spec(spec);spec.loader.exec_module(exporter)
        self.assertEqual(exporter.ROOT,REPO)
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'result.json';exporter.atomic_json(path,{'A':'é'})
            self.assertEqual(json.loads(path.read_text()),{'A':'é'});self.assertEqual(exporter.sha(path),r.sha(path))

if __name__=="__main__":unittest.main()
