import json
from pathlib import Path
import tempfile
import unittest
from harness.live_evaluation import task_payload, score, verify_inputs, report
from harness.llm_backends import strict_schema
from harness.llm import Extraction, ExtractedTime, SourceTime, _time_value


class LiveEvaluationTests(unittest.TestCase):
    def test_report_keeps_failures_and_unknown_costs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'results').mkdir()
            cells=[{'id':str(i),'backend':'codex','role':'screen','task':'s'} for i in range(2)]
            (root/'plan.json').write_text(json.dumps({'cells':cells,'tasks':{'s':{'kind':'screen','expected':['include']}}}))
            (root/'results/0.json').write_text(json.dumps({'status':'completed','elapsed_seconds':1,'score':{'correct':1}}))
            (root/'results/1.json').write_text(json.dumps({'status':'failed'}))
            group=report(root)['groups'][0]
            self.assertEqual((group['attempted_calls'],group['correct_cases'],group['case_denominator']),(2,1,2))
            self.assertIsNone(group['reported_cost_sum_usd'])
            self.assertEqual(group['cost_unreported_calls'],2)

    def test_production_schema_has_no_open_dictionary_objects(self):
        def check(node):
            if isinstance(node,dict):
                if node.get('type')=='object':
                    self.assertIs(node.get('additionalProperties'),False)
                    self.assertEqual(set(node['required']),set(node['properties']))
                for value in node.values():check(value)
            elif isinstance(node,list):
                for value in node:check(value)
        check(strict_schema(Extraction))

    def test_native_transport_rejects_unit_strings_and_invented_keys(self):
        for native in ({'form':'point','unit':'BP','value':'5000 BP'},
                       {'form':'event_range','unit':'BP','low':2300,'high':2800}):
            with self.assertRaises(ValueError):SourceTime.model_validate(native)

    def test_native_distribution_normalizes_and_unknown_epoch_preserves_numbers(self):
        t=ExtractedTime(expression='Mean 5000 BP, HPD 4000–6000 BP',conversion='',
            source_values={'form':'distribution','unit':'BP','present':2000,'age_scale':'calendar','mean':5000,
                           'lower':4000,'upper':6000,'interval_type':'hpd','probability':0.95})
        actual=_time_value(t)
        self.assertEqual((actual.mean,actual.hpd95),(-3000,[-4000,-2000]))
        t.source_values.present=None
        self.assertTrue(_time_value(t).unknown)
        self.assertEqual(t.model_dump()['source_values']['mean'],5000)

    def test_unknown_native_time_cannot_hide_numeric_claim(self):
        with self.assertRaises(ValueError):
            ExtractedTime(expression='unknown',conversion='',source_values={'form':'unknown','value':5000})

    def test_native_shape_checked_even_without_epoch(self):
        invalid=[{'form':'point','unit':'BP','median':3000},
                 {'form':'distribution','unit':'BP','value':5000,'mean':5000},
                 {'form':'event_range','unit':'BP','lower':2300,'upper':2800,'interval_type':'reported_range'}]
        for native in invalid:
            with self.assertRaises(ValueError):ExtractedTime(expression='test',conversion='',source_values=native)
        valid=ExtractedTime(expression='0.4–0.6 ka BP',conversion='',source_values={
            'form':'distribution','unit':'ka_BP','lower':0.4,'upper':0.6,'interval_type':'reported_range'})
        self.assertEqual(valid.source_values.lower,0.4)
        self.assertTrue(_time_value(valid).unknown)

    def test_vocabulary_supplies_alternatives_not_a_per_case_answer(self):
        task={'kind':'extract','vocabulary':{'role':['posterior','editorial','unknown']},
              'cases':[{'id':'c','input_paraphrase':'An assertion.','expected':{'role':'posterior'}}]}
        _,_,schema=task_payload(task)
        self.assertEqual(strict_schema(schema)['$defs']['Case0']['properties']['role']['enum'],
                         ['posterior','editorial','unknown'])

    def test_reference_values_are_not_in_model_input_or_schema(self):
        task={'kind':'extract','cases':[{'id':'case','input_paraphrase':'A dated event.',
            'expected':{'event':'secret_reference_value','calendar_epoch':123456}}]}
        system,user,schema=task_payload(task)
        serialized=system+user+json.dumps(strict_schema(schema))
        self.assertNotIn('secret_reference_value',serialized)
        self.assertNotIn('123456',serialized)
        self.assertIn('calendar_epoch',serialized)

    def test_missing_prediction_does_not_equal_explicit_unknown(self):
        task={'kind':'extract','cases':[{'id':'c','input_paraphrase':'No epoch.','expected':{'calendar_epoch':None}}]}
        self.assertEqual(score(task,{})['correct_fields'],0)
        self.assertEqual(score(task,{'c':{'calendar_epoch':None}})['correct_fields'],1)
        self.assertTrue(score(task,{})['errors'][0]['missing'])

    def test_screen_omissions_and_false_exclusions_stay_in_denominator(self):
        task={'kind':'screen','expected':['include','exclude','include']}
        result=score(task,{'decisions':[{'decision':'exclude'}]})
        self.assertEqual((result['cases'],result['correct'],result['missing'],result['false_exclusions']),(3,0,2,2))

    def test_input_lock_rejects_changed_experiment_configuration(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'configs').mkdir();(root/'plan.json').write_text('{}')
            config=root/'configs/a.json';config.write_text('{"model":"original"}')
            first=verify_inputs(root);self.assertEqual(verify_inputs(root),first)
            config.write_text('{"model":"other"}')
            with self.assertRaisesRegex(ValueError,'inputs changed'):verify_inputs(root)

    def test_duplicate_calls_are_diagnostics_not_production_acceptance(self):
        result=score({'kind':'probe'},{'kind':'conventional_period','earliest':450,'latest':1100})
        self.assertTrue(result['passed'])
        self.assertEqual(result['scope'],'transport probe only')

    def test_reference_must_exist_and_match_before_dispatch(self):
        import hashlib
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);(root/'configs').mkdir()
            (root/'plan.json').write_text(json.dumps({'reference_file':'reference.json','reference_sha256':hashlib.sha256(b'{}').hexdigest()}))
            with self.assertRaises(FileNotFoundError):verify_inputs(root)
            (root/'reference.json').write_text('{}');verify_inputs(root)
            (root/'reference.json').write_text('{"changed":true}')
            with self.assertRaisesRegex(ValueError,'reference hash'):verify_inputs(root)

    def test_extraction_does_not_send_catalog_descendants_as_evidence(self):
        from unittest.mock import patch
        from harness.llm import extract_system
        class G:
            rows={'family':{'name':'Research family'}}
            def descendants(self, family):raise AssertionError('catalog expansion must not enter extraction prompt')
        with patch('harness.llm.glotto.load',return_value=G()):
            prompt=extract_system('family')
        self.assertIn('leave glottocode null',prompt)
        self.assertIn('Research family',prompt)


if __name__=='__main__':unittest.main()
