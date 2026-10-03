# First live backend evaluation

Run date: 20 September 2026 UTC. **Live development evaluation is complete across OpenRouter, Claude Agent SDK and Codex.** It found and fixed production extraction-schema and temporal-form defects. It does not establish full-paper accuracy, a preferred backend, or readiness for broad family ingestion.

## Design and retained evidence

The [run workspace](../../../data/workspaces/dating/_research/evaluation-2026-09-20/) contains versioned plans, complete per-backend configs, input locks, results, failure records, model catalog snapshots, and copied content-addressed application requests/responses. `analysis.json` aggregates the retained results. Original failed outputs and superseded plans remain intact. No run imported claims or selected historical defaults.

The corrected `live-v3` comparison made **36 calls**: three backends × baseline/stronger configuration × screen/extract/synthesize × two repetitions. Screening used ten constructed records and the production decision schema; extraction used thirteen short paraphrases; synthesis used seven short semantic controls. The latter two used diagnostic schemas, not the production extraction/review schemas. Reference labels were withheld from requests, but field names and a common vocabulary were supplied. All cases were exposed development material, not held-out passages. Repeated outputs and fields are correlated observations. Cells ran sequentially within each backend and concurrently across backends; ordering was not randomized. Cache effects and server load confound latency comparisons.

| Backend | Baseline screen / extract / synthesize | Stronger screen / extract / synthesize |
| --- | --- | --- |
| OpenRouter | GLM 5.3 Flash low / Gemini 3.8 Flash medium / GPT-6 Astra high | GPT-6 Astra high / high / xhigh |
| Claude Agent SDK | Haiku 4.5 / Sonnet 5 medium / Opus 5 high | Opus 5 high / high / max |
| Codex | GPT-5.6 Terra low / medium / high | GPT-6 Astra high / high / high |

OpenRouter endpoints were explicitly pinned: Wafer for GLM, Google Vertex global for Gemini and Azure for Astra. Automatic provider/model fallback and response healing were disabled. Those experiment settings differ from the production defaults. Codex and Claude used existing subscription authentication; direct Anthropic API access was unavailable and remains untested. Codex logs identify requested models, not independently resolved model versions. Reported provider/model identities and costs are retained where exposed.

`live-v1` established transport access: BaseTen's GLM route returned HTTP 429; the next version explicitly selected Wafer and passed. This is endpoint availability evidence, not a model-quality failure. `live-v2` was stopped after identifying benchmark defects: free-text labels were scored against an undisclosed vocabulary, one screening control omitted its family, a dependency-count field was ambiguous, and the sensitivity paraphrase lacked explicit posterior context. Its 18 completed calls are retained as diagnostics, not usable model-error rates; 19 cells were aborted before dispatch. V3 corrected the task specification before rerunning. The original benchmark fixture was not silently rewritten or rescored.

## Development results

All 36 V3 calls completed. Screening counts are matching decisions out of twenty repeated controls. Extraction/synthesis counts are matching fields, not historical accuracy.

| Backend/config | Screening | Extraction | Synthesis | Median seconds, screen / extract / synthesize |
| --- | ---: | ---: | ---: | --- |
| OpenRouter baseline | 20/20 | 94/94 | 34/34 | 22.7 / 12.4 / 6.0 |
| OpenRouter stronger | 20/20 | 94/94 | 34/34 | 16.9 / 15.8 / 9.1 |
| Claude baseline | 20/20 | 92/94 | 34/34 | 25.0 / 24.1 / 8.1 |
| Claude stronger | 20/20 | 93/94 | 34/34 | 7.5 / 28.0 / 17.4 |
| Codex baseline | 20/20 | 94/94 | 34/34 | 12.0 / 18.2 / 9.1 |
| Codex stronger | 20/20 | 94/94 | 34/34 | 21.0 / 26.0 / 12.6 |

Claude/Sonnet twice classified explicitly reported posterior ages from a calibration-sensitivity demonstration as `editorial`. The study's methodological purpose does not change those estimates' statistical role. This is an actionable distinction for source-based semantic review. Claude/Opus classified the terse Grünthal borrowing-strata paraphrase as `observation` once and `hypothesis` once. The reference says hypothesis, but the paraphrase does not adequately distinguish observed linguistic correspondences from the inferred dispersal argument. Preserve the mismatch and instability; improve the passage-level case before attributing it confidently to the model. Do not tune a model to this shorthand and call that improved linguistic understanding.

These mostly saturated controls do **not** show that stronger settings are generally better. Keep baseline candidates for the next controlled comparison, retain capable reviewers for contested claims, and select models only after full-context and unseen-cluster evaluation. No production model/provider defaults were changed from these results.

## Production-schema defect and verified fix

Separate constructed controls exercised the actual `Extraction` and `FamilyReview` schemas and production prompts. They specified two sampled clades and a documentary tip prior, an explicitly defined BP epoch and a second analysis with an unknown epoch. A review control used two different source subjects sharing the same catalog context.

