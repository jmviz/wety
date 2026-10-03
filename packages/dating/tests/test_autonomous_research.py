import json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
from harness.autonomous_research import discover,screen,acquire_sources,network,candidates_from
from harness.io_utils import atomic_json

class QueueTests(unittest.TestCase):
 def test_source_selection_follows_only_bound_screen_and_is_idempotent(self):
  with tempfile.TemporaryDirectory() as temp:
   root=Path(temp);(root/'configs').mkdir();atomic_json(root/'brief.json',{})
   cs=[{'key':str(i),'urls':[]} for i in range(3)];atomic_json(root/'candidates.json',cs)
   brief={'family':'test','configs':{'codex':'unused'},'selection_backend':'codex','limits':{'sources':1,'followups':2}};state={'stages':{}}
   def model(*args):
    d=root/'screen/results';d.mkdir();atomic_json(d/'codex-screen.json',{'status':'completed','request_ref':'bound-request','output':{'decisions':[{'key':'0','decision':'exclude'},{'key':'1','decision':'include'},{'key':'2','decision':'include'}]}})
   with patch('harness.autonomous_research.run_phase',side_effect=model) as run:
    screen(root,brief,state);screen(root,brief,state);self.assertEqual(run.call_count,1)
   graph=json.loads((root/'source-graph.json').read_text());self.assertEqual([r['status'] for r in graph],['excluded','selected','source_cap_omission'])
   with patch('harness.autonomous_research.network') as get:
    acquire_sources(root,brief,state);self.assertFalse(get.called)
   self.assertEqual(json.loads((root/'source-graph.json').read_text())[1]['status'],'access_or_parser_gap')
   self.assertEqual(json.loads((root/'sources.json').read_text()),[])

 def test_misaligned_selector_cannot_acquire_and_failed_http_is_not_retried(self):
  with tempfile.TemporaryDirectory() as temp:
   root=Path(temp);(root/'configs').mkdir();atomic_json(root/'brief.json',{})
   atomic_json(root/'candidates.json',[{'key':'x','urls':['https://example.org']}])
   brief={'family':'test','configs':{'codex':'unused'},'selection_backend':'codex','limits':{'sources':1}};state={'stages':{},'network':{'x':{'status':'reserved'}}}
   with patch('harness.autonomous_research.run_phase'):
    screen(root,brief,state)
   self.assertEqual(json.loads((root/'source-graph.json').read_text())[0]['status'],'screen_unresolved')
   with patch('harness.autonomous_research.run_process_group') as process:
    self.assertIsNone(network(root,state,'x','https://example.org'));process.assert_not_called()

 def test_discovery_balances_queries_and_deduplicates_by_source_identity(self):
  with tempfile.TemporaryDirectory() as temp:
   root=Path(temp);paths=[]
   for i in range(2):
    p=root/f'{i}.json';atomic_json(p,{'resultList':{'result':[{'id':str(j),'source':'MED','doi':f'10.test/{j}','title':'title'} for j in ([1,2,3] if i==0 else [4,1,5])]}});paths.append(p)
   state={'stages':{},'network':{}}
   with patch('harness.autonomous_research.network',side_effect=paths):discover(root,{'queries':['a','b'],'limits':{'candidates':3}},state)
   rows=json.loads((root/'candidates.json').read_text());self.assertEqual([r['key'] for r in rows],['10.test/1','10.test/4','10.test/2'])
   self.assertEqual(rows[0]['found_by'],['0','1']);self.assertEqual(state['stages']['discover']['omitted_by_cap'],2)

class ResumeIntegrityTests(unittest.TestCase):
 def test_retained_network_payload_cannot_change_silently(self):
  import hashlib
  with tempfile.TemporaryDirectory() as temp:
   root=Path(temp);artifact=root/'artifact';(artifact/'files').mkdir(parents=True);p=artifact/'files/source';p.write_text('original')
   state={'network':{'x':{'status':'complete','artifact':str(artifact),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}}}
   self.assertEqual(network(root,state,'x','https://example.org'),p)
   p.write_text('changed')
   with self.assertRaisesRegex(ValueError,'retained acquisition changed'):network(root,state,'x','https://example.org')

 def test_stage_output_edit_is_detected_before_resuming(self):
  from harness.autonomous_research import pin_outputs,verify_outputs
  with tempfile.TemporaryDirectory() as temp:
   root=Path(temp);(root/'candidates.json').write_text('[]');state={}
   pin_outputs(root,state);verify_outputs(root,state)
   (root/'candidates.json').write_text('[{}]')
   with self.assertRaisesRegex(ValueError,'stage output changed'):verify_outputs(root,state)

class NamedScreenTests(unittest.TestCase):
 def test_selector_keys_bind_decisions_even_when_response_order_changes(self):
  from harness.autonomous_research import aligned_decisions
  candidates=[{'key':'a'},{'key':'b'}]
  result={'status':'completed','output':{'decisions':[{'key':'b','decision':'include'},{'key':'a','decision':'exclude'}]}}
  self.assertEqual([x['decision'] for x in aligned_decisions(candidates,result)],['exclude','include'])
  for bad in [[{'key':'a'},{'key':'a'}],[{'key':'a'},{'key':'unknown'}],[]]:
   self.assertIsNone(aligned_decisions(candidates,{'status':'completed','output':{'decisions':bad}}))

if __name__=='__main__':unittest.main()
