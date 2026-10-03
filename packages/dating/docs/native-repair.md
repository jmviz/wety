# Bounded native-time repair

`harness.compact_repair` proposes a single patch to `native_time` while retaining
the original subject, event, role, method, expression, quote, scenario and
qualifications. It cannot turn a repaired number into an approved assertion.
Original responses, including failed envelopes, remain immutable. Each target
binds the original plan, cell, result, item index and raw assertion digest.

The automatic planner selects assertions with invalid native fields, unsupported
quoted numbers or numeric expressions left unknown. Other fields must already
validate, and the original quotation must bind a primary source span. It schedules
at most six targets per cell and one repair pass per original cell. Omitted targets
and prior reservations remain explicit; a repair cannot generate another repair.
Unresolved outcomes preserve the original assertion rather than erasing its time.
Unknown or duplicate IDs, missing targets and invalid envelopes prevent an aligned
repair result from reaching the review planner. Item-level recovery still preserves
the failed response for diagnosis.

The existing autonomous queue can enable this stage with the brief limit
`native_repair_cells` (zero by default, maximum twelve). Original review, repair and
review of repairs all use the same cumulative call/cash/deadline ledger. The brief
must reserve enough of that budget for the requested stages. There is no automatic
model substitution or budget extension when a failure circuit opens.

For an existing **unexpired** extraction run with remaining budget:

```sh
.venv/bin/python -m harness.compact_repair EXTRACT_DIRECTORY NEW_REPAIR_DIRECTORY --max-cells 3 --max-claims 6
.venv/bin/python -m harness.live_evaluation run NEW_REPAIR_DIRECTORY --phase repairs
.venv/bin/python -m harness.compact_review NEW_REPAIR_DIRECTORY NEW_REVIEW_DIRECTORY --routing ROUTING.json
.venv/bin/python -m harness.live_evaluation run NEW_REVIEW_DIRECTORY --phase reviews
```

These commands neither renew an expired trial nor alter its failed-call accounting.
An exposed diagnostic comparison of retained failures is a separately declared
experiment, preserving the original trials and their budgets.

## Source context

New packets prioritize declared context, ancestor headings, then exact sentences
containing likely epoch, prior and branch-interval definitions. Sentence selection
can retain “taking 2000 as the date for the present” without consuming a whole long
paragraph. Selected and omitted spans retain offsets and source hashes. Primary
spans still partition the supplied text exactly. These heuristics cannot establish
complete methods coverage, and a study's epoch never automatically applies to a
secondary claim cited by that study.

Literal-number checking now recognizes comma, ordinary-space, thin-space,
narrow-no-break-space and no-break-space thousands separators. It never joins
numbers across table-row newlines. This preserves publisher typography without
rewriting source text or historical claim payloads.

Post-run quote checking also accepts **whitespace-only equivalence**, retaining the
actual contiguous source excerpt, exact text offsets, source hash and match method.
The model's quote is unchanged; `exact_quote` stays false for such a match. Words,
punctuation, digits and separated source segments are never silently rewritten or
joined. This repairs false failures caused by publisher nonbreaking spaces while
retaining auditable source text.

## Development comparison

The frozen experiment is
`data/workspaces/dating/_research/native-repair-2026-09-20/`. It presents the same
fifteen retained assertions to each backend: Philippine introduction and inferred
transition branches, plus Dravidian methods/calibrations. These are two exposed
study clusters, not held-out references. Five purely relative/schema-challenging
cases are included alongside ten representable quantities. One selected BP case
also has an inexact original quotation; the controlled comparison retains that
defect. The live implementation's automatic planner would have excluded it; the
post-run whitespace alignment now binds the actual source span separately.

The comparison freezes three extraction batches per backend, up to nine cross-backend
review batches, eighteen total calls, a $3 cash reservation cap and a 35-minute
deadline. OpenRouter uses the previously accepted minimal wire schema and the same
Gemini/Vertex profile; local canonical validation remains strict. Claude and Codex
retain their previous declared model profiles. The new repair prompt and context
selection are tested together, so their separate contributions are not identified.

