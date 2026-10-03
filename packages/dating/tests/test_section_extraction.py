import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from harness.extraction_recovery import inspect_response, source_segment
from harness.section_packets import build_packets, packet_prompt, reconcile
from harness.section_extraction import prepare_repairs


def claim():
    return {'glottocode':None,'languoid_name':'Sample A','kind':'phylo_node_age',
            'subject_type':'sampled_clade','subject_id':'sample:a','event':'crown_divergence',
            'epistemic_role':'posterior','analysis_id':'a','result_id':'r','sampled_taxa':['A','B'],
            'time':{'expression':'5000 BP','conversion':'','source_values':{'form':'point','unit':'BP','value':5000}},
            'locator':'Results','quote':'The Sample A node was dated to 5000 BP.'}


def envelope(claims):return json.dumps({'claims':claims,'gaps':[],'conflicts':[],'paper_summary':'A study.'})


class RecoveryTests(unittest.TestCase):
    def test_invented_calendar_fields_cannot_hide_behind_unknown_native_scale(self):
        c=claim();c['time']['earliest']=-3050;c['time']['latest']=-2050
        r=inspect_response(envelope([c]),[source_segment(c['quote'],'s')])['claims'][0]
        self.assertTrue(r['schema_valid']);self.assertTrue(r['exact_quote'])
        self.assertTrue(r['calendar_projection']['unknown']);self.assertFalse(r['mechanically_ready'])
        self.assertEqual(len(r['calendar_conflicts']),2)
        self.assertEqual(r['raw']['time']['earliest'],-3050)

    def test_bad_assertion_does_not_erase_valid_one_or_mutate_input(self):
        good=claim();bad=copy.deepcopy(good);bad['time']['source_values']['unit']=None
        raw=envelope([good,bad]);r=inspect_response(raw,[source_segment(good['quote'],'paper')])
        self.assertEqual((r['status'],r['valid_claims'],r['invalid_items']),('partial',1,1))
        self.assertTrue(r['claims'][0]['mechanically_ready']);self.assertEqual(r['claims'][1]['raw'],bad)
        self.assertEqual(r['claims'][0]['spans'][0]['start'],0)
        self.assertEqual(r['raw_sha256'],hashlib.sha256(raw.encode()).hexdigest())

    def test_numeric_coercion_unknown_fields_and_duplicate_keys_not_hidden(self):
        c=claim();c['time']['source_values']['value']='5000'
        self.assertFalse(inspect_response(envelope([c]),[])['claims'][0]['schema_valid'])
        c=claim();c['invented']='extra'
        self.assertFalse(inspect_response(envelope([c]),[])['claims'][0]['schema_valid'])
        for raw in ['{"claims":[],"claims":[1]}','{"claims":NaN}','{"claims":1e999}','{"claims":[']:
            self.assertEqual(inspect_response(raw,[])['status'],'unreadable')

    def test_envelope_and_gap_failures_are_partial_even_when_claims_valid(self):
        d=json.loads(envelope([claim()]));d['gaps']=[{'kind':'made_up'}];del d['paper_summary']
        r=inspect_response(json.dumps(d),[source_segment(claim()['quote'],'paper')])
        self.assertEqual((r['status'],r['valid_claims']),('partial',1))
        self.assertTrue(r['envelope_errors']);self.assertFalse(r['gaps'][0]['schema_valid'])

    def test_disjoint_segments_cannot_forge_contiguous_quote(self):
        c=claim();quote=c['quote'];segments=[source_segment(quote[:20],'s',start=0),source_segment(quote[20:],'s',start=100)]
        r=inspect_response(envelope([c]),segments)
        self.assertFalse(r['claims'][0]['exact_quote'])

    def test_context_only_quote_keeps_provenance_without_primary_credit(self):
        c=claim();r=inspect_response(envelope([c]),[source_segment(c['quote'],'s','context',100)])
        self.assertTrue(r['claims'][0]['exact_quote']);self.assertFalse(r['claims'][0]['primary_evidence'])
        self.assertEqual(r['claims'][0]['spans'][0]['start'],100)

    def test_invalid_numbers_and_semantic_event_stay_distinct_from_schema(self):
        c=claim();c['event']='relative_constraint';c['time']['source_values']['value']=7000
        r=inspect_response(envelope([c]),[source_segment(c['quote'],'s')])['claims'][0]
        self.assertTrue(r['schema_valid']);self.assertTrue(r['exact_quote'])
        self.assertFalse(r['native_support']);self.assertTrue(r['semantic_flags'])


