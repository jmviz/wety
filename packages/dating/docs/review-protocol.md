# Pilot annotation and evaluation protocol

Version 3, 20 September 2026. Model review and documented self-audit are accepted; no human reviewer is required. See [the feasibility assessment and operating policy](model-review.md). All current packets and semantic fixtures are exposed development material. **There are zero reserved, verified held-out passages and one completed separate-session model review (Uralic).** That review froze 32 source-first assertions before label comparison, using instruction-based separation. Shared-model errors and pretraining exposure remain possible. Its integration packet (`data/workspaces/dating/_research/packets/uralic-review-integration-2026-09-20/packet.json`) records scope and corrections. This is not project-held-out evaluation; a prospective sampling plan must not be relabeled a frozen test set.

## Unit of review

Select approximately 30–50 development passages/records across IE, Uralic and Sino-Tibetan. Include surrounding argument, footnotes, tables and source definitions needed to interpret the assertion; a sentence with a number is often insufficient. Pin original bytes, parsed text, parser/version, exact spans and rendered page/table locators. Flag defective transcription separately from missing text. Negative examples need a bounded inspected scope; zero extracted claims alone is not a negative label.

For each assertion record:

1. Literal entity and source-specific identity: sampled descendants, documentary corpus, reconstructed stage or population; catalog relation/certainty and explicit reasons to abstain.
2. Event/estimand, method, epistemic role and analysis/result identity. Separate quoted results, priors, demonstrations, posterior outputs, hypotheses and objections.
3. Native number, statistic, interval type, units and locally established epoch; preserve uncertainty about quantity and about interpretation separately.
4. Exact evidence and dependencies: source/result/analysis/dataset/calibration links, assumptions, missing context, and whether an inference is editorial rather than explicitly asserted.
5. Four separate judgments: textual fidelity, semantic interpretation, entity mapping and historical plausibility. A passed source-fidelity check does not resolve a disputed historical explanation.
6. Reviewer identity, date, relevant expertise, prior exposure, confidence in each judgment, rationale, alternatives and unresolved questions. A fresh model session can provide a second process-separated review; disclose shared-model dependence. Another pass in the same conversation is self-audit, not an independent annotation.

Contact cases also need donor/recipient direction, source-specific forms, locality if known, event identity, conditional premises, and scoped ordering constraints. Distinguish a phonological conditioning environment from temporal order. Keep alternative graphs separate; no conjunction, date intersection or selected scenario without a documented decision.

## Independent review and disagreements

A second reviewer, including a fresh frontier-model session, first annotates without seeing the first labels, while retaining access to the primary context. Then compare fields and reasons. Record both versions, the adjudicator and any resolution; retain contested alternatives when the disagreement is substantive. Report reviewer scope and relevant expertise without treating specialist status as proof. Textual errors can be corrected; historical uncertainty need not be voted away.

Prioritize the “hundred” sound-law and loan-adaptation argument, Maurits’s demonstration scope, Chang/Heggarty ancestry assumptions, and Sino-Tibetan tip/rooting semantics. Proceed with the remaining source packets and scenario work regardless of second-review availability. Use a structured adversarial self-audit when a fresh session is unavailable. A human specialist is optional; unresolved source or historical questions constrain the affected assertion, not unrelated work. Model-authored reference labels must be identified as such.

## Reserve evaluation without leakage

Before reporting a project-held-out evaluation, a separate model session or other reviewer not constructing extraction prompts should choose a separate set of roughly 30–50 passages from unexposed source/dependency clusters. Partition whole studies or connected analytical dependencies, including supplements, reused tables, critiques quoting the same result and repository copies. Distinct DOIs do not establish independence. Uncertain dependence warrants conservative co-grouping or an explicit exclusion from the independent estimate.

Stratify both sets by family and semantic risk: root/subgroup scope, ancient-taxon omission, prior/posterior, secondary report, documentary sample versus first attestation, manuscript/composition, native transmission, relative contact, population association, missing epoch and a negative/abstention case. If a stratum cannot be independently populated, report it as unmeasured instead of borrowing development examples.

The reservation manifest must list each case’s source/text hashes, context boundaries, dependency cluster, split, exposure history and reviewer. Keep reference labels inaccessible to the extractor/prompt-development process where possible; otherwise disclose instruction-only separation and any exposure. A fresh-session blind review of an existing pilot is useful but is not project-held-out evaluation. Model pretraining exposure is unknown. Freeze labels and scoring rules before generating predictions, then retain model/version, prompt/configuration, input/output hashes and run receipts. Any leaked case moves to development and is replaced before evaluation. Current agents cannot retroactively make already-read pilot passages blind; existing pilots can still support clearly labeled within-study diagnostics.

## Scoring and decision gates

Score retrieval completeness, extraction and adjudication separately. Use field denominators that distinguish explicit unknown/abstention from missing output. Record all reference assertions in the inspected scope for recall, all predicted assertions for precision, and unmatched/duplicate assertions. Report event/role/entity accuracy, numeric/statistic/epoch fidelity, exact-span integrity, abstention coverage and correctness, and scenario contamination. Disclose matching rules before running predictions. Do not score a silent omission as a correct null.

Give per-stratum counts, disagreement rates and error examples alongside aggregate scores. Resample or report uncertainty at the dependency-cluster level; correlated quotations are not independent trials. A small pilot is diagnostic, not an estimated universal historical accuracy.

Source recovery and bounded editorial work on further pilots may proceed now using model review or self-audit. The gate for **promoting evaluated output into supported evidence** is: no unresolved fabricated evidence, prior/posterior substitution, unknown-epoch conversion, catalog date transfer or mixing of alternative scenarios in the evaluated outputs; all other observed errors have a recorded disposition and regression where appropriate. Failed cases remain visible and predictions are never silently repaired before scoring. When labels are model-authored, report model-reference agreement and annotation uncertainty, not expert-validated accuracy. This qualitative gate does not authorize mass ingestion. Before broad expansion, precommit quantitative precision/recall and abstention targets and obtain enough independent clusters to estimate their uncertainty. Historical defaults additionally require the complete scenario review in the agenda, regardless of extraction score.
