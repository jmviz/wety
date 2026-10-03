import copy,json,tempfile,unittest
from pathlib import Path
from harness.compact_extraction import inspect_compact
from harness.compact_repair import inspect_repairs,prepare_repairs,repair_candidates,repair_payload
from harness.compact_review import prepare_reviews
from harness.io_utils import digest,atomic_json
from harness.section_packets import build_packets,definition_context


def fixture():
    a={'subject':'Sample','subject_type':'documentary_sample','event':'first_attestation','role':'observation','method':'dating',
       'native_time':{'form':'unknown'},'expression':'254 BCE','quote':'The earliest inscription is dated to 254 BCE.',
       'locator':'text','qualifications':['Attribution remains contested.']}
    p=build_packets(a['quote'],'s')['packets'][0]
    t={'kind':'compact_repair','family':'test','packet':p,'repair_targets':[{'target_id':'a','index':0,'raw':a,'raw_digest':digest(a)}]}
    return a,p,t


def patch(native=None,**kw):
    return {'target_id':'a','status':'proposed','native_time':native or {'form':'point','unit':'BCE','value':254},'rationale':'Explicit source value.',**kw}


def inspect(items,task):return inspect_repairs(json.dumps({'repairs':items}),task)


class RepairTests(unittest.TestCase):
    def test_patch_wire_contract_cannot_express_proposed_unknown(self):
        from harness.compact_repair import NativeRepairs
        schema=NativeRepairs.model_json_schema()
        self.assertNotIn('NativeUnknown',schema['$defs'])
        self.assertEqual(schema['$defs']['UnresolvedNativePatch']['properties']['native_time']['type'],'null')

    def test_publisher_thousands_separators_do_not_merge_table_rows(self):
        from harness.time_interpretation import native_numbers_match
        for sep in [',',' ','\u2009','\u202f','\u00a0']:
            self.assertTrue(native_numbers_match({'value':10000},'upper bound of 10'+sep+'000 yr'))
        self.assertFalse(native_numbers_match({'value':10000},'10\n000'))
        self.assertFalse(native_numbers_match({'value':800100},'800 1000 years'))

    def test_only_native_changes_and_original_digest_survives(self):
        a,p,t=fixture();r=inspect([patch()],t)
        self.assertTrue(r['aligned']);c=r['claims'][0]
        self.assertTrue(c['mechanically_ready']);self.assertEqual(c['original_raw_digest'],digest(a))
        for k in a:
            if k!='native_time':self.assertEqual(c['proposal'][k],a[k])
        self.assertEqual(a['native_time'],{'form':'unknown'})
        system,_,_=repair_payload(t);self.assertIn('native_time ONLY',system)

    def test_unresolved_preserves_gaps_and_invalid_envelopes_cannot_review(self):
        _,_,t=fixture()
        self.assertTrue(inspect([patch(status='unresolved',native_time=None)],t)['aligned'])
        for rows in [[],[patch(),patch()],[patch(target_id='alien')],[patch(native={'form':'unknown'})],
                     [patch(native={'form':'point','unit':'BCE','value':-254})],[patch(subject='rewritten')]]:
            r=inspect(rows,t);self.assertFalse(r['aligned'])
            self.assertFalse(any(c.get('mechanically_ready') for c in r['claims']))
        r=inspect([patch(native={'form':'point','unit':'BCE','value':255})],t)
        self.assertTrue(r['aligned']);self.assertFalse(r['claims'][0]['mechanically_ready'])
        t['repair_targets'][0]['raw']['subject']='changed'
        with self.assertRaisesRegex(ValueError,'digest mismatch'):inspect([patch()],t)

    def test_candidate_selection_does_not_claim_to_repair_other_fields(self):
        a,p,_=fixture();bad={**a,'event':'invented'}
        rec=inspect_compact(json.dumps({'assertions':[a,bad],'gaps':[],'conflicts':[],'overflow':False}),p)
        self.assertEqual(len(repair_candidates(rec,p)),1)
        a['native_time']={'form':'point','unit':'BCE','value':254}
        rec=inspect_compact(json.dumps({'assertions':[a],'gaps':[],'conflicts':[],'overflow':False}),p)
        self.assertEqual(repair_candidates(rec,p),[])

    def test_single_pass_shared_ledger_and_repaired_review_lineage(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)/'extract';root.mkdir();(root/'configs').mkdir();(root/'results').mkdir()
            a,p,_=fixture();atomic_json(root/'reference.json',{})
            plan={'tasks':{'t':{'kind':'compact_extract','family':'f','packet':p}},'cells':[{'id':'a','backend':'codex','task':'t'}],
                  'reference_file':'reference.json','budget_ledger':'budget.json'}
            atomic_json(root/'plan.json',plan)
            rec=inspect_compact(json.dumps({'assertions':[a],'gaps':[],'conflicts':[],'overflow':False}),p)
            atomic_json(root/'results/a.json',{'status':'completed','recovery':rec})
            repaired=Path(temp)/'repair';self.assertEqual(prepare_repairs(root,repaired)['targets'],1)
            self.assertEqual(prepare_repairs(root,Path(temp)/'again')['repair_cells'],0)
            with self.assertRaisesRegex(ValueError,'never repair a repair'):prepare_repairs(repaired,Path(temp)/'twice')
            new=json.loads((repaired/'plan.json').read_text());task=new['tasks']['a']
            self.assertEqual((repaired/new['budget_ledger']).resolve(),(root/'budget.json').resolve())
            result=inspect([patch(target_id=task['repair_targets'][0]['target_id'])],task)
            (repaired/'results').mkdir();atomic_json(repaired/'results/a-native-repair-1.json',{'status':'completed','native_repair':result})
            reviewed=Path(temp)/'review';self.assertEqual(prepare_reviews(repaired,reviewed)['assertions'],1)
            target=next(iter(json.loads((reviewed/'plan.json').read_text())['tasks'].values()))['targets'][0]
            self.assertEqual(target['original_raw_digest'],digest(a))


