# Etymology sources

This package holds the assertion schema, the converters that turn each etymological
source into assertions, and the curation tools those converters read. The design and
the work plan are in the shared doc "wety etymology claims model", outside git.

- [`schema/`](schema/README.md): the assertion schema, v3, and what it covers;
- `assertions.py`: builds assertion objects (their digest ids) and validates files;
- `convert/`: one converter per source. `acd` and `lila` read the retained release, whole
  or a sample; `wiktionary` converts claims a model annotated per entry, for samples;
  `wiktionary_references` reads every References section of the pages dump (core-5),
  for the graph build.
  Wiktionary's templates are read by the processor instead: `graph-assertions`
  (`crates/processor`) writes a graph's assertions in this schema, which
  `etymology.assertions` validates;
- `wikitext.py`: pages of a Wiktionary dump, their sections and templates;
- `language_map.py`, `orthography.py`: curation tools (language maps, comparison keys);
- `match_acd.py`: ACD records matched to Wiktionary's, and relays (below).
- `match_lila.py`: LiLa's records matched to Wiktionary's, and relays (below).

## A source's directory

Each source has one directory, `data/workspaces/etymology/sources/<source>/`, laid out
as `cldfbench` lays out a dataset:

- `raw/`: the source as published, unpacked from its retained artifact
  (`data/artifacts/<collection>/<sha256>/`, acquired with `wety-sources`);
- `etc/`: what wety's curators add: `source.json` (the source profile), the language
  map, the kind map and, where forms need it, an orthography profile and readings of
  forms a converter's rules cannot read (`forms.json`). These are reviewed like code and
  tracked in git;
- `checks/`: what the milestone checks measured, with the samples they read.

`raw/` and `checks/` stay in the research store.

## Converting

Each converter writes an assertions file, which `etymology.assertions` validates. The ACD
converter reads the whole release, compiles the links its tree implies, and writes its
own counts; `checks/acd-2/acd_counts.py` compares them with the release's tables. The
LiLa converter takes the core-2 sample here (see below for the whole release), the
Wiktionary converter its annotated sample. From the main checkout:

```sh
S=data/workspaces/etymology/sources
uv run --frozen python -m etymology.convert.acd --out $S/acd/checks/acd-2/assertions.jsonl --counts $S/acd/checks/acd-2/converter_counts.json
uv run --frozen python -m etymology.convert.lila --sample $S/lila-edl/checks/core-2/sample.json --out $S/lila-edl/checks/core-2/assertions.jsonl
uv run --frozen python -m etymology.convert.wiktionary $S/wiktionary/checks/core-2/sample.json $S/wiktionary/checks/core-2/claims.json --out $S/wiktionary/checks/core-2/assertions.jsonl
uv run --frozen python -m etymology.assertions --summary $S/acd/checks/acd-2/assertions.jsonl $S/*/checks/core-2/assertions.jsonl
```

## Converting a whole source

The LiLa converter reads the whole release when it is given no sample (lila-2). Written
forms its rules cannot read are read in `etc/forms.json`, and it writes every decision
it took:

```sh
uv run --frozen python -m etymology.convert.lila --out $S/lila-edl/checks/lila-2/assertions.jsonl --decisions $S/lila-edl/checks/lila-2/decisions.json
```

## Wiktionary's References

wiktextract has no field for References, so the workflow `wiktionary_references` reads
them from the pages dump of the same date (`etc/source.json`, artifact `pages`) and
writes the product the graph build reads:

```sh
uv run --frozen wety-data run wiktionary_references
```

`extract` keeps the dump's References and Further reading sections and its
`Template:R:` pages; `convert` writes the bibliography (`bibliography.jsonl`) and each
References section's credits (`section_credits.jsonl.gz`), and checks them. The
bibliography has a work for every `Template:R:` page that is not a redirect, read from
the citation template it calls (authors, or editors when it names none; year; title),
and for every `{{cite-...}}` line of a References section. A section's works are credits
of role `follows`, scope `entry`, keyed by page, language heading and etymology label
(none for a section beside the etymologies). The processor joins them to its own items:
a section under "Etymology N" covers that etymology, one beside the etymologies every
etymology of its language, and an item's own section's assertions carry them, unless a
rule reads the section. `<ref>` footnotes are left to the processor (note scope);
Further reading gives no credits and is kept the same way
(`further_reading.jsonl.gz`).

