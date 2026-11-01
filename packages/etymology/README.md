# Etymology sources

This package holds the assertion schema, the converters that turn each etymological
source into assertions, and the curation tools those converters read. The design and
the work plan are in the shared doc "wety etymology claims model", outside git.

- [`schema/`](schema/README.md): the assertion schema, v0, and what it covers;
- `assertions.py`: builds assertion objects (their digest ids) and validates files;
- `convert/`: one converter per source. `acd` (the whole release) and `lila` read the
  retained release;
  `wiktionary` converts claims annotated per entry, for samples, until the processor
  emits Wiktionary's assertions (core-4);
- `language_map.py`, `orthography.py`: curation tools (language maps, comparison keys).

## A source's directory

Each source has one directory, `data/workspaces/etymology/sources/<source>/`, laid out
as `cldfbench` lays out a dataset:

- `raw/`: the source as published, unpacked from its retained artifact
  (`data/artifacts/<collection>/<sha256>/`, acquired with `wety-sources`);
- `etc/`: what wety's curators add: `source.json` (the source profile), the language
  map, the kind map and, where forms need it, an orthography profile. These are
  reviewed like code and tracked in git;
- `checks/`: what the milestone checks measured, with the samples they read.

`raw/` and `checks/` stay in the research store.

## Converting

Each converter writes an assertions file, which `etymology.assertions` validates. The ACD
converter reads the whole release, compiles the links its tree implies, and writes its
own counts; `checks/acd-2/acd_counts.py` compares them with the release's tables. The
LiLa and Wiktionary converters take the core-2 samples. From the main checkout:

```sh
S=data/workspaces/etymology/sources
uv run --frozen python -m etymology.convert.acd --out $S/acd/checks/acd-2/assertions.jsonl --counts $S/acd/checks/acd-2/converter_counts.json
uv run --frozen python -m etymology.convert.lila $S/lila-edl/checks/core-2/sample.json --out $S/lila-edl/checks/core-2/assertions.jsonl
uv run --frozen python -m etymology.convert.wiktionary $S/wiktionary/checks/core-2/sample.json $S/wiktionary/checks/core-2/claims.json --out $S/wiktionary/checks/core-2/assertions.jsonl
uv run --frozen python -m etymology.assertions --summary $S/acd/checks/acd-2/assertions.jsonl $S/*/checks/core-2/assertions.jsonl
```

## Tests

```sh
uv run --frozen python -m unittest discover -s packages/etymology/tests
```
