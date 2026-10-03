"""Validate typed evidence, provenance, dependencies and published export safety."""
from workflow_tools.paths import DATING_PRODUCT, DATING_GENERATIONS, DATING_HISTORY

import json
from pathlib import Path
from .config import DATA_DIR
from .registry import current_bundle, validate_bundle, build_bundle
from .io_utils import digest


def validate_all():
    try:
        bundle=current_bundle()
        errors=validate_bundle(bundle)
        if digest(build_bundle()) != digest(bundle):
            errors.append('published generation or decisions are stale; run dating synthesize')
        for filename,key in [('wiktionary_dates','view'),('evidence','claims'),('review_queue','review_queue'),('rejected_decisions','rejected_decisions'),('research_graph','research_graph'),('pilot_coverage','pilot_coverage')]:
            path=Path(DATING_PRODUCT)/(filename+'.json')
            if json.loads(path.read_text())!=bundle[key]: errors.append('export mirror differs from canonical generation: '+filename)
        import csv
        with (Path(DATING_PRODUCT)/'wiktionary_dates.csv').open() as f:
            rows=list(csv.DictReader(f))
        expected=[{'code':v['code'],'name':v['name'],'display_status':v['display_status'],'chronology_json':json.dumps(v['chronology'],ensure_ascii=False)} for v in bundle['view'].values()]
        if rows!=expected: errors.append('CSV mirror differs from canonical generation')
        from . import glotto
        grouped={}
        for gid,r in bundle['records'].items(): grouped.setdefault(glotto.load().top_family(gid),{})[gid]=r
        for family,recs in grouped.items():
            if json.loads((Path(DATA_DIR)/family/'languoids.json').read_text()) != recs:
                errors.append('record mirror differs: '+family)
        return errors
    except (ValueError,KeyError,OSError) as e:
        return [str(e)]
