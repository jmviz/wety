# Compact proposals and bounded source review

This opt-in workflow extracts source-native assertions, then requests bounded
semantic judgments. It never imports claims or selects historical defaults.
Existing claim readers and ordinary extraction remain compatible.

`CompactExtraction` removes catalog IDs, legacy method/kind labels, model-written
calendar fields and duplicated result metadata. Its temporal variants only expose
fields relevant to points, ranges, distributions, bounds or unknown time. Calendar
projection belongs to code. Each assertion retains the literal subject, subject
type, event, role, method, native quantity/expression, exact quotation, locator,
scenario, attribution, qualifications, relative constraints and sampled taxa.
Environment and artifact subjects are explicitly permitted.

A packet returns at most twelve assertions. Hitting the limit requires an overflow
flag and a scope gap; overflow never counts as complete extraction. Strict recovery
retains each invalid item and usable proposal separately. Duplicate JSON keys,
nonfinite numbers, unknown fields and scalar coercions are rejected. A supported
native assertion may lack a calendar projection. Missing epochs/calibration are
abstentions, not permission to guess. Quotations and native quantities are checked
against supplied source segments; context-only quotes remain identifiable.

The semantic reviewer receives the source packet and explicitly untrusted
proposals. It judges subject/event, role/dependency, native time and scenario/scope,
with a quoted rationale and a supported/revise/reject/unresolved disposition.
Unknown, duplicate or missing assertion IDs prevent a complete review. A reviewer
cannot waive failed mechanical provenance. Model support only makes a proposal
eligible for editorial consideration; it cannot approve an import or establish
historical truth. Source omissions and access gaps remain in the completion report.

## Commands

Run from the repository root. Set `transport` to `compact` in a section manifest;
retain a frozen reference and exact source/configuration hashes as usual.

```sh
.venv/bin/python -m harness.section_extraction prepare MANIFEST.json EXTRACT_DIRECTORY
.venv/bin/python -m harness.live_evaluation run EXTRACT_DIRECTORY --phase sections
.venv/bin/python -m harness.section_extraction report EXTRACT_DIRECTORY
.venv/bin/python -m harness.compact_review EXTRACT_DIRECTORY REVIEW_DIRECTORY --max-batches 4
.venv/bin/python -m harness.live_evaluation run REVIEW_DIRECTORY --phase reviews
```

The default reviewer rotation is OpenRouter extraction → Claude review, Claude
extraction → Codex review, Codex extraction → OpenRouter review. Different reviewers
see different proposals; these results cannot rank reviewer quality causally.
The planner allows only one review plan per extraction cell and records omitted,
missing and failed extraction inputs. Review plans share the extraction ledger.

A shared ledger can declare `deadline_utc` and `max_consecutive_failures`. The deadline
survives restarts and new phase directories; process-group timeouts cannot exceed
its remaining time. Consecutive failures stop that backend across phases. Uncertain
paid reservations remain held. Configuring OpenRouter `schema_repair: false` disables
the transport's whole-response repair, preserving the first response for item-level
inspection. Canonical schemas and effective settings remain in request snapshots.

## Trial scope