## Matching the ACD to Wiktionary

`match_acd.py` (acd-3) links the ACD's records to Wiktionary's, which stand for wety's
items, and Wiktionary's assertions that credit the ACD to the ACD's own. `scan` keeps the
Austronesian part of a graph's assertion export and the glosses of Austronesian entries
from the wiktextract data it was built from; `run` writes the `match` and `relay` objects
with everything they refer to, every record's outcome (`decisions.jsonl.gz`), every
crediting assertion's outcome (`relays.json`), counts and a sample of 100 matches to
review; given an earlier run's check directory (`--previous`), it reviews what changed
instead (`changed-sample.json`, and `previous-sample.json`: the earlier reviewed sample
with each match's outcome now). From a checkout whose `data/` holds the inputs:

```sh
W=<a work directory>; C=$S/acd/checks/acd-3-ancestry
E=data/runs/ccaf05246fcb466f94916b3f5ae6654a/checks/core-4-references/assertions.jsonl.gz
X=data/artifacts/wiktextract/6da0ba5bf330650cba59911466da388d3f60c69519a50dc9ca71451b0bfba506/files/raw-wiktextract-data.jsonl.gz
uv run --frozen python -m etymology.match_acd scan --export $E --wiktextract $X --work $W
uv run --frozen python -m etymology.match_acd run --acd $S/acd/checks/acd-2/assertions.jsonl --export $E --wiktextract $X --work $W --out $C --previous $S/acd/checks/acd-3
```

The matcher, rule `agent:wety-match-acd@2`, takes only languages the ACD's map calls
`exact`. Forms are compared by comparison keys, from one orthography profile per source
for reconstructions (`acd/etc/orthography.tsv`, `wiktionary/etc/orthography-map.tsv`)
and one for attested forms (`orthography-attested.tsv`, `orthography-map-attested.tsv`);
an affix meets only an affix of its kind (or a word, failing one). A reconstruction is
weighed by how many of the reflexes the ACD cites for it are its candidate's descendants
in wety; every record by how many of its matched ACD ancestors are its candidate's
ancestors in wety (`basis.ancestors`, so reconstructions are matched first, in rounds),
and by its gloss. A candidate whose ancestry names other words at an ACD ancestor's level
is taken only when its gloss confirms it. Equal fits stay `candidates`; of several ACD
records fitting one Wiktionary record (homographs, doublets), the best is matched, an
equal key beating a near one. v1, which weighed no ancestors, is reviewed in
`checks/acd-3/`. The relay rule, `agent:wety-relay-acd@1`, compares by ancestry, since the
ACD links a reflex straight to its set's reconstruction. The docstrings of `Matcher` and
`Relayer` give the rules in full.

## Matching LiLa to Wiktionary

`match_lila.py` (lila-3) matches LiLa's de Vaan records to Wiktionary's records, which
stand for wety's items, and relays Wiktionary's credits to de Vaan (2008) to his
assertions in LiLa. It reads a graph's assertion export (`graph-assertions`) and the
lila-2 assertions; both must lie in the checkout, so hard-link them in a worktree:

```sh
uv run --frozen python -m etymology.match_lila --export data/runs/<run>/checks/core-4-references/assertions.jsonl.gz --out $S/lila-edl/checks/lila-3
uv run --frozen python -m etymology.match_lila --out $S/lila-edl/checks/lila-3 --review
```

The first writes `assertions.jsonl` (the matches, the relays and what they refer to),
`relays.json` (every crediting assertion and what became of it), `result.json` and
`sample.json` (100 matches to review). The second adds the precision of the judged
sample, `review.json`, to `result.json`. The rule, `agent:wety-match-lila@1`, and what
each match weighs are in the module's docstring.

## Tests

```sh
uv run --frozen python -m unittest discover -s packages/etymology/tests
```
