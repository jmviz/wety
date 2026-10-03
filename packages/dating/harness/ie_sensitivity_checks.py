"""Reproduce selected deposited IE sensitivity summaries, without importing dates."""
import csv
import hashlib
import io
import json
from pathlib import Path
import statistics
import xml.etree.ElementTree as ET
import zipfile
from workflow_tools.paths import REPO

ARCHIVE_SHA256 = 'ec6cdfc18b9e3ae96599e71ce2bd81588e397d6921a2db04ff6c2c404a9d3482'
ARCHIVE = REPO/'data/artifacts/dating-ie-sensitivity'/ARCHIVE_SHA256/'files/source'
SCENARIOS = {
    'SA7a': 'IECoR_SA7a_Anc_Constrs_01_Latin_Only',
    'SA8a': 'SA8a_M3_on_Chang_Broad_Data_NO_Anc_Constrs',
    'SA8b': 'SA8b_M3_on_Chang_Broad_Data_WITH_Anc_Constrs',
}


def check_deposit(path=ARCHIVE):
    raw = Path(path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != ARCHIVE_SHA256:
        raise ValueError('IE sensitivity archive hash mismatch')
    results = {}
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        for key, name in SCENARIOS.items():
            prefix = 'IECoR_Suppl_Files_Light/03_Sensitivity_Analyses_SA1_to_SA10/'+name+'/'+name
            xml_bytes = archive.read(prefix+'.xml')
            log_bytes = archive.read(prefix+'_combined.log')
            root = ET.fromstring(xml_bytes)
            reader = csv.DictReader((line for line in log_bytes.decode().splitlines() if not line.startswith('#')), delimiter='\t')
            rows = list(reader)
            if len(rows) != 27003 or len({r['Sample'] for r in rows}) != 27003:
                raise ValueError('unexpected combined log sampling')
            heights = [float(r['TreeHeight.t:tree']) for r in rows]
            sequences = sorted((e.get('taxon'), e.get('value')) for e in root.findall('data/sequence'))
            ancestors = [e for e in root.iter() if e.get('spec','').endswith('AncestryConstraint')]
            results[key] = {
                'files': {suffix: {'member': prefix+suffix, 'sha256': hashlib.sha256(b).hexdigest()}
                          for suffix,b in [('.xml',xml_bytes),('_combined.log',log_bytes)]},
                'retained_rows': len(rows), 'median_native_tree_height': statistics.median(heights),
                'mean_native_tree_height': statistics.mean(heights),
                'comparison_median_years_bp': statistics.median(heights)*1000,
                'taxa': [name for name,value in sequences],
                'alignment_sha256': hashlib.sha256(json.dumps(sequences).encode()).hexdigest(),
                'ancestry_constraints': [e.get('id') for e in ancestors],
                'monophyly_labelled_distributions': [e.get('id') for e in root.iter() if e.get('id','').startswith('monophyly(')],
            }
    return {
        'archive_sha256': ARCHIVE_SHA256,
        'scope': 'Development numerical/configuration audit; no production claims, rerun or convergence validation.',
        'selection': 'All rows of author-combined logs; no second burn-in. Guide sections 4.2 and 5.4 describe combination and per-run burn-in.',
        'scale': {'factor':1000, 'basis':'Explicit comparison convention, anchored to the published SA8a/SA8b year-BP medians and the existing main-M3 analysis scale; native heights retained. Not an inferred calendar epoch or production normalization.'},
        'epoch': None,
        'convergence': 'Independent chains and chain-specific diagnostics not verified from these combined logs.',
        'scenarios': results,
        'checks': {
            'reported_sa8_medians': [round(results[k]['comparison_median_years_bp']) for k in ('SA8a','SA8b')]==[8629,4997],
            'sa7a_supports_331_reduction_not_printed_7889': round(results['SA7a']['comparison_median_years_bp'])==7789,
            'same_sa8_alignment': results['SA8a']['alignment_sha256']==results['SA8b']['alignment_sha256'],
            'expected_ancestry_counts': [len(results[k]['ancestry_constraints']) for k in SCENARIOS]==[1,0,8],
        },
    }


def main():
    result = check_deposit()
    print(json.dumps(result, indent=2))
    raise SystemExit(not all(result['checks'].values()))


if __name__ == '__main__': main()
