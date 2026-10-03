"""Source fidelity and semantic traps in the first bounded editorial packet."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from harness.config import DATA_DIR
from harness.packets import validate_packet
from harness.uralic_checks import check_supplements, xlsx_cells


PACKET = Path(DATA_DIR) / '_research/packets/uralic-2026-09-19/packet.json'


class UralicPacketTests(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads(PACKET.read_text())
        self.payload = json.loads((PACKET.parent / 'claims.json').read_text())
        self.claims = {c['provenance']['packet_key']: c for c in self.payload['claims']}

    def check_mutation(self, mutate, rehash=True):
        with tempfile.TemporaryDirectory() as tmp:
            mutate(self.packet, self.payload)
            raw = json.dumps(self.payload).encode()
            if rehash:
                self.packet['claims_sha256'] = hashlib.sha256(raw).hexdigest()
            path = Path(tmp) / 'packet.json'
            path.write_text(json.dumps(self.packet))
            (path.parent / 'claims.json').write_bytes(raw)
            return validate_packet(path)['errors']

    def test_complete_packet_and_review_limits(self):
        result = validate_packet(PACKET)
        self.assertEqual(result['errors'], [])
        self.assertEqual((result['claims'], result['findings'], result['relations']), (13, 18, 2))
        self.assertFalse(result['independent_review'])
        self.assertEqual(result['evaluation_split'], 'development')
        self.assertEqual(sum(c['status'] == 'supported' for c in self.claims.values()), 5)
        self.assertTrue(all(c['subject']['catalog_mapping'] == 'unresolved' for c in self.claims.values()))
        self.assertTrue(all(not c['subject']['id'].startswith('wikt:') for c in self.claims.values()))

    def test_maurits_epoch_is_local_and_priors_are_not_hpd(self):
        for name in ('finnic', 'permic', 'saamic'):
            c = self.claims['maurits-prior-' + name]
            self.assertEqual(c['role'], 'calibration_prior')
            self.assertEqual(c['time']['original']['present'], 2000)
            self.assertEqual(c['time']['interval_type'], 'reported_range')
        finn = self.claims['maurits-prior-finnic']['time']
        self.assertEqual(finn['mean'], 775)
        self.assertEqual(finn['original']['reported_rounded_mean_years_bp'], 1230)
        perm = self.claims['maurits-prior-permic']['time']
        self.assertEqual((perm['lower'], perm['upper'], perm['mean']), (737, 1339, None))
        self.assertEqual(perm['original']['raw_parameters']['log_mean'], -0.18)
        for c in self.claims.values():
            if c['source_id'] != 'src_10_1093_jole_lzz009':
                self.assertEqual(c['time']['form'], 'unknown')
                self.assertIsNone(c['time']['original'].get('present'))

    def test_relative_argument_does_not_manufacture_absolute_interval(self):
        divergence = self.claims['grunthal-divergence']
        contact = self.claims['grunthal-contact']
        changes = self.claims['grunthal-branch-changes']
        self.assertEqual(divergence['event'], 'linguistic_divergence')
        self.assertEqual(changes['event'], 'sound_change')
        self.assertNotIn('lower', divergence['time']['original'])
        edges = {(r['before_event'], r['after_event']) for r in self.payload['relations']}
        self.assertEqual(edges, {(divergence['id'], contact['id']), (contact['id'], changes['id'])})
        for r in self.payload['relations']:
            basis = next(c for c in self.claims.values() if c['id'] == r['basis_claim_ids'][0])
            self.assertEqual((basis['status'], basis['role']), ('supported', 'hypothesis'))

    def test_candidate_spans_checked_even_after_rehash(self):
        def change(packet, payload):
            span = payload['claims'][0]['spans'][0]
            span['quote'] = '?' + span['quote'][1:]
        self.assertTrue(any('span mismatch' in e for e in self.check_mutation(change)))

    def test_changed_claim_file_rejected(self):
        self.assertIn('packet claims hash mismatch', self.check_mutation(lambda p, c: None, rehash=False))

    def test_source_hash_and_inventory_checked(self):
        def change(packet, payload):
            packet['source_reviews'][0]['raw_sha256'] = '0' * 64
            packet['source_reviews'][0]['claim_ids'] = []
        errors = self.check_mutation(change)
        self.assertTrue(any('raw hash mismatch' in e for e in errors))
        self.assertTrue(any('inventory mismatch' in e for e in errors))

    def test_import_check_includes_relative_relations(self):
        imported = {c['id']: c for c in self.payload['claims']}
        result = validate_packet(PACKET, imported, [])
        self.assertIn('packet relation missing or changed in registry', result['errors'])
        self.assertEqual(validate_packet(PACKET, imported, self.payload['relations'])['errors'], [])


class UralicSupplementTests(unittest.TestCase):
    path = Path(DATA_DIR) / '_research/packets/uralic-supplements-2026-09-19/packet.json'

    def test_recovered_sources_and_exact_spans(self):
        result = validate_packet(self.path)
        self.assertEqual(result['errors'], [])
        self.assertEqual((result['claims'], result['findings']), (0, 10))
        self.assertEqual(sum(result['finding_counts'].values()), 10)
        self.assertFalse(result['independent_review'])

    def test_published_numerical_results_reproduce(self):
        result = check_supplements(self.path)
        self.assertTrue(result['passed'])
        self.assertEqual(len(result['grunthal_table_s4_2']), 5)
        for row in result['grunthal_table_s4_2']:
            self.assertEqual((row['PU']['n'], row['II']['n']), (8, 9))
        self.assertAlmostEqual(result['maurits_permic']['positive_age_probability'], .999995, places=6)

    def test_lexical_uncertainty_survives_workbook_reading(self):
        from workflow_tools.paths import resolve
        p = json.loads(self.path.read_text())
        source = next(s for s in p['source_reviews'] if s['key'] == 'loans-workbook')
        cells = xlsx_cells(resolve(source['raw_path']))['Indo-Iranian']
        self.assertEqual(cells['DW41']['value'], '?')
        self.assertEqual(cells['DW42']['value'], '?')
        self.assertIn('parallel borrowing', cells['DW12']['value'])
        self.assertIn('regular before *t', cells['DW12']['value'])


if __name__ == '__main__':
    unittest.main()
