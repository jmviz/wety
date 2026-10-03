import json
import unittest
from harness.extraction_repair import inspect_repairs,RepairOutcome


class RepairTests(unittest.TestCase):
    def test_task_schema_and_scoring_allow_explicit_abstention(self):
        from harness.live_evaluation import task_payload,score
        from harness.section_packets import build_packets
        from harness.extraction_repair import RepairResponse
        task={'kind':'section_repair','family':'Sample','packet':build_packets('Evidence text.','s')['packets'][0],
              'repair_targets':[{'index':4,'raw_digest':'original'}]}
        _,prompt,schema=task_payload(task)
        self.assertIs(schema,RepairResponse);self.assertIn('status unresolved',prompt)
        self.assertEqual(score(task,{'repairs':[]})['outcomes'],0)

    def test_abstention_is_valid_without_fabricating_claim(self):
        raw=json.dumps({'repairs':[{'original_index':4,'status':'unresolved','claim':None,'rationale':'The source does not establish this assertion.'}]})
        result=inspect_repairs(raw,[],[{'index':4,'raw_digest':'original'}])
        self.assertTrue(result['aligned']);self.assertEqual(result['outcomes'][0]['original_digest'],'original')
        self.assertNotIn('claim_review',result['outcomes'][0])

    def test_duplicate_unknown_missing_and_inconsistent_targets_fail(self):
        row={'original_index':4,'status':'unresolved','claim':None,'rationale':'Unknown'}
        for rows in [[row,row],[dict(row,original_index=8)],[],[dict(row,status='corrected')]]:
            result=inspect_repairs(json.dumps({'repairs':rows}),[],[{'index':4,'raw_digest':'original'}])
            self.assertFalse(result['aligned'])
        with self.assertRaises(ValueError):RepairOutcome.model_validate(dict(row,original_index='4'),strict=True)


if __name__=='__main__':unittest.main()
