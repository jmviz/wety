"""Version 2 synthesis preserves study-specific evidence and abstains by default."""
from .registry import build_bundle, publish


def synthesize(run=None, only_family=None):
    from . import glotto
    bundle=build_bundle(); result={}
    for gid,r in bundle['records'].items():
        family=glotto.load().top_family(gid)
        if only_family is None or family==only_family:
            result.setdefault(family,{})[gid]=r
    return result


def claim_point(c):
    """A statistic for diagnostics only; never midpoint an interval."""
    t=c['time']
    return next((t[k] for k in ('value','median','mean') if t.get(k) is not None),None)


def claim_interval(c):
    t=c['time']
    return t.get('lower',t.get('earliest')), t.get('upper',t.get('latest'))