Before repair:

- OpenRouter/Gemini returned native-time dictionaries with strings such as `"5000 BP"` and invented keys such as `hpd95`, `low` and `high`. Pydantic accepted them, but the normalizer could not use them.
- Claude/Sonnet also used incompatible native-time field shapes; its result failed the specified epoch/native-statistic checks.
- Codex first failed to initialize under the outer filesystem sandbox. A permitted retry reached the service and rejected the extraction JSON schema because its arbitrary dictionary lacked `additionalProperties: false`.

The first cause was an underspecified `source_values: dict`. Native time now uses closed, form-specific alternatives with explicit unit, numeric quantities, epoch, interval type/probability and relation fields. Structural checks run before missing-epoch abstention, so an unknown epoch cannot hide an invalid temporal shape. Ingestion serializes it to the existing native-evidence dictionary and normalizes in code. Unknown epochs retain native numbers and yield unknown calendar dates. Invalid numeric strings, invented keys and numbers hidden under `form=unknown` have regression coverage. Existing evidence payloads and identities were not rewritten. Pending extraction schemas become stale under the existing digest guards and must be resubmitted.

The first repair (`production-v2`) matched the requested fields on all three backends, but an added ingestion audit found incompatible shapes: point values mixed with means/medians, or distribution metadata on event ranges. The original field-only scores remain retained alongside a separate normalization audit; they are insufficient. Form-specific alternatives and actual normalization checks followed in V3. Codex passed; Gemini still described a uniform prior as an event range, and Claude failed local validation by supplying interval metadata without interval bounds. A general system-instruction clarification distinguishes distributions, means/medians, event ranges and sensitivity-study roles.

The final `production-v4` rerun passed **all specified source fields, quotations, temporal forms and programmatic normalization checks on all three backends**, using the same sources and model configurations. Each version made one extraction call per backend; this is an iterative development smoke test, not a reliability estimate. V3 diagnostic scores above describe the earlier frozen prompts; the entire 36-cell matrix was not rerun after the final instruction clarification. Production review controls also completed with both source subjects preserved and no forbidden override or cross-subject recommendation. Their checks establish structural coverage only, not the correctness of every rationale.

## Cost, reproducibility and limits

Across probes, superseded diagnostics, the 36 corrected comparisons and production checks, **74 calls were attempted; 69 completed and five failed**. OpenRouter reported **$0.782065** across 24 successful calls; one failed call has no reported charge. Claude reported **$1.240844 in nominal SDK estimates** across 20 successful calls, not subscription cash expenditure. One Claude validation failure has no retained nominal charge or full raw output. Codex's 28 attempts consumed subscription allowance with cash cost unknown. V3 alone reported $0.417078 through OpenRouter. No unknown cost is treated as a known zero.

The runner limits concurrency to one call per backend, uses a 240-second worker timeout and a $10 reported-cost stop per experiment directory, and supports a `STOP` file. That cost stop is not a hard account-wide spending guarantee; unknown failed charges and an in-flight request remain possible. No timeout occurred in this run. Before longer trials, add process-group cleanup and an experiment-wide budget ledger. Input locks bind plans/configs; request snapshots bind actual application prompts/schemas. These are not full provider wire transcripts. Original implementation bytes for earlier runner revisions and all failed provider response/charge details were not retained; this provenance limitation remains explicit. The final implementation is included in the adopted evidence set for recovery.

Read-only regeneration of a run summary:

```sh
.venv/bin/python -m harness.live_evaluation report data/workspaces/dating/_research/evaluation-2026-09-20/live-v3
```

Do not rerun completed directories to obtain a new sample. Create a new versioned directory/config/plan and lock its input before dispatch. The benchmark product's existing `not_measured` semantic metric remains unchanged: these modified development diagnostics are a separate evaluation.

## Next operational gate

No human review is required to proceed. Before claiming autonomous-family readiness: supply full source passages/attachments through the production extraction path; precommit assertion matching, omission/abstention scoring and severe-error criteria; reserve previously unexposed study/dependency clusters; and run a bounded contrasting-family trial that measures discovery and supplied-context completeness as well as extraction. Use the same three required backends, retain all failures, and separate improved inputs/schema from model upgrades. The pilot's inaccessible files can remain explicitly scoped gaps. See the [updated operating recommendations](harness-recommendations.md) and [agenda](next-steps.md).


Validation of the completed live-evaluation continuation: **165 dating tests pass**, the managed workflow (`ff2cbb2686fd4cbaab5d1c7e6ff7d226`) and all three publication-reproduction checks pass. All **3,575 claims and the complete 8,625-code view are unchanged** from the pre-continuation baseline; every `spoken` remains null. The full-source/held-out semantic benchmark remains explicitly unmeasured.
