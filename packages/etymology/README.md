# Etymology sources

This package holds the assertion schema, the converters that turn each etymological
source into assertions, and the curation tools those converters read. The design and
the work plan are in the shared doc "wety etymology claims model", outside git.

- [`schema/`](schema/README.md): the assertion schema, v1, and what it covers;
- `assertions.py`: builds assertion objects (their digest ids) and validates files;
- `convert/`: one converter per source. `acd` and `lila` read the retained release, whole
  or a sample; `wiktionary` converts claims annotated per entry, for samples, until the
  processor emits Wiktionary's assertions (core-4); `wiktionary_references` reads every
  References section of the pages dump (core-5);
- `wikitext.py`: pages of a Wiktionary dump, their sections and templates;
- `language_map.py`, `orthography.py`: curation tools (language maps, comparison keys).

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

wiktextract has no field for References, so `wiktionary_references` reads them from the
pages dump of the same date (`etc/source.json`, artifact `pages`), in three steps: the
dump's References and Further reading sections and `Template:R:` pages; wiktextract's
entry keys (page, language, etymology label); then the bibliography and the credits.

```sh
C=$S/wiktionary/checks/core-5
uv run --frozen python -m etymology.convert.wiktionary_references extract <pages dump> --out $C/sections.jsonl.gz --counts $C/extract_counts.json
uv run --frozen python -m etymology.convert.wiktionary_references entries <raw wiktextract> --out $C/entries.jsonl.gz
uv run --frozen python -m etymology.convert.wiktionary_references convert $C/sections.jsonl.gz $C/entries.jsonl.gz --out $C
```

The bibliography (`assertions.jsonl`) has a work for every `Template:R:` page that is not
a redirect, read from the citation template it calls (authors, or editors when it names
none; year; title), and for every `{{cite-...}}` line of a References section. Each
entry's References give credits of role `follows`, scope `entry`
(`entry_credits.jsonl.gz`). A row names its entry as the processor does: page title,
language code, etymology label and number, and the record id `graph-assertions` gives
the word (`wiktionary:<code>:<term>#<n>`; where another page took the same term and
number first, the processor raises the number, so join on page, code and label). A section beside the
etymologies covers every etymology of its language; one under "Etymology N", only that
one. `<ref>` footnotes are left to the processor (note scope) and Further reading gives
no credits. Until the processor emits Wiktionary's assertions, the credits wait in that
table; `convert` checks each against the schema's `Credit` and the bibliography, and the
processor will copy an entry's credits onto each assertion the entry endorses.

## Tests

```sh
uv run --frozen python -m unittest discover -s packages/etymology/tests
```