class SectionTests(unittest.TestCase):
    def test_partitions_text_without_holes_with_bounded_exact_context(self):
        text='Methods\nBP refers to 2000.\n'+('A whole paragraph. '*20+'\n')*20
        d=build_packets(text,'s',headings=[{'start':0,'end':7,'text':'Methods','level':2}],max_chars=900,context_chars=200,overlap_chars=50)
        self.assertEqual(''.join(p['primary']['text'] for p in d['packets']),text)
        for p in d['packets']:
            self.assertLessEqual(sum(len(s['text']) for s in [p['primary'],*p['context']]),900)
            for s in [p['primary'],*p['context']]:self.assertEqual(text[s['start']:s['end']],s['text'])
        self.assertIn('Methods', ''.join(s['text'] for s in d['packets'][-1]['context']))

    def test_context_budget_omissions_and_long_line_split_are_visible(self):
        text='X'*3000
        d=build_packets(text,'s',context_spans=[{'start':0,'end':1000}],max_chars=500,context_chars=100,overlap_chars=50)
        self.assertTrue(d['packets'][0]['boundary_split'])
        self.assertTrue(d['packets'][1]['context_omissions'])
        bad=copy.deepcopy(d['packets'][0]);bad['primary']['text']='changed'
        with self.assertRaises(ValueError):packet_prompt(bad)

    def test_same_date_different_scenario_not_deduplicated(self):
        c=claim();text=c['quote'];packet=build_packets(text,'s')['packets'][0]
        d=copy.deepcopy(c);d['analysis_id']='alternative'
        recovery=inspect_response(envelope([c,d]),[packet['primary']])
        plan={'tasks':{'t':{'kind':'section_extract','packet':packet}},'cells':[{'id':'a','task':'t'},{'id':'b','task':'t'}]}
        r=reconcile(plan,{'a':{'status':'failed','recovery':recovery}})
        self.assertEqual(len(r['proposals']),2);self.assertFalse(r['document_complete'])
        self.assertEqual(r['incomplete_cells'],['a','b'])
        r=reconcile(plan,{key:{'status':'completed','recovery':recovery} for key in ['a','b']})
        self.assertEqual(len(r['proposals']),2);self.assertEqual(len(r['proposals'][0]['occurrences']),2)
        plan['tasks']['unassigned']={'kind':'section_extract','packet':packet}
        r=reconcile(plan,{key:{'status':'completed','recovery':recovery} for key in ['a','b']})
        self.assertFalse(r['document_complete']);self.assertEqual(r['unscheduled_sections'],['unassigned'])

    def test_repair_plan_shares_budget_and_only_targets_invalid_claims(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)/'first';root.mkdir();(root/'configs').mkdir();(root/'results').mkdir();(root/'reference.json').write_text('{}')
            c=claim();bad=copy.deepcopy(c);bad['time']['source_values']['unit']=None
            packet=build_packets(c['quote'],'s')['packets'][0];recovery=inspect_response(envelope([c,bad]),[packet['primary']])
            plan={'tasks':{'t':{'kind':'section_extract','packet':packet}},'cells':[{'id':'a','task':'t'}],
                  'budget_ledger':'budget.json','reference_file':'reference.json'}
            (root/'plan.json').write_text(json.dumps(plan));(root/'results/a.json').write_text(json.dumps({'recovery':recovery}))
            target=Path(tmp)/'repairs';prepare_repairs(root,target)
            out=json.loads((target/'plan.json').read_text())
            self.assertEqual(out['tasks']['a']['repair_targets'][0]['index'],1)
            self.assertEqual((target/out['budget_ledger']).resolve(),(root/'budget.json').resolve())
            with self.assertRaises(ValueError):prepare_repairs(target,Path(tmp)/'again')
            self.assertEqual(prepare_repairs(root,Path(tmp)/'other')['repair_cells'],0)


if __name__=='__main__':unittest.main()
