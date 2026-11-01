# Etymology sources

This package will hold the assertion schema, the converters that turn each etymological
source into assertions, and the curation tools those converters read. The design and
the work plan are in the shared doc "wety etymology claims model", outside git.

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

## Tests

```sh
uv run --frozen python -m unittest discover -s packages/etymology/tests
```
