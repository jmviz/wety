# Etymology assertions, v4

Every etymological source enters wety as assertions in one schema,
[`assertion.schema.json`](assertion.schema.json) (JSON Schema 2020-12). A source's
assertions file is JSON Lines: one object per line, told apart by `type`. The design
behind it is the shared doc "wety etymology claims model", outside git; this page is what
code relies on.

Check a file:

```sh
uv run --frozen python -m etymology.assertions --summary <file.jsonl>
```

The schema checks each line. `etymology.assertions` also checks what spans lines: ids are
unique, every reference resolves to an object of the right type in the same file, digest
ids equal their digest, a hypothesis's links chain from its word, and hedges point at
links or members that exist.

## Objects

| `type` | What it is | Id |
| --- | --- | --- |
| `source` | A document or dataset at a version, with its licence and what may be displayed or exported | `<name>@<release>` |
| `agent` | An asserter or a holder: a `person`, `editors`, `unnamed` (holders the source does not name), a wety `rule` or `model` | `agent:<slug>` |
| `work` | A work an assertion credits; `source` names it when wety reads it too | `work:<source>:<key>` |
| `extraction` | One run of one tool (`parser`, `converter`, `model`) over its inputs, at a git revision | digest |
| `record` | A word as one source writes it: language (the source's label, its Wiktionary code and the relation from the language map), form, gloss, homograph label, locator; `form` null when only a language is named | `<source name>:<source key>` |
| `hypothesis` | One word's origin: links whose first child is the word, each later link's child a parent of an earlier one | digest of word and links |
| `cognate_set` | Forms said to share an origin: a reconstruction, or null when none is named, and members (none only when the source lists no forms for its reconstruction) | `<source name>:<source key>` |
| `alternative_set` | Hypotheses or cognate sets one asserter offers for one word; `exclusive` false means all are right (doublets) | digest of members, exclusivity, preference |
| `assertion` | An asserter's stance on a hypothesis, cognate set or alternative set: who holds it, hedges, credits, locator, the source's words | digest of source, locator, target, stance |
| `match` | A record matched to another source's record for the same word (in wety, a Wiktionary record, standing for an item), held by a matcher rule or a reviewer, with a score and what it weighed; or, with `candidates` instead of a `target`, left unmatched between equal fits | digest of record, target, candidates, holder |
| `relay` | An assertion that credits a work wety reads directly, linked to that work's own assertion of the same hypothesis; whether they agree, and if not how they differ | digest of the two assertions |

A link has a `child`, `parents` and a `relation`: `derivation` (with a `mode`, wety_core's
EtyMode names, and optionally the `head` parent), `ultimate` (descends from the parent by
unstated steps, as `{{root}}` says), `influence` (shaped by, not descended from) or
`unknown` (no parents: known to be unknown).

An assertion's `stance` is `endorses`, `reports` (another holder's view, no side taken) or
`rejects`. Its `hedges` each carry a level (`probable`, `possible`, `doubtful`; no hedge
is `stated`) and the source's words, on a `link` or a cognate set `member` by index, or on
neither when the extractor could not place it; such a hedge lowers nothing. Doubt about a
reconstruction itself is the record's `certainty`. A member of an alternative set is
asserted on its own too, and names the set in `in_sets`, so it is never counted twice.

An assertion is stated unless it carries `compiled`: a wety rule derived its target from
a source's structure, such as a cognate set placed on the source's tree. `compiled.from`
locates that structure and `compiled.words` is what readers see ("implied by the ACD's
tree"). Only an agent of kind `rule` compiles, and only hypotheses; it holds what it
compiles and credits the source with role `follows`. Two sources that state and compile
the same link assert the same hypothesis.

A credit names a `work`, a `role` (`follows`, `reports`, `against`, `data`) and a
`scope` (`inline`, `note`, `entry`), and keeps its locator.

Records stay in their source's terms. A `match` links a record to a Wiktionary record,
which stands for wety's item; only a language mapped `exact` may match. Its `basis` says
what it weighed: the forms' comparison keys (`equal`, `near`, `different`), the gloss
similarity, of the reflexes the record's source cites how many are the target's
descendants in wety (`reflexes`), and of the record's ancestors in its source that are
matched how many are the target's ancestors in wety (`ancestors`): a Latin entry whose
matched etymon is among one homograph's ancestors, a reflex whose set's reconstruction
is. A record that fits two candidates equally names both and has no
target, rather than being guessed into one. Two sources' hypotheses about the same word
share an id only once their records are rewritten through matches.

A `relay` links an assertion that credits a work (Wiktionary following de Vaan) to the
work's own assertion, which wety reads directly; they stay two assertions. `agrees`
false lists the `differences` (`parents`, `form`, `relation`, `mode`, `hedge`,
`edition`, `locator`): Wiktionary misquotes the work or follows another version of it.
`form` says that the two name the same parent, matched by lineage, but reconstruct it
otherwise: a match whose `key` is `different` aligns two records that stand in the same
place of a word's lineage, which is how two scholars' reconstructions of one word can
be compared at all.

## Extensions

A field that only one source needs goes under `x.<source name>` on the object it
describes. Every extension is declared in [`extensions.json`](extensions.json): its
source, object and field, the concept it carries (a short id) and its kind. The validator
refuses an undeclared one. Extensions are a waiting room, not a home:

- **design** extensions carry a concept the claims model already names (a holder's own
  stance, a link's sense, a date). One enters the core as soon as a second source gives
  it, in that source's converter milestone.
- **source-native** extensions carry a category or id whose meaning is the source's own.
  One enters the core only when another source uses it with the same documented meaning.
- Before declaring an extension, look in the registry for its concept under another
  source. If it is there, do not declare it: move the concept into the core in the same
  change, with every converter that used it, and raise the schema's version.
- A test fails while two sources declare the same concept, so a shared concept cannot
  stay split across extensions.
- Each converter milestone's check reports the extensions it added and promoted.

## What the converters exercise

core-2 converted 30 Wiktionary entries; acd-2 and lila-2 convert the whole ACD v2.0 and
LiLa releases; core-5 reads every References section of Wiktionary's pages dump into a
bibliography of 19,699 works and 990,292 entry-scope credits, which wait in a table
beside it until the processor emits the assertions they belong on. All validate. Parts of the core only one source uses:

| Part | Only in |
| --- | --- |
| stance `reports`; credit role `reports`; scope `note`; `probable` | Wiktionary |
| link relations `influence`, `unknown`; `head`; agents of kind `unnamed`; `quote`; `work.authors` | Wiktionary |
| credit role `against`; cognate sets with a named reconstruction or no members; alternative sets of cognate sets; `compiled`; agents of kind `rule` | ACD |
| record `certainty` | LiLa (the ACD's Doubt flags are never set in v2.0) |

No converter sets `preference`, `replaces`, or an unplaced hedge.

Not in v4, by design: readings, overrides and gradings. Not representable yet: an origin that names no word in any language ("from a
call used for dogs", Old Spanish *perro*); and the ACD's doublets and disjuncts between
forms: alternative sets hold cognate sets, so a variant that is a member of a set stands
for its whole set, two groups can collapse into one, and a group within one set (a
reconstruction and its own member) has no alternative set.

## Versions

- **4**: a relay's `differences` include `form`.
- **3**: a match's `basis` records `ancestors`.
- **2** (acd-3, lila-3): `match` and `relay`.
- **1** (acd-2): `compiled` on assertions; a cognate set may have no members when it
  names a reconstruction.
- **0** (core-2): the first schema.
