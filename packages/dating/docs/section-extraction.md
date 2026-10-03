# Bounded sections and incomplete-response recovery

The optional section workflow produces **reviewable proposals**, retaining every
response and failed assertion. It never imports claims or sets historical defaults.
The ordinary `extract` command now also records a recovery sidecar when a retained
failed response contains usable assertions; it does not overwrite a previous
successful extraction with an incomplete replacement.

## Prepare and execute

Run from the repository root. A manifest selects exact source text hashes, parser
metadata, family search scope, backend configurations, a frozen reference, source
window/context limits and one shared call/cash budget. An example is retained in
`data/workspaces/dating/_research/section-evaluation-2026-09-20/manifest.json`.
Its separate metadata-correction receipt supplies the correct Ehret title; the
original evaluation plan remains frozen. For new manifests use the corrected title.

```sh
.venv/bin/python -m harness.section_extraction prepare MANIFEST.json NEW_RUN_DIRECTORY
.venv/bin/python -m harness.live_evaluation run NEW_RUN_DIRECTORY --phase sections
.venv/bin/python -m harness.section_extraction report NEW_RUN_DIRECTORY
.venv/bin/python -m harness.section_extraction repairs NEW_RUN_DIRECTORY NEW_REPAIR_DIRECTORY --max-cells 3 --max-claims 4
.venv/bin/python -m harness.live_evaluation run NEW_REPAIR_DIRECTORY --phase repairs
.venv/bin/python -m harness.section_extraction report NEW_REPAIR_DIRECTORY
```

The text paths in a manifest are relative to the repository root. Configurations
are copied into the run; tasks embed immutable excerpts. The runner hashes the
plan, configs, reference and budget policy and snapshots the implementation. Never
edit an executed plan to make a failed result pass. A new version must explicitly
share the existing ledger when it belongs to the same experiment.

## What a packet guarantees

Primary source spans partition every supplied character without gaps. Paragraph
boundaries are preferred; splitting a long line is flagged. Heading ancestry,
caller-selected definition spans, heuristic definition lines and a short preceding
overlap provide context under a separate character budget. Every excerpt retains
its source ID, full-text hash and exact offsets. Omitted context and known linked
files/pixels remain visible. Headings recovered from article HTML are aligned back
to the unchanged parsed text; unaligned headings are explicitly reported.

These are bounded source windows, not an assurance that every argument fits inside
one window. Definition selection is heuristic. Long tables, cross-references,
figures, footnotes and remote supplements can need additional context. A model
must report such gaps instead of guessing. Bibliography-title dates are discovery
leads, not assertions established by the citing article. Source acquisition
completeness remains separate from complete processing of supplied text.

## What recovery guarantees

Recovery parses only complete JSON, rejecting duplicate keys and nonfinite numbers.
It validates assertions independently with strict scalar types, preserves every
raw assertion, flags unknown fields, and records invalid gap/conflict objects or
envelope metadata. It does not invent units, reverse bounds, silently coerce numbers
or change an event label. Historical failed outputs remain failed even when some
proposals are usable under the current schema.

Exact quotes must occur within a single supplied segment: joining disjoint excerpts
cannot create a valid quote. Matching retains source offsets and distinguishes
primary evidence from repeated context. Native quantity/unit checks and calendar
projection remain separate from schema validity. Semantic flags identify selected
known traps, but absence of flags is not semantic approval. Every recovered proposal
still requires source-scope/event/role/scenario review.

Reconciliation retains all occurrences and only groups structurally identical
proposals with the same source spans. Distinct analyses or source interpretations
never merge because their dates or catalog labels match. Semantic duplicates and
conflicts remain a review task. Missing, stopped, failed and unscheduled sections
prevent an execution-complete report; zero-claim successful sections remain in the
denominator. Execution completeness is not assertion recall or acquisition coverage.

## Bounded repair

The repair planner selects only schema-invalid assertions, at most the specified
number of assertions/cells. Valid originals stay untouched. Each original cell
can receive one repair plan, recorded under a lock; repair tasks cannot recursively
generate repairs. Both plans use the same budget ledger. A repair receives the
original source packet and the invalid raw assertions/errors. Each outcome names
its original assertion index and digest, with either a proposed corrected claim or
an explicit unresolved outcome and rationale. Unknown, duplicate and missing target
IDs fail alignment. Insufficient evidence never requires a fabricated correction.
Changed identities or interpretations remain explicit proposed revisions; the
program never substitutes them into the original response or declares the original
document successful. Envelope errors, omissions and semantic errors are not silently
cleared by a syntactically successful repair.

