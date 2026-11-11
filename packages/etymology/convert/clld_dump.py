"""The tables of a clld app's PostgreSQL dump (`pg_dump` plain text, gzipped), for sources
published only as an app deposit (the CSD, the Comparative Cariban Database).

A table is read from its `COPY public.<table> (<columns>) FROM stdin;` block: one row per
line, columns separated by tabs, `\\N` for null, and backslash escapes for tab, newline,
carriage return and backslash.
"""

import gzip
import re

COPY = re.compile(r"^COPY public\.(\w+) \((.*)\) FROM stdin;$")
ESCAPES = {"t": "\t", "n": "\n", "r": "\r", "\\": "\\"}
ESCAPE = re.compile(r"\\(.)")


def value(field):
    if field == "\\N":
        return None
    return ESCAPE.sub(lambda m: ESCAPES.get(m.group(1), m.group(1)), field)


def tables(path, wanted=None):
    """Each table's rows as dicts, by table name; only the `wanted` ones if given."""
    out, name, columns = {}, None, None
    with gzip.open(path, "rt", encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if name is None:
                m = COPY.match(line)
                if m and (wanted is None or m.group(1) in wanted):
                    name = m.group(1)
                    columns = [c.strip().strip('"') for c in m.group(2).split(",")]
                    out[name] = []
                continue
            if line == "\\.":
                name = None
                continue
            out[name].append(dict(zip(columns, map(value, line.split("\t")))))
    return out
