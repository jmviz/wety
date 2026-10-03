import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from harness.benchmark import semantic_metrics
from harness.config import DATA_DIR
from harness.contact_cases import validate_contact_case
from harness.packets import validate_packet


PACKET = Path(DATA_DIR) / '_research/packets/uralic-hundred-2026-09-19/packet.json'


class ContactCaseTests(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads(PACKET.read_text())
        self.case = json.loads((PACKET.parent / 'case.json').read_text())
        self.findings = [f['id'] for f in self.packet['findings']]

    def test_source_packet_and_abstention(self):
        result = validate_packet(PACKET, {}, [])
        self.assertEqual(result['errors'], [])
        self.assertEqual((result['claims'], result['relations'], result['findings']), (0, 0, 11))
        self.assertEqual(len(self.case['scenarios'][0]['orders']), 1)
        self.assertEqual(self.case['scenarios'][1]['orders'], [])
        self.assertFalse(result['independent_review'])

    def test_cycle_detected_within_scenario(self):
        s = self.case['scenarios'][0]
        edge = copy.deepcopy(s['orders'][0])
        edge['before'], edge['after'] = edge['after'], edge['before']
        s['orders'].append(edge)
        self.assertTrue(any('cycle' in e for e in validate_contact_case(self.case, self.findings)))

    def test_other_scenario_event_cannot_leak_into_order(self):
        self.case['scenarios'][0]['orders'][0]['before'] = 'permic-separate-loan'
        self.assertTrue(any('outside scenario' in e for e in validate_contact_case(self.case, self.findings)))

    def test_missing_basis_rejected(self):
        self.case['scenarios'][0]['orders'][0]['basis_findings'] = ['not-reviewed']
        self.assertTrue(any('unknown scenario finding' in e for e in validate_contact_case(self.case, self.findings)))

    def test_non_roster_packet_participates_in_publication_digest(self):
        from harness.coverage import pilot_coverage
        from harness.io_utils import digest
        summary = validate_packet(PACKET)
        with patch('harness.packets.packet_summaries', return_value=[summary]):
            before = pilot_coverage()
            self.assertEqual(before['editorial_packets'][0]['packet_id'], self.packet['id'])
            summary['packet_sha256'] = '0' * 64
            after = pilot_coverage()
        self.assertNotEqual(digest(before), digest(after))

    def test_root_environment_is_hashed(self):
        from harness.registry import context_hashes
        context = context_hashes()
        self.assertIn('pyproject.toml', context)
        self.assertNotIn('packages/dating/pyproject.toml', context)

    def test_alternatives_checked_separately(self):
        a, b = self.case['scenarios']
        b['event_ids'] = a['event_ids'][:]
        b['orders'] = copy.deepcopy(a['orders'])
        b['orders'][0]['before'], b['orders'][0]['after'] = b['orders'][0]['after'], b['orders'][0]['before']
        self.case['events'] = self.case['events'][:2]
        self.assertEqual(validate_contact_case(self.case, self.findings), [])

    def test_case_hash_and_export_guard(self):
        import hashlib
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'packet.json'
            (path.parent / 'case.json').write_text(json.dumps(self.case))
            earlier = PACKET.parent.parent / 'uralic-2026-09-19/claims.json'
            payload = {'claims': [], 'relations': json.loads(earlier.read_text())['relations'][:1]}
            raw = json.dumps(payload).encode()
            (path.parent / 'claims.json').write_bytes(raw)
            self.packet['claims_sha256'] = hashlib.sha256(raw).hexdigest()
            path.write_text(json.dumps(self.packet))
            # The case hash is independently enforced even with a refreshed claims hash.
            result = validate_packet(path)
            self.assertIn('contact scenario packet must not export registry claims or relations', result['errors'])
            payload['relations'] = []
            raw = json.dumps(payload).encode()
            (path.parent / 'claims.json').write_bytes(raw)
            self.packet['claims_sha256'] = hashlib.sha256(raw).hexdigest()
            path.write_text(json.dumps(self.packet))
            self.assertIn('packet contact case hash mismatch', validate_packet(path)['errors'])


class SemanticScoringTests(unittest.TestCase):
    cases = [{'id': 'unknown', 'expected': {'epoch': None, 'role': 'hypothesis'}}]

    def test_missing_null_field_is_not_a_correct_abstention(self):
        for pred in ({}, {'unknown': {}}):
            metrics = semantic_metrics(pred, self.cases)
            self.assertEqual(metrics['field_accuracy']['epoch'], 0)
        self.assertEqual(semantic_metrics({'unknown': {'epoch': None}}, self.cases)['field_accuracy']['epoch'], 1)

    def test_malformed_or_unknown_predictions_rejected(self):
        for pred in ([], {'unknown': None}, {'extra': {}}):
            with self.assertRaises(ValueError):
                semantic_metrics(pred, self.cases)


if __name__ == '__main__':
    unittest.main()