class ContextTests(unittest.TestCase):
    def test_whitespace_alignment_binds_original_excerpt_without_changing_words(self):
        from harness.compact_extraction import quote_spans
        text='The dated population arrived before 4\u2009500\u00a0BP.\nThis second statement has no date.'
        p=build_packets(text,'s')['packets'][0]
        quote='The dated population arrived before 4 500 BP.'
        spans=quote_spans(quote,[p['primary']]);self.assertEqual(len(spans),1)
        s=spans[0];self.assertEqual(s['match'],'whitespace_equivalent')
        self.assertEqual(s['source_quote'],text[s['start']:s['end']])
        self.assertNotEqual(s['source_quote'],quote)
        self.assertEqual(quote_spans(quote.replace('before','after'),[p['primary']]),[])
        self.assertEqual(quote_spans(quote.replace('BP.','BP!'),[p['primary']]),[])

    def test_epoch_sentence_survives_long_paragraph_without_inventing_text(self):
        for year in (1950,2000,2010):
            self.assertEqual(definition_context(f'We take {year} as the date for the present.')[0]['selector'],'epoch_definition')
        epoch='We calibrated the tree taking 2000 as the date for the present.'
        text='Introduction. '+'Unrelated methodological detail. '*80+epoch+'\n'+'Results without dates.\n'*100
        d=build_packets(text,'s',max_chars=700,context_chars=200,overlap_chars=50)
        last=d['packets'][-1]
        self.assertIn(epoch,''.join(s['text'] for s in last['context']))
        self.assertEqual(''.join(p['primary']['text'] for p in d['packets']),text)
        self.assertTrue(any(s['selector']=='epoch_definition' for s in last['context_selection']))
        for p in d['packets']:
            self.assertLessEqual(sum(len(s['text']) for s in [p['primary'],*p['context']]),700)
            for s in p['context']:self.assertEqual(s['text'],text[s['start']:s['end']])

    def test_explicit_context_precedes_headings_and_prior_support_is_retained(self):
        text='Methods\nA normal prior has standard deviation 50 years.\n'+'Results.\n'*100
        hints=definition_context(text);self.assertEqual(hints[0]['selector'],'prior_definition')
        d=build_packets(text,'s',context_spans=hints,headings=[{'start':0,'end':7}],max_chars=200,context_chars=50,overlap_chars=0)
        self.assertIn('standard deviation 50',d['packets'][-1]['context'][0]['text'])


