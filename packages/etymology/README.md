# Etymology sources

This package holds the assertion schema, the converters that turn each etymological
source into assertions, and the curation tools those converters read. The design and
the work plan are in the shared doc "wety etymology claims model", outside git.

- [`schema/`](schema/README.md): the assertion schema, v5, and what it covers;
- `assertions.py`: builds assertion objects (their digest ids) and validates files;
- `convert/`: one converter per source. `acd` and `lila` read the retained release, whole
  or a sample; `wiktionary` converts claims a model annotated per entry, for samples;
  `wiktionary_references` reads the citation templates of every References section
  and footnote of wety's entries (core-5, x-5), for the graph build.
  Wiktionary's templates are read by the processor instead: `graph-assertions`
  (`crates/processor`) writes a graph's assertions in this schema, which
  `etymology.assertions` validates;
- `wikitext.py`: pages of a Wiktionary dump, their sections and templates;
- `language_map.py`, `orthography.py`: curation tools (language maps, comparison keys);
  `conventions/`: orthography profiles several sources write in, which a source's
  profile includes;
- `matching.py`: what every matcher shares (below); `notation.py`: how two sources'
  keys differ where they are matched, mined into a distance;
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

The workflow `wiktionary_references` reads them from wety's entries (`wiktionary_entries`)
and the pages dump of the same date (`etc/source.json`, artifact `pages`), and writes the
product the graph build reads:

```sh
uv run --frozen wety-data run wiktionary_references
```

`extract` keeps the dump's `Template:R:` pages; `convert` writes the bibliography
(`bibliography.jsonl`) and the credits of each References section and footnote
(`section_credits.jsonl.gz`), and checks them. The bibliography has a work for every
`Template:R:` page that is not a redirect, read from the citation template it calls
(authors, or editors when it names none; year; title), and for every `{{cite-...}}` call.
A References section's works are credits of role `follows`, scope `entry`, keyed by page,
language heading and etymology label (none for a section beside the etymologies). The
processor joins them to its own items: a section under "Etymology N" covers that
etymology, one beside the etymologies every etymology of its language, and an item's own
section's assertions carry them, unless a rule reads the section. A footnote's works are
credits of scope `note`, keyed by its section and note id: a footnote citing
`{{R:itc:EDL}}` credits the same work as a References line citing it. The processor
reads a footnote citing no template from its text. Further reading gives no credits and
is kept the same way (`further_reading.jsonl.gz`).

## Matching the ACD to Wiktionary

`match_acd.py` (acd-3) links the ACD's records to Wiktionary's, which stand for wety's
items, and Wiktionary's assertions that credit the ACD to the ACD's own. `scan` keeps the
Austronesian part of a graph's assertion export and the glosses of Austronesian entries
from the entries it was built from (`wiktionary_entries`); `run` writes the `match` and `relay` objects
with everything they refer to, every record's outcome (`decisions.jsonl.gz`), every
crediting assertion's outcome (`relays.json`), counts and a sample of 100 matches to
review; given an earlier run's check directory (`--previous`), it reviews what changed
instead (`changed-sample.json`), and the ties and least probable targets for a reviewer
(`hard.json`). Every run is scored against the judgements of all earlier reviews
(`counts.json`: `judgements`, and `held out`, five folds scored on judgements the fit did
not see); `review` adds a check's judged `review.json` and `hard.json` to them. From a
checkout whose `data/` holds the inputs:

```sh
W=<a work directory>; C=$S/acd/checks/acd-3-v4
E=data/runs/ccaf05246fcb466f94916b3f5ae6654a/checks/core-4-references/assertions.jsonl.gz
X=data/artifacts/wiktionary-entries/466445a2b9d76a586ab53d059f8c9895b4f31141f20b1666f0d2c33aec73bd06/files/entries.jsonl.gz
L=data/artifacts/wiktionary-export/57d16abeffbce475ff189e0734fd8937eab8d7417b1ee08273e746b43b350cb0/files
uv run --frozen python -m etymology.match_acd scan --export $E --entries $X --languages $L/languages.json --families $L/families.json --work $W
uv run --frozen python -m etymology.match_acd run --acd $S/acd/checks/acd-2/assertions.jsonl --export $E --entries $X --work $W --out $C --previous $S/acd/checks/acd-3-v3
uv run --frozen python -m etymology.match_acd review --out $C
```

