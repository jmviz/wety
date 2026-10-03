import io,json,tempfile,unittest,zipfile
from pathlib import Path
from unittest.mock import patch
from workflow_tools import artifacts,runner
from workflow_tools.paths import REPO, historical
from source_tools import acquire

class Contracts(unittest.TestCase):
    def test_artifact_hash_detects_edit(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'files').mkdir();(p/'files/x').write_text('original')
            artifacts.atomic_json(p/'manifest.json',artifacts.manifest(p,'test','source',{}))
            self.assertEqual(artifacts.verify(p),[])
            (p/'files/x').write_text('changed')
            self.assertTrue(artifacts.verify(p))
    def test_zip_cannot_escape(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);z=p/'bad.zip'
            with zipfile.ZipFile(z,'w') as f:f.writestr('../escape','bad')
            with self.assertRaisesRegex(ValueError,'unsafe'):acquire.safe_extract_zip(z,p/'extract')
            self.assertFalse((p/'escape').exists())
    def test_download_checksum_and_repeat_receipts(self):
        class Response(io.BytesIO):
            status=200;headers={'Content-Type':'text/plain','Set-Cookie':'not-recorded'}
            def geturl(self):return 'https://example.org/source'
        with tempfile.TemporaryDirectory(dir=REPO/'data/cache') as d,patch.object(acquire,'DATA',Path(d)):
            with patch.object(acquire.urllib.request,'urlopen',side_effect=lambda *a,**k:Response(b'abc')):
                with self.assertRaisesRegex(ValueError,'checksum'):acquire.acquire('https://example.org/source','fixture',expected_sha256='0'*64)
                self.assertEqual(list(Path(d).glob('artifacts/*/*/manifest.json')),[])
                a=acquire.acquire('https://example.org/source','fixture')
                b=acquire.acquire('https://example.org/source','fixture')
                self.assertEqual(a,b);self.assertEqual(artifacts.verify(a),[])
                receipts=list(Path(d).glob('registry/acquisitions/*.json'));self.assertEqual(len(receipts),2)
                self.assertNotIn('Set-Cookie',receipts[0].read_text())
    def workflow(self,d,command,validation=()):
        p=Path(d)/'workflow.json';p.write_text(json.dumps({'schema_version':1,'mode':'artifact','collection':'fixture','product':'fixture','commands':[command],'validation':list(validation),'required_outputs':['result.txt']}));return p
    def test_success_publishes_and_failure_keeps_selection(self):
        with tempfile.TemporaryDirectory(dir=REPO/'data/cache') as d,patch.object(runner,'DATA',Path(d)):
            spec=self.workflow(d,['{python}','-c','from pathlib import Path; Path(r"{output}/result.txt").write_text("ok")'])
            first=runner.run(spec);self.assertEqual(first['status'],'complete')
            pointer=Path(d)/'products/fixture/CURRENT.json';before=pointer.read_bytes()
            spec=self.workflow(d,['{python}','-c','raise SystemExit(3)'])
            with self.assertRaisesRegex(RuntimeError,'failed'):runner.run(spec)
            self.assertEqual(pointer.read_bytes(),before)
            receipts=[json.loads(p.read_text()) for p in Path(d).glob('runs/*/receipt.json')]
            self.assertEqual({r['status'] for r in receipts},{'complete','failed'})
    def test_validation_failure_never_publishes(self):
        with tempfile.TemporaryDirectory(dir=REPO/'data/cache') as d,patch.object(runner,'DATA',Path(d)):
            spec=self.workflow(d,['{python}','-c','from pathlib import Path; Path(r"{output}/result.txt").write_text("bad")'],[['{python}','-c','raise SystemExit(2)']])
            with self.assertRaises(RuntimeError):runner.run(spec)
            self.assertFalse((Path(d)/'products/fixture/CURRENT.json').exists())
    def test_required_output_and_traversal_rejected(self):
        with tempfile.TemporaryDirectory(dir=REPO/'data/cache') as d,patch.object(runner,'DATA',Path(d)):
            spec=self.workflow(d,['{python}','-c','pass'])
            with self.assertRaisesRegex(ValueError,'missing'):runner.run(spec)
            s=json.loads(spec.read_text());s['required_outputs']=['../outside'];spec.write_text(json.dumps(s))
            with self.assertRaisesRegex(ValueError,'missing'):runner.run(spec)
    def test_historical_reference_is_exact(self):
        self.assertTrue(historical('cache/datasets/src_10_1126_science_abg0818/zenodo_8147476/IECoR_Suppl_Files_Light_Main_M3_Only.zip/01_Main_Analysis_M3/IECoR_Main_M3_Binary_Covarion_Rates_By_Mg_Bin/IECoR_Main_M3_Binary_Covarion_Rates_By_Mg_Bin_mcc.tree').is_file())
        with self.assertRaises(ValueError):historical('/someone/else/same-name.tree')
    def test_pos_order_preserved(self):
        import re
        baseline=REPO/'data/archive/reorganization-originals'
        # Committed numeric order, preserved from the pre-migration Rust table.
        order=json.loads((REPO/'packages/source_tools/pos_order.json').read_text())
        actual=re.findall(r'^    "([^"]+)",',(REPO/'crates/core/src/pos_phf.rs').read_text(),re.M)
        self.assertEqual(order,actual)

if __name__=='__main__':unittest.main()
