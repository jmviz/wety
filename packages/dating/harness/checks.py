"""Semantic integrity checks operate on the final generation, after decisions."""
from .registry import validate_bundle


def check_analysis_edges(edges, claims):
    """Chronological comparisons require exact relations within one analysis.

    Compare hard nonoverlapping ranges only; conflicting marginal estimates from
    different studies do not constitute a violated evolutionary constraint.
    """
    errors=[]
    for parent,child in edges:
        a,b=claims[parent],claims[child]
        if not a.get('analysis_id') or a['analysis_id']!=b.get('analysis_id'):
            errors.append('incomparable analyses: '+parent+' / '+child); continue
        ta,tb=a['time'],b['time']
        # Conditional marginal posterior intervals are not hard historical bounds.
        if ta.get('form')=='distribution' or tb.get('form')=='distribution':
            continue
        pa=ta.get('lower') if ta.get('lower') is not None else ta.get('value')
        cb=tb.get('upper') if tb.get('upper') is not None else tb.get('value')
        if pa is not None and cb is not None and pa>cb:
            errors.append('parent younger than child: '+parent+' / '+child)
    return errors