The budget is a conservative dispatch reservation, not a provider billing guarantee.
Schema repair inside the OpenRouter transport can consume an additional provider
request within one logical call and is retained in its trace. Failed paid calls
keep uncertain reservations; subscription cost estimates are not cash expenditure.

## Validation scope

Offline tests cover complete source partitioning and budgets, exact offsets,
context-only quotes, disjoint-excerpt rejection, strict partial recovery, repeated
repair prevention, missing sections and scenario-preserving reconciliation.
Replaying the earlier failures preserves 15 structurally valid assertions from
each 18-assertion Claude/Codex response without editing their three invalid units.
This is recovery of proposals, not retrospective model success or linguistic gold.

The exposed Bantu trial uses the original article and critique as one dependent
cluster, the existing 26-sentinel reference, and all three required backends. Its
new runs measure the combined current schema/prompt/section workflow; they cannot
isolate a causal gain from chunking alone. All source/field errors, missing context,
access gaps and unknown billing remain part of the result. A family specialist is
optional; documented model review/self-audit must still state its limits.

## Development results, 20 September 2026

The [immutable editorial artifact](../../../data/artifacts/dating-section-evaluation-2026-09-20/617163b2c6468e557431d721320c1c73d555b7be845a3648dffe4478f3dd41f0/manifest.json)
retains plans, source packets, requests/responses, failures, implementation
snapshots, per-assertion audits, budgets and validation receipts.

The six packets cover 76,252 supplied characters: five main-article windows and one
critique. Baseline calls use Gemini 3.8 Flash/medium through OpenRouter Vertex,
Sonnet 5/medium through Claude Agent SDK, and Astra/medium through Codex. Each has
one in-flight request per backend, a 300-second worker limit (Codex completion
limit 240 seconds), and explicitly frozen settings. These are tested profiles,
not recommendations about every model accessible through those transports.

| Baseline | Completed / six | Structurally valid / returned assertions | Selected sentinel presence / 26 |
| --- | ---: | ---: | ---: |
| OpenRouter | 4 | 10 / 12 | 7 |
| Claude Agent SDK | 5 | 29 / 29 | 17 |
| Codex | 4 | 19 / 19 | 13 |

Presence matching includes duplicate reconciliation and source/subject/event
judgment; it does **not** require all fields to be correct. Failed sections remain
in the denominator. The coordinator inspected every returned baseline assertion
for core semantics and selected contradictions. All-field supported precision
remains unmeasured, and no profile passes the predeclared acceptance gate. Codex
retained all eight selected critique assertions; that useful narrow result does
not offset its two main-article timeouts. Claude missed the calibration section
through a worker timeout; OpenRouter omitted substantive archaeological assertions
from a successfully processed calibration section and failed the critique with an
upstream rate limit. Two identical opening/calibration packets sent to a stronger
OpenRouter Astra/high/Azure profile both timed out. There is no universal winner.

One bounded Claude repair of two invalid OpenRouter assertions correctly supplied
missing interval labels and explicit target identities. Both repaired proposals
still contain model-written calendar values while their native age scale remains
unknown. Programmatic normalization withholds these conversions. Recovery now
reports conflicting legacy calendar fields explicitly; a post-run audit retains
these findings separately from the original results. Minor rounding differences
are also exposed as disagreements, not automatically severe historical errors.

The semantic audit found three environmental assertions incorrectly typed as
Bantu population events, even though their notes denied a population date. The
production prompt contributed by requiring archaeological dates to be population
events. It now distinguishes population movements from artifact layers, vegetation
and climate, permits unresolved subjects/events with a gap, and requires agreement
between fields and notes. This correction occurred **after** the evaluated calls;
its model-quality benefit is not measured. Other observed issues include unsupported
software/dispersion descriptions, uniform priors encoded as event ranges, and a
quote altered by punctuation. Exact numbers and fluent notes are insufficient.

There were **21 logical calls** including the two stronger-profile calls and one
repair. Reported OpenRouter cost totals $0.290229; settled reported cash is
$0.17607825. Four uncertain paid calls keep conservative reservations, bringing
accounted cash reservations to $4.17607825 against the $5 limit. Reported Claude SDK
nominal cost is $1.2061828, not subscription cash expenditure. Timeout charges and
Codex monetary costs are unknown. No further paid retries were launched.

**Validation:** 199 dating tests, the managed rebuild and all three publication
checks pass. All 3,575 claims and the complete 8,625-code view are unchanged; every
`spoken` remains null. The experiment is editorial evidence, not scientific import.