The matcher, rule `agent:wety-match-acd@4`, takes only languages the ACD's map calls
`exact`. Forms are compared by comparison keys, from one orthography profile per source
for reconstructions (`acd/etc/orthography.tsv`, `wiktionary/etc/orthography-map.tsv`)
and one for attested forms (`orthography-attested.tsv`, `orthography-map-attested.tsv`);
an affix meets only an affix of its kind (or a word, failing one, or for a reconstruction
whose shape has only records Wiktionary cites, Wiktionary's bound stems). A reconstruction is
weighed by how many of the reflexes the ACD cites for it are its candidate's descendants
in wety; every record by how many of its matched ACD ancestors are its candidate's
ancestors in wety (`basis.ancestors`, so reconstructions are matched first, in rounds),
and by its gloss. A candidate whose ancestry names other words at an ACD ancestor's level,
of another key and gloss, is taken only when its gloss confirms it. Equal fits stay `candidates`; of several ACD
records fitting one Wiktionary record (homographs, doublets), the best is matched (by
shared reflexes, then an equal key over a near one). A shared ancestor vouches for a
gloss that shares nothing with the record's only when it fits the ancestor's gloss, and
a fitting gloss stands against the shared ancestor of a candidate Wiktionary only cites
(often its entry under another spelling). v4 decides as v3 does, except that reviewers'
judgements decide the pairs they cover; its score is the probability learnt weights
(`matching.Linkage`) give the match. Letting the weights decide did worse than v3's
ranking (`checks/acd-3-v4-trial/`). v1, which weighed no ancestors, is reviewed in
`checks/acd-3/`, v2 in `checks/acd-3-ancestry/`, v3 in `checks/acd-3-v3/`. The relay rule, `agent:wety-relay-acd@1`, compares by ancestry, since the
ACD links a reflex straight to its set's reconstruction. The docstrings of `Matcher` and
`Relayer` give the rules in full.

## Matching LiLa to Wiktionary

`match_lila.py` (lila-3) matches LiLa's de Vaan records to Wiktionary's records, which
stand for wety's items, and relays Wiktionary's credits to de Vaan (2008) to his
assertions in LiLa. It reads a graph's assertion export (`graph-assertions`) and the
lila-2 assertions; both must lie in the checkout, so hard-link them in a worktree:

```sh
C=$S/lila-edl/checks/lila-3-v4
uv run --frozen python -m etymology.match_lila --export data/runs/<run>/checks/core-4-references/assertions.jsonl.gz --lila $S/lila-edl/checks/lila-2/assertions.jsonl --out $C --compare $S/lila-edl/checks/lila-3-linkage
uv run --frozen python -m etymology.match_lila --out $C --review
uv run --frozen python -m etymology.notation $C/pairs.json --out $S/lila-edl/etc/substitutions-wiktionary.tsv --note "de Vaan's keys (LiLa) against Wiktionary's, from judged pairs"
```

The first writes `assertions.jsonl` (the matches, the relays and what they refer to),
`relays.json` (every crediting assertion and what became of it), `result.json` (counts,
the learnt weights, the score against the judgements, what the judgements decided, and
the held-out score), `sample.json` (matches to review where no judgement decides them,
stratified by kind, key and lineage, and records left unmatched, for recall), `hard.json`
(ties and targets of low probability) and `pairs.json` (the judged etymon pairs, for
`notation`); with
`--compare`, also `changes.json`, every record whose match differs from an earlier
check's. The second, once a reviewer has judged the sample (`review.json`), the changes
or the hard cases, adds their precision to `result.json` and their judgements to
`etc/judgements.jsonl` (an `answer` names the right record where the rule found none).
The third mines the substitutions table the next run's distance reads. The rule,
`agent:wety-match-lila@4`, is in the module's docstring. Earlier rules' reviewed outputs
stay in `lila-3/` (v1), `lila-3-ancestry/` (v2) and `lila-3-linkage/` (v3), and v4's
trials in `lila-3-v4-trial/` and `lila-3-v4-trial2/`.

## What the matchers share

`matching.py` holds what every matcher needs, so that a new source's matcher is an
adapter (its records, their lineage, its forms) and not another thousand lines:

- **Judgements.** A source's `etc/judgements.jsonl` keeps every reviewed judgement on
  a pair (record, Wiktionary record: the same word or not; or the word is among these
  candidates), apart from the rule that proposed it. Every run is scored against all of
  them: target precision, recall of judged matches, and each regression. Judgements
  also decide the pairs they cover (`choose`), so each check is scored on judgements
  held out of the fit too (`split`, `pooled`). `harvest` reads review files into it; a
  review only judges what the judgements do not decide.
- **Weighing.** `Linkage` weighs categorical evidence as Fellegi and Sunter's record
  linkage does: per field and level, log2(m/u), learnt by EM from every candidate pair
  with judged pairs held at their label and the weights bound to their levels' order.
  A score is a probability. Fitted alone, EM finds another split than "one word or
  not" (lineage-connected pairs in LiLa, glosses in the ACD), so judged pairs weigh
  heavily, and conditions that are not additive (a weak key needs a shared reflex and
  a common stem class) stay conditions.
- **Decisions.** `decide` keeps the best candidate when it reaches the threshold and
  beats the next by a margin, else every tied one; `assign` keeps a source's
  homographs from sharing a target.
- **Triangulation.** `triangulate` checks three sources' matches against each other with
  no review: where a record of each reaches Wiktionary and the two are linked directly,
  both routes should arrive at one record, and the share that do not bounds the
  matchers' summed error from below. No third source with lexical data overlaps LiLa or
  the ACD yet (IE-CoR is in the store only as languages).
- **Notation.** `notation.py` aligns the keys of judged pairs, counts what differs,
  and writes a substitutions table per source pair (`etc/substitutions-<other>.tsv`);
  `Distance` reads it, so a difference seen often (a laryngeal one source writes and
  the other drops) costs little. A curator moves a difference into the source's
  orthography profile once it is plainly notation. A profile may `include` a convention
  in `conventions/` (`ine`: the Indo-European notation de Vaan and Wiktionary share).

## Claims in the graph build

Each source enters the graph as a claims product: `wety-data run acd_claims` and
`lila_claims` convert the source's release and match it against the graph's assertion
export (`wety-data run etymology_assertions`), and the graph build reads both, with
`sources/wiktionary/etc/overrides.json` (reviewers' overrides of words' readings). The
processor joins a claim to the words its records are matched to and reads each word it
touches by synthesis tiers (one lineage, consensus, preference, else unsettled);
`crates/processor/src/claims.rs` gives the rules. After a build, its generation report:

```sh
target/release/graph-edges <graph> edges.tsv --readings readings.jsonl
LC_ALL=C sort edges.tsv > new.tsv   # and the previous generation's dump, sorted, as old.tsv
LC_ALL=C comm -3 old.tsv new.tsv > diff.tsv
uv run --frozen python -m etymology.generation_report readings.jsonl diff.tsv --matches <each claims product's matches/assertions.jsonl> --out <dir>
```

## Tests

```sh
uv run --frozen python -m unittest discover -s packages/etymology/tests
```
