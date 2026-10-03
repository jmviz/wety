import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

from harness import batch, llm, model_requests, registry
from harness.io_utils import digest
from harness.llm_backends import LLMResult, strict_schema


class FakeBackend:
    name = 'fixture'
    cfg = {'roles': {'extract': {'model':'fixture-model', 'reasoning':{'effort':'high'},
                                'api_key':'DO_NOT_STORE'}, 'synthesize':{'model':'fixture-model'}, 'screen':{'model':'fixture-model'}}, 'headers':{'Authorization':'DO_NOT_STORE'}}

    def batch_submit(self, role, items, schema):
        # Dispatch cannot precede persistence.
        assert list(model_requests.REQUEST_DIR.glob('inputs/*.json'))
        return {'id':'batch1', 'status':'submitted'}

    def batch_status(self, bid):
        return {'status':'completed'}

    def batch_results(self, bid, schema):
        return self.results


class ModelRequestTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        patcher = patch.object(model_requests, 'REQUEST_DIR', self.root/'requests')
        patcher.start(); self.addCleanup(patcher.stop)
        self.backend = FakeBackend()
        self.c = {'key':'key', 'source_id':'source', 'title':'Example', 'abstract':'This corpus was sampled at 1000 BP.'}

    def prepare(self):
        with patch('harness.retrieve.cached_text', return_value=None):
            return llm.prepare_extraction('fami1234', self.c, self.c['abstract'], 'system', self.backend, 'run')

    def result(self, **kw):
        ext = llm.Extraction(claims=[], paper_summary='A source.', **kw)
        return LLMResult(parsed=ext, text=ext.model_dump_json(), model='returned-model', backend='fixture')

    def test_snapshot_replay_and_settings_exclude_credentials(self):
        user, ref = self.prepare()
        request = model_requests.load_request(ref)
        self.assertEqual(request['user'], user)
        self.assertEqual(request['schema'], strict_schema(llm.Extraction))
        self.assertEqual(request['prompt_sha256'], digest({'system':'system','user':user}))
        self.assertNotIn('DO_NOT_STORE', json.dumps(request))
        self.assertEqual(request['requested_execution']['role_settings']['reasoning']['effort'], 'high')
        self.assertEqual(self.prepare()[1], ref)
        response_ref = model_requests.record_response(ref, self.result())
        response = json.loads((model_requests.REQUEST_DIR/'outputs'/(response_ref+'.json')).read_text())
        self.assertEqual(response['reported_execution']['model'], 'returned-model')
        self.assertEqual(response['request_ref'], ref)

    def test_modified_or_path_shaped_reference_rejected(self):
        _, ref = self.prepare()
        p = model_requests.REQUEST_DIR/'inputs'/(ref+'.json')
        d = json.loads(p.read_text()); d['user']='changed'; p.write_text(json.dumps(d))
        with self.assertRaisesRegex(ValueError,'digest/version mismatch'): model_requests.load_request(ref)
        with self.assertRaisesRegex(ValueError,'invalid model snapshot'): model_requests.load_request('../secret')
        with self.assertRaisesRegex(ValueError,'content mismatch'): self.prepare()

    def test_failed_extraction_recovery_is_immutable_sidecar_not_ingestion(self):
        _,ref=self.prepare()
        good=llm.ExtractedClaim(glottocode=None,languoid_name='Sample',kind='phylo_node_age',
            locator='source',quote=self.c['abstract'],time=llm.ExtractedTime(expression='1000 BP',conversion='',
            source_values={'form':'point','unit':'BP','value':1000})).model_dump(mode='json')
        bad=json.loads(json.dumps(good));bad['time']['source_values']['unit']=None
        failure=model_requests.record_failure(ref,ValueError('partial'),{'text':json.dumps({
            'claims':[good,bad],'paper_summary':'Source.','gaps':[],'conflicts':[]})})
        recovery_ref=model_requests.recover_failed_extraction(ref,failure)
        recovered=json.loads((model_requests.REQUEST_DIR/'recoveries'/(recovery_ref+'.json')).read_text())
        self.assertEqual(recovered['recovery']['valid_claims'],1)
        self.assertEqual(recovered['status'],'proposals_only_incomplete_extraction')
        self.assertEqual(model_requests.recover_failed_extraction(ref,failure),recovery_ref)
        self.assertEqual(list(model_requests.REQUEST_DIR.glob('outputs/*.json')),[])
        self.c['source_id']='different'
        _,other=self.prepare()
        with self.assertRaises(ValueError):model_requests.recover_failed_extraction(other,failure)

    def test_ingestion_uses_submitted_evidence_after_source_changes(self):
        _, ref = self.prepare()
        quote = self.c['abstract']
        claim = llm.ExtractedClaim(glottocode=None, languoid_name='Sample', kind='attestation_first',
            locator='Source section', quote=quote, time=llm.ExtractedTime(expression='1000 BP', conversion='unknown',
                source_values={'form':'point','unit':'BP','value':1000,'present':2000}))
        res = self.result(); res.parsed.claims=[claim]
        self.c['abstract']='Changed after submission.'
        self.c['retrieved']={'quality':'changed after submission'}
        stats={'claims':0,'verified':0,'unverified':0,'placed':0}
        with patch.object(llm.glotto,'load',return_value=SimpleNamespace(rows={})), \
             patch('harness.store.replace_extraction') as save, patch.object(llm,'append_log'), \
             patch('harness.config.CACHE_DIR',str(self.root/'cache')), \
             patch.object(llm,'extraction_input',side_effect=AssertionError('must not reread inputs')):
            llm._ingest_extraction('fami1234',self.c,res,ref,'run',stats)
        stored = save.call_args.args[2][0]
        self.assertTrue(stored.quote_verified)
        self.assertEqual(stored.evidence['retrieval_quality'],'unverified')
        self.assertEqual(self.c['extracted']['request_ref'],ref)
        self.assertEqual(stats['verified'],1)

    def test_request_cannot_be_reassigned_to_another_source(self):
        _, ref = self.prepare()
        with patch.object(llm.glotto,'load',return_value=SimpleNamespace(rows={})), self.assertRaisesRegex(ValueError,'identity mismatch'):
            llm._ingest_extraction('fami1234',{'source_id':'other'},self.result(),ref,'run',{})

    def test_zero_claim_gaps_and_conflicts_remain_proposals(self):
        first='This source explicitly gives the date as 1000 BP.'
        second='The deposited table instead gives the date as 1200 BP.'
        res = self.result(gaps=[{'kind':'missing_diagnostics','description':'No chains supplied','next_action':'Inspect the named deposit'}],
            conflicts=[{'kind':'quantity','description':'Different dates','sides':[{'locator':'article','quote':first},{'locator':'table','quote':second}], 'next_action':'Check version and scenario identity'}])
        coverage={'main_text':{'status':'included'},'attachments':[{'name':'table','status':'not_retrieved'}]}
        issues = llm.extraction_issues(res.parsed, first, coverage)
        self.assertEqual(issues['conflicts'][0]['status'],'unverified_quote')
        self.assertEqual(issues['conflicts'][0]['quote_matches'],[True,False])
        self.assertEqual(len(issues['gaps']),1)
        issues = llm.extraction_issues(res.parsed, first+'\n'+second, coverage)
        self.assertEqual(issues['conflicts'][0]['status'],'pending_semantic_review')
        self.assertEqual(len(issues['input_limits']),1)

    def run_batch(self, mutate=False, legacy=False):
        # Freeze the same payload used by the extraction API, then mutate its source if requested.
        with patch('harness.retrieve.cached_text', return_value=None):
            user, ref=llm.prepare_extraction('fami1234',self.c,self.c['abstract'],'system',self.backend,'run',batch=True)
        (self.root/'candidates.json').write_text(json.dumps([self.c]))
        self.backend.results={'source':self.result()}
        with patch.object(batch,'family_dir',return_value=str(self.root)), patch.object(batch,'append_log'), \
             patch('harness.llm_backends.get_backend',return_value=self.backend), \
             patch.object(llm,'extract_system',return_value='system'), \
             patch('harness.retrieve.text_for',side_effect=lambda c:c['abstract']), \
             patch.object(llm,'_ingest_extraction') as ingest:
            batch.submit('fami1234','extract',[('source','system',user)],llm.Extraction,self.backend,run='run',request_refs={'source':ref})
            if mutate:
                self.c['abstract']='New source text'; (self.root/'candidates.json').write_text(json.dumps([self.c]))
            if legacy:
                batches=batch._load('fami1234'); batches[0].pop('request_refs'); batch._save('fami1234',batches)
            out=batch.poll('fami1234')
            return out, ingest.call_args_list, ref

    def test_batch_passes_frozen_context_to_ingestion(self):
        out,calls,ref=self.run_batch()
        self.assertEqual(out[0]['ok'],1)
        self.assertEqual(calls[0].args[3],ref)

    def test_stale_batch_output_is_preserved_but_not_ingested(self):
        out,calls,_=self.run_batch(mutate=True)
        self.assertEqual(out[0]['error'],1)
        self.assertEqual(out[0]['ok'],0)
        self.assertEqual(calls,[])
        self.assertEqual(len(list(model_requests.REQUEST_DIR.glob('outputs/*.json'))),1)

    def test_legacy_batch_requires_resubmission(self):
        out,calls,_=self.run_batch(legacy=True)
        self.assertEqual(out[0]['error'],1)
        self.assertEqual(calls,[])

    def test_extraction_batch_rejects_missing_context_before_dispatch(self):
        with patch.object(batch,'family_dir',return_value=str(self.root)), self.assertRaisesRegex(ValueError,'frozen evidence context'):
            batch.submit('fami1234','extract',[('source','system','user')],llm.Extraction,self.backend)

    def test_review_coverage_rejects_unknown_duplicate_and_cross_subject_claims(self):
        payload=[{'subject':k,'claims':[{'id':'c'+k}]} for k in ('a','b','c','d')]
        review=llm.FamilyReview(reviews=[
            llm.LanguoidReview(subject_id='a',rationale='ok',recommended_claim_ids=['ca']),
            llm.LanguoidReview(subject_id='b',rationale='wrong claim',recommended_claim_ids=['ca']),
            llm.LanguoidReview(subject_id='c',rationale='duplicate'),
            llm.LanguoidReview(subject_id='c',rationale='duplicate'),
            llm.LanguoidReview(subject_id='not supplied',rationale='unknown')])
        r=registry.review_response_coverage(payload,review)
        self.assertEqual(r['valid_subject_ids'],['a'])
        self.assertEqual(r['unreviewed_subject_ids'],['b','c','d'])
        self.assertEqual(len(r['errors']),3)

    def test_synthesis_preserves_source_subjects_and_reports_limit_and_response_omissions(self):
        claims=[{'id':'c'+x,'subject':{'id':x,'catalog_id':'same-catalog'},'provenance':{'family':'fami1234'}} for x in ('a','b','c')]
        self.backend.complete=lambda *a,**kw: LLMResult(parsed=llm.FamilyReview(reviews=[llm.LanguoidReview(subject_id='a',rationale='review')]),text='response',model='fixture',backend='fixture')
        with patch.object(registry,'current_bundle',return_value={'claims':claims,'input_digest':'d'}), \
             patch('harness.llm_backends.get_backend',return_value=self.backend), patch.object(registry,'DATA_DIR',str(self.root)):
            r=registry.propose_review('fami1234','run',2)
        self.assertEqual((r['submitted'],r['reviewed'],r['adopted']),(2,1,0))
        self.assertEqual(r['input_coverage']['omitted_subject_ids'],['c'])
        self.assertEqual(r['response_coverage']['unreviewed_subject_ids'],['b'])
        proposal=json.loads(Path(r['proposal']).read_text())
        request=model_requests.load_request(proposal['request_ref'])
        self.assertEqual([p['subject'] for p in json.loads(request['user'])['subjects']],['a','b'])


if __name__ == '__main__': unittest.main()
