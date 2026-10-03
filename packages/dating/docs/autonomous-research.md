# Bounded autonomous research queue

`harness.autonomous_research` connects retained literature discovery, model
screening, automatic source selection, acquisition, source windows, compact
extraction and cross-backend review. It produces an editorial research packet;
it cannot import claims or assign historical defaults.

## Execution contract

The brief freezes the family, queries, selection backend, configuration files,
source/packet/follow-up limits, review routing and one cumulative model budget.
Preparation copies configurations and implementation. Changing these inputs
requires a new declared experiment; an existing budget/deadline is not reset.

Discovery currently uses Europe PMC core metadata. Query results are interleaved
round-robin and deduplicated by DOI or provider ID before a twelve-candidate cap.
The complete retained responses and candidates omitted by that cap remain visible.
Europe PMC is a biased, bounded bibliography, not a comprehensive linguistics
index. Query order and result order are not evidence-quality rankings.

All configured screening backends see the same ordered candidates. One selector
is declared in advance. Only its complete, aligned include/exclude response
selects the first two included candidates. A failed/misaligned selector produces
an explicit unresolved result; another model or the coordinator cannot silently
substitute a choice. Other screens are comparisons, not votes or independent
historical confirmations.

Each selected source has one initial discovered route. At most two additional
routes or linked-material requests are permitted across the run. Article HTML is
parsed only when an adequate article element exists. Unsupported PDFs/XML,
landing pages, challenges and failed requests remain access/parser gaps.
Acquired linked files are explicitly **unprocessed**, not supplied evidence or
completed supplement review. Images remain omitted pixels. The current parser
and follow-up ordering are deliberately limited; the report must retain those gaps.

Source windows are interleaved across parsed articles before the four-packet cap.
The document inventories retain omitted windows and context. Extraction uses only
declared operational backends; exclusions require a reason in the brief. Recovered
valid proposals can receive one source-context review; invalid items, missing
batches and overflow remain incomplete coverage. Reviews never approve imports.

New briefs may enable [one bounded native-time repair pass](native-repair.md) with
`limits.native_repair_cells` (default zero, maximum twelve). Original review,
repair and review of repaired proposals all share the original run ledger. The
repair changes only proposed native-time fields; semantic errors in other fields
remain for review. The implementation now supports explicit prior parameters,
elapsed durations and quantified offsets without automatic calendar projection.
Those post-trial shapes and revised review instructions are offline-tested; their
live improvements remain unmeasured. The original Austronesian run below is unchanged.

HTTP actions are durably reserved before dispatch. Interrupted or failed actions
are not silently repeated on resume. Acquisitions have a sixty-second worker limit;
model workers have a 150-second limit, all bounded by the shared run deadline.
Model phases share one call/cash ledger and consecutive-failure stop. A `STOP` file
prevents subsequent dispatch. In-flight calls are bounded by their worker deadlines.

## Commands

Run from the root environment:

```sh
.venv/bin/python -m harness.autonomous_research prepare NEW_DIRECTORY --brief BRIEF.json
.venv/bin/python -m harness.autonomous_research run NEW_DIRECTORY
.venv/bin/python -m harness.autonomous_research report NEW_DIRECTORY
```

Use the frozen brief at
`data/workspaces/dating/_research/autonomous-austronesian-brief-2026-09-20.json`
as an example. A fresh run needs a fresh directory and explicitly chosen deadline;
copying an expired brief does not renew its budget. The source graph, acquisition
receipts, source texts, windows, execution snapshots and model judgments make the
selection and processing decisions inspectable.

## Corrected controls and provider compatibility

The prerequisite control run is
`data/workspaces/dating/_research/compact-controls-2026-09-20/`. Five invented
statements test positive BCE quantities, an author-defined epoch, coarse centuries,
normal-prior parameters and purely relative chronology. Identical fixed proposals
also test reviewers. They are software controls, not historical evidence.

Six calls were made; an OpenRouter review cell was skipped after two extraction
failures. Claude passed the declared extraction checks. Codex still left explicit
1200 BCE unknown, although its other four assertions were structured correctly.
Both reviewers recognized negative BCE and missing posterior quantities and accepted
qualified enclosing centuries. A faulty control proposal called generic divergence
a sampled crown: Codex correctly requested revision, while Claude approved it.
The original expected label was wrong; `coordinator-audit.json` preserves the correction.
After this disclosed adjudication, reviewer disposition agreement is 4/5 for Claude
and 5/5 for Codex. This tiny exposed set does not select a production winner.

