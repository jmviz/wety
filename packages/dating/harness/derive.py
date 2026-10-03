"""Wiktionary views are published as part of an atomic registry generation."""
from .registry import build_bundle, publish


def build(claim_by_id=None):
    return build_bundle()['view']


def write(view=None):
    # Never publish a view independently of its evidence and decisions.
    return publish()
