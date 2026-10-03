"""Catalog concepts, documentary samples and dated entities cannot be merged."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch
from harness import glotto
from harness.bootstrap import iecor
from harness.config import IECOR_DIR


class CatalogIntegration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.g=glotto.load()
    def test_family_is_not_a_reconstructed_language(self):self.assertEqual(self.g.status('indo1319'),'unknown')
    def test_unknown_tip_makes_mrca_unresolved(self):self.assertIsNone(self.g.mrca(['stan1293','missing']))
    def test_nongenealogical_bucket_is_not_a_family(self):
        nodes=[g for g,r in self.g.rows.items() if r.get('category')=='Bookkeeping']
        self.assertTrue(nodes)
        for g in nodes[:10]:
            self.assertEqual(self.g.top_family(g),glotto.UNCLASSIFIED_BUCKET)
            self.assertEqual(self.g.genetic_path(g),[])
            self.assertIsNone(self.g.mrca([g]))
    def test_proto_associations_only_explicit_context(self):
        self.assertNotIn('urj-ugr-pro',self.g.wikt_by_glotto.get('ural1272',[]))
        self.assertNotIn('urj-fpr-pro',self.g.proto_by_family_node.get('ural1272',[]))
    def test_source_samples_survive_proposals(self):
        rows=json.loads((Path(IECOR_DIR)/'iecor_wiktionary_matches.json').read_text())
        by_id={r['iecor']['ID']:r for r in rows}
        self.assertEqual(len(rows),160)
        for code in ('230','245','259','157'):
            self.assertIsNone(by_id[code]['wiktionary'])
            self.assertEqual(by_id[code]['sample_id'],'iecor:sample:'+code)
    def test_ambiguous_wikidata_context_preserves_unplaced_claim(self):
        from harness.bootstrap import wikidata
        from types import SimpleNamespace
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)/'sources';root.mkdir()
            for name,data in {'wikidata_items.json':{'Q1':{'label':'source concept','inception':['+0100-01-01T00:00:00Z']}},'wikidata_codes.json':{'Q1':{'glottocode':['a','b']}},'manifest.json':{'files':{'wikidata_items.json':{'retrieved':'2026-09-19'}}}}.items():
                (root/name).write_text(json.dumps(data))
            g=SimpleNamespace(rows={'a':{},'b':{}},wikt={})
            with patch.object(wikidata,'LANGCODES_DIR',tmp),patch.object(wikidata.glotto,'load',return_value=g),patch.object(wikidata,'SOURCE',wikidata.SOURCE.model_copy(deep=True)),patch.object(wikidata,'write_claims',side_effect=lambda g,s,c:c):
                claims=wikidata.run()
            self.assertEqual(len(claims),1);self.assertIsNone(claims[0].languoid)
            self.assertEqual(claims[0].evidence['subject_id'],'wikidata:Q1')
            self.assertEqual(claims[0].evidence['catalog_candidates'],['a','b'])
    def test_unknown_catalog_codes_do_not_drop_sample_priors(self):
        from types import SimpleNamespace
        g=SimpleNamespace(rows={},crossref_generation=self.g.crossref_generation)
        with patch.object(iecor.glotto,'load',return_value=g),patch.object(iecor,'write_claims',side_effect=lambda g,s,c:c):claims=iecor.run()
        self.assertTrue(claims)
        self.assertTrue(all(c.languoid is None for c in claims))
        self.assertTrue(all(c.evidence['raw_prior'].get('Glottocode') for c in claims))
        self.assertEqual(len({c.evidence['subject_id'] for c in claims}),len(claims))
    def test_importer_keeps_raw_sample_name_and_catalog_code(self):
        with patch.object(iecor,'write_claims',side_effect=lambda g,s,c:c):claims=iecor.run()
        self.assertTrue(claims)
        for c in claims:
            self.assertEqual(c.languoid,c.evidence['raw_prior']['Glottocode'])
            self.assertEqual(c.languoid_name,c.evidence['raw_prior']['Name'])
            self.assertEqual(c.evidence['subject_id'],'iecor:sample:'+c.evidence['raw_prior']['ID'])
            self.assertEqual(c.evidence['catalog_mapping'],'unresolved')
            self.assertTrue(c.time.unknown)


if __name__=='__main__':unittest.main()
