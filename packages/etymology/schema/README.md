# Etymology assertions, v0

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
| `cognate_set` | Forms said to share an origin: a reconstruction, or null when none is named, and members | `<source name>:<source key>` |
| `alternative_set` | Hypotheses or cognate sets one asserter offers for one word; `exclusive` false means all are right (doublets) | digest of members, exclusivity, preference |
| `assertion` | An asserter's stance on a hypothesis, cognate set or alternative set: who holds it, hedges, credits, locator, the source's words | digest of source, locator, target, stance |

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

A credit names a `work`, a `role` (`follows`, `reports`, `against`, `data`) and a
`scope` (`inline`, `note`, `entry`), and keeps its locator.

Records stay in their source's terms: matching them to wety's items comes later, so two
sources' hypotheses share an id only once their records are matched.

## Extensions

A field only one source needs goes under `x.<source name>` on the object it describes,
until a second source uses it the same way. In v0:

| Source | Object | Field | What it holds |
| --- | --- | --- | --- |
| acd | cognate_set | `etymon` | The etymon grouping the set's levels (etyma.csv) |
| acd | cognate_set | `metathesis`, `assimilation` | Members whose irregularity the ACD explains so, by index |
| acd | cognate_set | `gloss` | A cf.csv group's gloss |
| lila-edl | record | `lila_lemma` | The entry's lemma in LiLa's lemma bank |
| lila-edl | record | `forms`, `cognates` | Other written forms with their features; LiLa's cognate links between etymons |
| wiktionary | assertion | `holder_stance` | `rejects` when the editors report that a scholar rejects the hypothesis ("denied by De Vaan") |
| wiktionary | assertion | `sense` | The sense a link concerns, by link index |
| wiktionary | record | `first_attested` | A first attestation (`{{etydate}}`, prose) |

`holder_stance`, `sense` and `first_attested` carry concepts the design names (a reported
rejection; form or sense; date); they move into the core when a second source gives them.

## What the samples exercise

core-2 converted 30 Wiktionary entries, 24 ACD groups (with their doublet and disjunct
partners) and 20 LiLa etymologies; all validate. Parts of the core only one sample uses:

| Part | Only in |
| --- | --- |
| stance `reports`; credit roles `follows` and `reports`; scopes `note` and `entry`; hedges on links; `probable` | Wiktionary |
| link relations `ultimate`, `influence`, `unknown`; `head`; records with no form; agents of kind `unnamed`; `quote`; `work.authors`, `work.source` | Wiktionary |
| credit role `against`; cognate sets with a named reconstruction; alternative sets of cognate sets | ACD |
| record `certainty` | LiLa (the ACD's Doubt flags are never set in v2.0) |

No sample sets `preference`, `replaces`, or an unplaced hedge.

Not in v0, by design: matches of records to items, relays, readings, overrides, gradings,
and links compiled from a source's tree (acd-2). Not representable yet: an origin that
names no word in any language ("from a call used for dogs", Old Spanish *perro*).
