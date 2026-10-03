# Full-source diagnostic, 20 September 2026

This follow-up tests the actual extraction schema on a contrasting family before
autonomous expansion. The unit is **one dependent cluster**, Grollemund et al.'s
[Bantu study](https://pmc.ncbi.nlm.nih.gov/articles/PMC4629331/) and
[Ehret's commentary](https://pmc.ncbi.nlm.nih.gov/articles/PMC4640748/).
It is neither two independent confirmations nor a representative accuracy sample.
No output was imported as scientific evidence or selected as a default.

## Inputs and review

Shared acquisition retained original PMC HTML and receipts. Two Europe PMC XML
routes failed with HTTP 500; successful HTML acquisition does not erase them.
Article-element parsing supplied 67,898 and 8,354 characters respectively, including
the main study's embedded supplementary text. The linked workbook, deposited data
and figure pixels were not supplied; their omissions are explicit. Captions and
embedded text are available. Licensing remains as recorded in acquisition receipts.

The [protocol](../../../data/workspaces/dating/_research/full-source-evaluation-2026-09-20/protocol.json)
was written before reading the complete bodies. Source-first Claude Opus/high and
Codex Astra/high calls produced three completed annotations and one 480-second
timeout on the longer paper. Completed annotation counts were 54/18/16; quote
matches were 50/18/16. Claude repeatedly confused the age with the epoch. These
annotations were reviewed against the source, never treated as gold by agreement.
The coordinator used the documented self-audit fallback for the timed-out review.

The [frozen reference](../../../data/workspaces/dating/_research/full-source-evaluation-2026-09-20/reference-frozen.json)
contains 26 selected sentinel assertions (18 main article, 8 commentary), with
checked source offsets. This is a subset denominator, not exhaustive-paper recall.
Reference matching uses subject, event and scenario before numerical fidelity;
duplicates and additional predictions remain visible. A freeze-script path error
meant the first extraction was dispatched before the final reference file was
written. The draft labels already existed and no prediction had been inspected.
That deviation is recorded; it is not advertised as perfect preregistration.

The user's BP reference arrived during reference review. The amendment distinguishes
the conventional 1950 interpretation from an explicitly source-stated epoch, and
from radiocarbon calibration. Original prompts and outputs remain unchanged.
See [time interpretation](time-interpretation.md).

## Original full-source results

| Backend/configuration | Main article | Commentary |
| --- | --- | --- |
| OpenRouter, Gemini 3.8 Flash/medium | Completed, 6 claims, 6 matched quotes | Completed after schema repair, 7 claims, 7 matched quotes |
| Claude Agent SDK, Sonnet 5/medium | Validation failure; raw 15-claim response retained | Validation failure; raw 8-claim response retained |
| Codex, Terra/medium | Capacity failure | Completed, 6 claims, 6 matched quotes |
| Codex, explicit Astra/medium alternate | 480-second timeout | Not rerun in this version |

The [original adjudication](../../../data/workspaces/dating/_research/full-source-evaluation-2026-09-20/adjudication-original.json)
records one-to-one matches and partial interpretations. OpenRouter main output
matches 6/18 selected subjects/events, and the completed commentary outputs each
match 6/8. **These are presence matches, not all-field correctness or supported
precision.** Entity/sample adjudication remains incomplete, and several matched
claims have temporal or role errors. Failed calls remain failures, not zero-cost
successes or repaired predictions. No configuration meets the release criteria.

Concrete findings:

* Codex encoded “10th century BCE” as 10 BCE despite correctly describing the
  century in prose. OpenRouter used a different boundary in native values than
  in its conversion prose. Expanded millennium endpoints failed the old literal
  number check even when historically correct. Typed ordinal units now support
  programmatic period expansion and matching word ordinals in the quote.
* Claude's main response mislabeled posterior node ages as lineage spans and
  calibration dates as relative constraints. Its commentary called population
  evidence an attestation and expansion a linguistic divergence. Reversed native
  bounds and interval metadata without endpoints caused validation failures.
* The main OpenRouter response retained the two posterior ages and four priors,
  but omitted the selected archaeological, environmental and population evidence.
  Quote matching alone concealed that low coverage.
* “By” constraints, secondary reporting and relative environmental ordering need
  separate checks; correct numbers do not establish correct assertions.
* The source reports elapsed durations and uncertainties that the current calendar
  transport cannot fully represent. Preserve their wording/gaps, never invent a
  year-zero span. Standard errors/deviations must not become HPDs. A separate code
  bug promoted any HPD probability to the legacy `hpd95` field; that is repaired,
  while the complete native probability remains in v2 evidence.

## Development corrections and operating implications

Subsequent versions are exposed development tests. Version 2 added typed ordinal
units, separate conventional BP interpretation and age-scale checks, and clearer
event/interval instructions. Codex and baseline Claude returned valid commentary
outputs with matched native quantities; role/boundary errors remained. OpenRouter
hit an upstream 429. Stronger Claude still failed validation. Its main-article
cell was stopped before dispatch so the strength comparison could use version 3.

Inspection found roughly 2,000 catalog rows in the original
extraction system prompt. Version 3 removes that list and requests literal source
subjects with null catalog mappings. This implements extraction-before-mapping and
reduces irrelevant input; it does not establish the historical identity of a clade.
The actual system string shrank from 59,597 to 6,371 characters (the earlier
65,773-character measurement was its JSON-escaped representation).
Version 3 repeats the main paper on all backends, including an explicit Codex Astra
alternate, and compares Claude baseline/strong on identical revised inputs.

The shorter prompt's baseline Claude and Codex outputs each contained 18 claims,
with 15 individually schema-valid. The same three otherwise well-formed assertions
used a null unit for literal “y ago”: the instructions correctly distinguished that
notation from BP, but the schema offered no corresponding unit. This is a harness
representation defect, not evidence that those models needed more reasoning.
Adding explicit `years_ago`/`ka_ago` units resolves the transport gap while preserving
unknown epochs. The [post-hoc transport diagnosis](../../../data/workspaces/dating/_research/full-source-evaluation-2026-09-20/transport-diagnosis-v3.json)
records the three proposed unit substitutions for each output: the diagnostic
copies validate and all 18 quotes match. Original calls remain failed, diagnostic
copies are not imported, and semantic accuracy is not inferred from this repair.

OpenRouter's main version-3 response used an invalid convention identifier and
put conventional 1950 into the source-explicit field as well. Its commentary
response used the new ordinal and BP fields correctly but returned only five
claims, omitting the author's chronological inferences. The endpoint distinction
matters as much as successful transport. A final constructed three-backend probe
checks the new native-ago representation; it is not another full-paper accuracy test.

The stronger Claude version-3 run completed in 339 seconds with 23 claims and 23
matching quotes. It mentions 17/18 selected main-paper assertions, but posterior
nodes are labeled linguistic divergence and calibration nodes relative constraints.
This is **assertion presence, not correct event/role recall**. One conservative
native-support check fails because its quote omits the BP unit, although adjacent
source context establishes it; the numbers themselves match. Future section
packets must preserve such definitions rather than misclassifying context loss as
fabrication. The [development adjudication](../../../data/workspaces/dating/_research/full-source-evaluation-2026-09-20/adjudication-development.json)
retains these distinctions and additional predictions. This one completed result
does not justify an automatic strong-model upgrade, especially because the baseline
failures largely reflect the schema's missing native-ago unit.

Runtime changes now preserve selected raw validation-failure responses and costs,
share a locked call/cash-reservation ledger across versions, require a frozen
reference hash for new full-source dispatch, and clean up inherited POSIX process
groups on timeout or worker exit. A reservation is a dispatch control, not a
provider billing guarantee; unknown failed paid charges retain their reservation.
Subscription nominal estimates are not cash costs. Detached provider processes
outside the owned group are not covered by the cleanup guarantee.

Do not replace the whole workflow with a stronger model on these results. Use
typed deterministic checks, source-first extraction, bounded sections with their
definitions/headings, and separate semantic review. Compare models within each
role and retain complete failures. The next operational trial should measure
discovery and attachment recovery as well as extraction, with fixed follow-up
budgets and explicit gaps. A missing expert is not a blocker; limited model review
and self-audit remain explicitly limited evidence.

## Retained result and validation

The completed [evaluation artifact](../../../data/artifacts/dating-full-source-evaluation-2026-09-20/ca31b596e8523a8ef066843c4ced8b54e6928dce19d49bf337a60a5349ade337)
contains source text/inventories, protocol, references, adjudications, all run
plans/configurations/implementation snapshots, linked request/response/failure
evidence, the shared ledger, tests and workflow receipt. Original HTML remains in
the two acquired source artifacts identified by its provenance file.

Across source-first annotation, full-source experiments and the final control,
23 calls were attempted: 13 completed and 10 failed; one additional planned cell
was stopped before dispatch. These different task types are not an accuracy
denominator. All three final native-ago controls passed. Known OpenRouter response
charges total **$0.3694365**; failed-call uncertainty keeps the conservative cash
ledger at **$4.2503005**. Claude's **$5.2938882** is a nominal SDK estimate, not
subscription cash expenditure; Codex cash cost is unknown.

**184 tests passed**, the dating workflow completed, and all three publication
reproduction checks passed. Exact before/after comparison preserves **3,575 claims**
and the **8,625-code view**, with every `spoken` value null. The final workflow receipt
is `d1a07dc62d964d6980535e2749a1ea3e`. No current model configuration is promoted to a
validated general-purpose autonomous researcher by this trial.