class QuantityTests(unittest.TestCase):
    def inspect(self,native,quote,role='calibration_prior'):
        a,_,_=fixture();a.update(native_time=native,quote=quote,role=role,expression=quote)
        p=build_packets(quote,'s')['packets'][0]
        return inspect_compact(json.dumps({'assertions':[a],'gaps':[],'conflicts':[],'overflow':False}),p)['claims'][0]

    def test_prior_parameters_are_not_posterior_or_calendar_event_intervals(self):
        native={'form':'prior_distribution','unit':'years_ago','present':2000,'age_scale':'calendar','distribution_family':'normal',
                'location':1000,'standard_deviation':50,'support':'truncated','support_lower':800,'support_upper':1200}
        quote='A normal distribution centred on 1000 with standard deviation 50, truncated to 800–1200 years.'
        row=self.inspect(native,quote);self.assertTrue(row['mechanically_ready']);self.assertNotIn('calendar_projection',row)
        self.assertIn('calendar_abstention',row)
        for edit in [{'mean':1000},{'standard_deviation':0},{'support_lower':1300},{'support':'unbounded'}]:
            self.assertFalse(self.inspect({**native,**edit},quote)['schema_valid'])
        self.assertFalse(self.inspect(native,quote,role='posterior')['schema_valid'])
        self.assertFalse(self.inspect({**native,'standard_deviation':51},quote)['native_support'])
        # Location outside truncation support remains a valid parameterization.
        self.assertTrue(self.inspect({**native,'location':700},quote)['schema_valid'])

    def test_uniform_support_and_relative_quantities_retain_shape_without_projection(self):
        uniform={'form':'prior_distribution','unit':'years_ago','distribution_family':'uniform','support':'bounded','support_lower':0,'support_upper':10000}
        self.assertTrue(self.inspect(uniform,'A uniform prior has support from 0 to 10\u2009000 years.')['mechanically_ready'])
        elapsed={'form':'elapsed_duration','unit':'years','value':2000,'relation':'more_than'}
        row=self.inspect(elapsed,'Contact continued for more than 2000 years.',role='hypothesis')
        self.assertTrue(row['mechanically_ready']);self.assertNotIn('calendar_projection',row)
        self.assertFalse(self.inspect({**elapsed,'present':2000},'Contact continued for more than 2000 years.')['schema_valid'])
        offset={'form':'relative_offset','unit':'centuries','value':1,'direction':'before','approximate':True,'reference_event':'Nannaya’s work'}
        row=self.inspect(offset,'This text was probably about a century earlier than Nannaya’s work.',role='secondary_report')
        self.assertTrue(row['mechanically_ready']);self.assertNotIn('calendar_projection',row)
        self.assertFalse(self.inspect({**offset,'unit':'years'},'This text was about a century earlier than Nannaya’s work.')['native_support'])

    def test_failed_review_preserves_valid_judgments_without_eligibility(self):
        from harness.compact_extraction import inspect_review
        a,p,_=fixture();task={'packet':p,'targets':[{'assertion_id':k,'mechanically_ready':True,'primary_evidence':True} for k in ['a','b']]}
        j={'assertion_id':'a','disposition':'supported','subject_event':'supported','role_dependency':'supported','native_time':'supported',
           'scenario_scope':'supported','evidence_quote':a['quote'],'rationale':'Source supports the proposal.'}
        bad={**j,'assertion_id':'b','native_time':'uncertain'}
        r=inspect_review(json.dumps({'judgments':[j,bad],'omissions':[],'context_gaps':[]}),task)
        self.assertFalse(r['aligned']);self.assertEqual(len(r['judgments']),1);self.assertEqual(len(r['invalid_judgments']),1)
        self.assertFalse(r['judgments'][0]['eligible_for_editorial_consideration']);self.assertEqual(r['missing'],['b'])


if __name__=='__main__':unittest.main()