The Dravidian trial uses a research brief frozen before source collection, four
search-engine queries and two retained Crossref responses, title-only screening,
two selected author-written texts, four source packets and two bounded supplement
follow-ups. Sources are [Kolipakam et al. 2018](https://pmc.ncbi.nlm.nih.gov/articles/PMC5882685/)
and [Mahadevan's 2003 author-written book excerpt](https://www.tamilheritage.org/old/monument/oodu/mahadev.html).
The latter is a retained mirror, not the complete monograph or a publisher-original
capture. Supplement HTTP responses were HTML download challenges and never entered
the evidence text. Images and unretrieved deposits remain explicit gaps.

The coordinator selected sources using discovery results before model screening.
Thus this is **supervised end-to-end development**, not a demonstration of fully
autonomous source selection. Fourteen selected assertions were frozen before
extraction predictions; they are a coordinator-authored reference, not an expert
gold standard or exhaustive discovery/claim denominator. The primary text selection
excludes the study's bibliography/repeated data footer while retaining them in the
full parsed source. It includes the excerpt's source-authored sidebars, which should
not become language chronology simply because they contain dates.

## Results, 20 September 2026

The versioned workspace is
`data/workspaces/dating/_research/dravidian-trial-2026-09-20/`. Its `manifest.json`,
`brief.json`, `reference.json`, `source-inventory.json`, `sentinel-audit.json`,
`coordinator-audit.json` and `report.json` retain the protocol, inputs and outcomes.
The actual source-defined model epoch is **2000**. Secondary cited dates do not
inherit it. BP convention, explicit author epoch and radiocarbon/calendar scale
remain separate concerns; bare “years ago” is not automatically BP notation.

There were **21 logical calls**: three screening, ten attempted extraction and eight
review. Two additional extraction cells were skipped by the OpenRouter failure
circuit. All three screenings completed. Extraction yielded six completed envelopes,
two schema-invalid epigraphy responses and two provider failures. Strict recovery
retained **32 Claude and 39 Codex proposals**, with 15 invalid items still visible.
No backend completed all document coverage; Codex also reported section overflow.

| Backend/profile | Selected assertions found /14 | Matched assertions schema-valid /14 | Matched assertions mechanically ready /14 |
| --- | ---: | ---: | ---: |
| OpenRouter Gemini 3.8 Flash, medium, Vertex | 0 | 0 | 0 |
| Claude Agent SDK Sonnet 5, medium | 14 | 8 | 5 |
| Codex Astra, medium | 13 | 9 | 8 |

These are selected-reference presence and mechanical diagnostics, **not supported
precision**. Invalid raw items count as found; explicit numbers left only in prose
can pass quote/shape checks. The reference's best-model root HPD scope itself needed
qualification: the text describes approximate intervals across analyses, not
unambiguously one distribution. Its frozen original and audit correction are both
retained. The reference is not exhaustive or independently held out.

OpenRouter's two compact calls failed with Vertex HTTP 400 `INVALID_ARGUMENT`.
The cause remains unresolved. Provider schema complexity is one documented possible
cause, not a diagnosis of these requests ([Google's structured-output documentation](https://docs.cloud.google.com/gemini-enterprise-agent-platform/models/capabilities/control-generated-output)).
The failure circuit was preserved. A declared routing amendment sent Codex proposals
to Claude and Claude proposals to Codex, using the same ledger. To specify such a
routing explicitly, add `--routing PATH.json` to `harness.compact_review`; the file
maps all three backend names to different supported reviewers. This never resets
the deadline or stop conditions.

Seven review batches completed and one failed because “supported” conflicted with
an “uncertain” dimension. **59 of 71 proposals received aligned review judgments:
35 revise and 24 supported.** Mechanical/source gates reduced the latter to twenty
eligibility diagnostics; none were imported. The failed batch's twelve targets and
raw judgments remain retained, not counted as reviewed successfully. Reviewers saw
different proposals, so these counts do not establish a reviewer-quality ranking.

The concrete utility result is a source-bound, qualified **secondary contact claim
around 1200 BCE**, kept distinct from a family divergence, plus documentary and
composition distinctions. It does not establish the date of any particular borrowing
edge. Reviewer/coordinator disagreements and source gaps preclude a claim of complete
semantic precision. The limited development-utility criterion passes; autonomous
source-selection and mass-release criteria do not.

The run exposed and led to corrections for:

- A legacy astronomical-conversion directive embedded in the compact prompt;
  removed, with positive native BCE/CE validation and explicit ordinal examples.
- Numeric over-abstention despite explicit dates; flagged for review, with instructions
  to preserve numbers independently of calendar convertibility.
- Reviewers assuming unavailable mixed-era/relative-offset shapes and requesting
  invented narrower dates; reviewers now receive the proposal schema and explicit
  representation limits. An enclosing century range must retain its narrower wording.
- Context-only evidence passing an eligibility diagnostic; primary-segment provenance
  is now required.

These post-trial prompt corrections are **offline tested; their live quality gain is
unmeasured**. They do not retroactively repair the trial outputs. No ordinary
production profile is promoted. Compact schemas are smaller, but smaller output
alone did not guarantee scientific fidelity or provider compatibility.

Known OpenRouter cash is **$0.00771075**; two failed requests retain **$1.50** in
uncertain reservations, for **$1.50771075 accounted/reserved** against the $6 ceiling.
SDK cost estimates and subscription token usage are retained separately and are not
claims of free computation. The shared 30-call/35-minute budget was not reset.

Final validation: **207 dating tests**, the managed rebuild and all three publication
reproduction checks pass. All **3,575 claims and 8,625 code views match the baseline
exactly**, with every `spoken` null. See `validation.json` for the final workflow
receipt and scientific digests. The [operating package](harness-recommendations.md#operating-package-after-the-dravidian-trial)
sets the next small transport controls and autonomous queue milestone.

Immutable editorial archive:
[`c9af881d386dec519b46840f8dabb832735e387772380fbb58c6045a94b0f210`](../../../data/artifacts/dating-compact-trial-2026-09-20/c9af881d386dec519b46840f8dabb832735e387772380fbb58c6045a94b0f210/manifest.json).
It contains frozen phase implementations, referenced request/response/failure
snapshots, original source-artifact references, final implementation, review audits
and the final validation receipt (`06cc6daa6e1c40bbbde5598684f95965`).
