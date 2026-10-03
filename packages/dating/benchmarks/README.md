# Benchmark scope and provenance

Three real-data checks compare deposited tree summaries with published root statistics. They deliberately test different scales and statistics: IE median in millennia, Sino-Tibetan mean in millennia, Uralic approximately reported root age in centuries. Each asserts taxon count and HPD type, and preserves the distinction between native BP quantities and an unverified calendar epoch.

| Analysis | Source | Publication reference | Tolerance |
|---|---|---|---|
| Heggarty et al. 2023 main M3, 161 tips | [Zenodo deposit](https://doi.org/10.5281/zenodo.8147476) | Median 8120 BP; HPD 6740–9610 | 5 years (rounded publication values) |
| Sagart et al. 2019 relaxed FBD, 50 tips | [Zenodo deposit](https://doi.org/10.5281/zenodo.2598440) | Mean 7184 BP; HPD 5093–9568 | 1 year |
| Honkola et al. 2013, 17 tips | [Phlorest pinned commit](https://github.com/phlorest/honkola_et_al2013/tree/e7899550daeafb249182fe509f6c7fae3ecbc393) | Approximately 5300 BP; HPD 3330–7503 | 50 years (root rounded to a century) |

The Uralic deposited mean is 5266.054 BP and median 5112.612 BP. The paper's approximate 5300 should not be relabeled as its median. Phlorest explicitly documents centuries. The raw tree, README and CC-BY-4.0 license are retained in `fixtures/phlorest/`; cite both Honkola et al. and the pinned derived collection. This is a curated representation of the same study, not independent support for its date. IE/ST deposits remain in their existing cache paths; their manifest hashes pin the exact inputs.

The small `semantic_cases.json` is a **development** set of source-grounded paraphrases, including weak topology support, calibration versus posterior, documentary samples, explicit epoch and contact chronology. These are reviewable labels, not specialist consensus or a held-out evaluation. Paraphrases are labeled and are never passed off as verbatim source spans. Predictions must be supplied separately; the benchmark reports `not_measured` without them. Do not report benchmark fixture agreement as LLM accuracy.

To extend evaluation, sample full passages stratified by family, event type, source quality, negative examples and contested mapping. Freeze source text hashes and annotations before running a model. Review disagreements independently where possible; report precision/recall, abstention, mapping and numeric fidelity separately, with uncertainty. Keep development cases out of the held-out test split. No statistically meaningful quality threshold can be inferred from twenty development cases.

Tests under `tests/` additionally check negative controls: fabricated quote suffixes, wrong epoch, mixed temporal forms, family-to-stage leakage, stale/content digests, missing references, dependency cycles, source duplication, missing tree taxa/units, and complete replacement by an empty extraction.

The [review protocol](../docs/review-protocol.md) specifies independent annotation, dependency-cluster reservation and separate extraction/adjudication metrics. No verified held-out passages have yet been reserved. Missing predictions/fields score as omissions, including when the reference value is null; an explicit null is a submitted unknown. The [hundred contact case](../families/uralic.md) is additional development evidence, with graph-integrity regressions, not an independently labeled evaluation case.

The follow-up adds explicit cases for Sagart corpus/basal uncertainty, Zhang rooting sensitivity, Zeng population associations, critiques, concept-translation/topology limits, model lineage spans, manuscripts, regional shift, etymology and duplicate deposits. Constructed controls and review-supplied cases awaiting full text are labeled in their locators; none is presented as a verified quotation.