OpenRouter's canonical compact schema failed at Vertex with HTTP 400. The same
model/provider accepted the explicit `schema_profile: "minimal"` variant, which
omits schema annotations and selected validation constraints from constrained
decoding. Local canonical validation is unchanged, and request snapshots preserve
both schemas. The response still used invalid temporal forms `mean` and `prior`;
three of five assertions were recovered, and the envelope remained failed.
Therefore this is an API-compatibility observation, not a semantic repair or a
reason to promote the configuration. Which removed schema keyword caused the
rejection remains unresolved. See [Google's supported-schema guidance](https://docs.cloud.google.com/gemini-enterprise-agent-platform/models/capabilities/control-generated-output).

The application continues to default to canonical schemas. The minimal profile is
an opt-in OpenRouter role setting and cannot weaken canonical shape, provenance or
scientific import checks. Model/provider changes remain explicit experiments.

## Austronesian development run

The first brief froze three contact/chronology/correction queries, twelve candidates,
two source targets, two follow-ups, four source windows, twenty model calls, $2 in
cash reservations and a thirty-minute deadline. All three backends were screened;
Claude was the predeclared selector. Claude and Codex handled extraction and reciprocal
review. OpenRouter compact extraction was explicitly disabled for this run after
the control failures, rather than silently changing its model/provider.

The query responses contained 28 distinct records. Round-robin selection supplied
twelve for model screening and recorded sixteen cap omissions. The selecting model
chose these two author-written articles:

- [Valério, Tamburini & Corazza 2026, East Polynesian lunar calendars](https://doi.org/10.1371/journal.pone.0353287).
- [King et al. 2024, Philippine language phylogeny](https://doi.org/10.1038/s41598-024-65810-x).

Both main article bodies were acquired and parsed automatically. The first concerns
calendar traditions and their proposed relationship to contact and linguistic
history; its fitted calendar relationships are not language divergence events.
The second infers language-switching intervals using branches on a fitted tree;
those intervals are conditional model inferences, not direct observations of shifts.
Two linked responses were retained but not processed. Five source windows were
omitted by the declared cap. No coordinator replaced a selected source or picked
an alternative URL after seeing screening decisions.

OpenRouter screening returned an upstream 429; its missing comparison remains in
the report. The completed source-selection decision came from the originally declared
selector. This establishes that the bounded queue can make and execute research
choices; it does not establish comprehensive discovery, screening precision, complete
source coverage or historical accuracy.

The autonomous run made **19 model calls**: three screens, eight extractions and
eight reviews. Six extraction envelopes completed and two failed; item-level recovery
retained **53 schema-valid proposals and twelve invalid items**. Seven review batches
completed, covering **41 proposals: sixteen supported and twenty-five revise**.
One inconsistent review envelope leaves twelve targets without aligned judgments.
Mechanical provenance checks reduced supported outcomes to ten eligibility
diagnostics; none were imported. The four scheduled windows cover 58,635 of 108,694
parsed characters. These are processing metrics, not precision or exhaustive recall.

A narrow utility result is the source-specific Tongareva–Rarotonga contact interval
1864–1929, retained as a secondary report and distinguished from the hypothesis of
calendar-list borrowing. Competing, undated histories of night-name forms also remain
separate research leads. Original cited sources, omitted sections/figures and detailed
phonological adjudication remain gaps. The report makes no historical-default or
mass-release claim.

The strongest new extraction failure was twelve proposals declaring both a conventional
BP1950 interpretation and an explicit present1950. Validation caught all twelve;
recovery preserved the unaffected items. Codex often retained numeric branch bounds
only in prose; review correctly requested structured native values without requiring
an explicit epoch. Review also caught calendar/language substitutions and overprecise
calendar endpoints. The final compact schema now includes explicit epoch-field
descriptions and a mutual-exclusion instruction. Its gain on new live sources is
**unmeasured**; the failed outputs remain unchanged.

Post-run engineering hardening adds candidate-ID alignment, derived-stage and source
byte integrity checks on resume, and partial completion reports for phase failures.
These changes are regression-tested. The live run used the earlier ordered screening
contract; its input/output order is retained, and the ID-binding improvement is not
retroactively claimed as live-tested. Re-running the completed original queue before
those changes left its state and budget unchanged and made no additional requests.
Historical reporting remains available after code upgrades; dispatch/resume requires
the frozen implementation.

Across controls plus the autonomous trial, **25 logical calls** were made. There are
no confirmed cash figures for the three failed OpenRouter calls; **$1.50 remains
conservatively reserved**. Subscription usage and SDK estimates are recorded separately.
Both experiments stayed within their independent, predeclared budgets and deadlines.

Validation: **215 dating tests**, the managed workflow and all three publication
reproduction checks pass. All **3,575 claims and the complete 8,625-code view remain
exactly unchanged**, with every `spoken` null. `validation.json` records the workflow
ID and scientific-content digests. No catalog inputs or model production defaults
were changed.

Immutable editorial archive:
[`1c4b4526fe34a8e79ff11465b64b5c5c0e7049989a70d77468b7b6fb4eb44bb4`](../../../data/artifacts/dating-autonomous-development-2026-09-20/1c4b4526fe34a8e79ff11465b64b5c5c0e7049989a70d77468b7b6fb4eb44bb4/manifest.json).
It preserves both experiments, frozen phase code and model snapshots, source-manifest
references, post-run implementation, audits and validation receipts. The final managed
workflow is `6b7f88a0e8c244bf95182c7ac63fb2fd`.
