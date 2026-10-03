"""Coverage means evidence coverage, not guessed language lifespans."""
from .registry import current_bundle, report


def write(view=None):
    return report(current_bundle())