The coordinator-authored rubric evaluates source fidelity, not historical truth.
Restoring a quantity is distinct from resolving all fields: the native-only patch
deliberately cannot remove contradictory qualifications or repair an erroneous
contact/shift event label. Review results remain editorial proposals.

### Results

| Backend | Repair batches | Proposed patches | Unresolved outcomes | Mechanically ready patches in live run |
| --- | --- | --- | --- | --- |
| OpenRouter | 2 complete, 1 failed | 4 | 5 | 3 |
| Claude Agent SDK | 3 complete | 12 | 3 | 8 |
| Codex | 2 failed, 1 circuit-skipped | 0 | 0 | 0 |

Codex returned a single incomplete placeholder/unknown patch in each attempted
batch. The validator rejected `proposed` with unknown time; the two-failure circuit
stopped its third repair and all three planned review batches. OpenRouter repaired
the introduction quantities, declined the conditional branch brackets, then returned
truncated JSON on Dravidian methods. Claude restored all ten selected representable
quantities and proposed ordinary range/distribution fields for the two prior-shape
challenges. Restored quantities are **not** an all-field correctness score: event
mislabels, unsupported precision or age-scale assumptions, obsolete qualifications,
and missing table-unit provenance remain.

The one dispatched review batch failed because a judgment combined `supported`
with an uncertain dimension. Original live review coverage is therefore zero
complete aligned batches. The total was **nine logical calls**: eight repairs and
one review. The retained ledger records two confirmed OpenRouter charges and retains
the failed paid call's conservative reservation. No circuit, deadline or budget was
reset; no model profile was promoted.

### Post-run changes, separately validated

- **Prior parameters:** `prior_distribution` records uniform support or normal
  location, SD and truncation. A normal location may lie outside truncated support;
  it is not automatically a truncated mean. These proposals require a prior or
  secondary-report role and never produce an automatic calendar event interval.
- **Elapsed durations and offsets:** `elapsed_duration` records a length and bound
  relation; `relative_offset` records a quantified before/after relation to a named
  event. Neither takes a present epoch or generates an absolute date. Mixed-era
  endpoints, elapsed ranges and unsupported distribution families remain shape gaps.
- **Wire contract:** proposed patches now require a represented quantity; unresolved
  patches require null. The schema itself excludes `proposed` plus `unknown`, rather
  than relying only on a later Python validator.
- **Review recovery:** valid individual judgments survive a failed batch, with invalid
  items and missing targets retained. Any batch error still withholds eligibility for
  every judgment. Replaying the retained failure recovers three structurally valid
  judgments and one invalid judgment; it does not convert the original live failure
  into a success.
- **Instructions:** source fidelity is separate from historical confidence; a null
  optional scenario label is not uncertainty by itself. Context-only assertions and
  assertions outside a repair batch are not automatically extraction omissions.

The two new prior forms are demonstrated against retained source spans in
`post-run-prior-examples.json`. They are coordinator-authored shape examples with
original qualifications still requiring review, **not additional model successes**.
New schema, alignment and review-prompt gains remain live-unmeasured. Request/schema
hashes and frozen implementation snapshots distinguish the execution versions.

Validation: **227 dating tests**, managed workflow and all three publication checks
pass. All **3,575 claims and the complete 8,625-code view remain exactly unchanged**,
with every `spoken` null. See the experiment's `coordinator-audit.json` and
`validation.json` for the full failure dispositions and scientific digests.

Immutable editorial archive:
[`65c14ac1a474c44b0e5e715102cbdf5d14aee1ac99c513da9600b11bc91fe4f2`](../../../data/artifacts/dating-native-repair-2026-09-20/65c14ac1a474c44b0e5e715102cbdf5d14aee1ac99c513da9600b11bc91fe4f2/manifest.json).
It retains both phase implementations, eighteen request/response/failure snapshots,
source references and texts, the compressed pre-run scientific baseline, coordinator
audits, post-run implementation/schemas and validation. Final managed workflow:
`6fc260ce40cc4f0cad3da5f7f3050b59`. Completed calls reported $0.07523325 in OpenRouter
charges; the failed call retains $0.50 reserved, for $0.57523325 accounted/reserved.
Subscription consumption is recorded separately.
