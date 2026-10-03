> Historical audit: original paths are resolved by `data/registry/migrations/2026-09-reorganization/relocations.json`. See the package README for current commands.

# Version 2 overhaul audit

**Follow-up:** [the September 19 critique review](critique-review.md) identifies and repairs gaps that remained after this first pass, and distinguishes them from unfinished evidence adjudication. Use [the current agenda](next-steps.md) for subsequent work.

This document records the disposition of the review findings. **Implemented safeguards are not completed linguistic adjudications.** Existing claims remain candidates until their textual, semantic and entity-scope checks are recorded. The previous artifacts and plan are archived. No universal birth/death inventory is asserted.

| Review finding | Implemented response | Verification / remaining limit |
|---|---|---|
| Parent crowns became descendant birth dates; proto stem–crown became lifespan | Removed automatic lifespan synthesis; separate exact code chronology, catalog context and ancestry context | Full-export regression over 8,625 codes; `spoken=null` everywhere |
| Modern Arabic/Chinese ended in ancient periods; Latin/Sanskrit stages shared coarse dates | Catalog link no longer authorizes a stage-specific claim; source-specific subject identity and explicit mapping relation | Regression includes affected codes; precise stage mappings still require evidence review |
| Attestation, sample date, period, prior and posterior conflated | Typed event/role/subject/time separation; IE-CoR priors retained as raw sample priors; Glottolog documentation no longer becomes periodization | Contract tests and semantic development cases; legacy roles are provisional |
| Archaeology/population dates treated as language dates | Distinct population subjects/events and hypothesis role; no automatic projection to linguistic entity | Source-grounded interpretation still needed for individual proposals |
| Borrowing/contact and relative chronology lost | Relative claims can retain unknown absolute time; structured relations reference events and evidence; strict cycles rejected | No automatic language-lifespan constraints on etymology; populating the contact graph is future evidence work |
| Glottolog family treated as exact sampled clade | Exact source taxon sets identify clades independently; catalog relation explicit; unmapped taxa cannot be silently dropped | Reproduction of 100-versus-1000 inflation bug; manifest scope checks |
| Tree annotations stripped; HPD/median lost | Parser retains BEAST annotations, raw annotation text, summary statistics, clade support and topology parents | Real IE/Uralic/ST deposits tested |
| Units, epoch, posterior status guessed | Manifest requires units with basis, sample type, file hash and taxa; missing epoch keeps native values | BP conversion rejects implicit epoch; summary `.trees` file is not mistaken for posterior |
| Arbitrary burn-in/thinning; ETI called HPD | Explicit burn-in and justification; referenced diagnostic trace hash; no default thinning; labeled ETI for sampled quantiles | Diagnostic values are declared, not recalculated by this importer; full inferential reruns are outside this overhaul |
| Clade age interpreted without topology uncertainty | Exact descendant sets, node support, conditional age label and alternative parent sets retained | Deposited summaries cannot recover joint posterior uncertainty; do not infer it |
| Paper/deposit/D-PLACE copies counted independently | DOI/study identity separates bibliographic objects from analysis/results; no confidence by file count | Non-DOI historical source equivalence can still require explicit curation |
| Newest source/type won automatically | Removed rank-based adoption; reviewed claim selection only, all alternatives retained | No automatic historical adjudication; controversial analyses remain separate |
| Quote verification accepted invented suffix | Complete normalized quote match; native numeric assertions checked against quote numbers; hash/offset spans for strict support | Textual match alone never marks semantic correctness |
| Model-supplied BCE/calendar conversions trusted | Native numbers converted by code; explicit epoch; proper astronomical BCE conversion | Units/meaning still checked during review; failed/ambiguous extraction withheld |
| Model overrode endpoints; descendants stayed stale | Model output is immutable proposal; decision can only select existing eligible evidence | Global publication rebuild; stale input digests withhold old decisions |
| Stable IDs hid changed content; shorter reruns left old claims | Content-based extraction IDs, complete source/run replacement, immutable old/new snapshots | Empty extraction replacement regression; imported evidence changes require new IDs |
| Family-name gate excluded subgroups/critiques | Removed hard gate; aliases and subgroup/methodology searches; versioned screening | Search recall is not measured; excluded-source audit still necessary |
| Citation walk exception disabled followups | Fixed `must_mention` interface; enrichment/recommendation failures separated from followups | Network/API failures remain logged and retryable |
| Full-family papers restricted to task residue | Extraction sees all descendants of the requested family | Task partitions schedule work; they no longer define evidence scope |
| Long landing pages counted as full text | Raw/text caches, parser metadata, section-aware candidate diagnostic, publisher-link following | “Fulltext candidate” is not proof of identity/completeness; title inspection remains necessary |
| Key Uralic works produced zero evidence after poor retrieval | Recovered Honkola, Maurits, Grünthal plus Chang and Sagart primary texts; prior extraction metadata retained in history | No fabricated replacement model outputs; Phlorest adds 16 actual Uralic source-clade summaries |
| Raw hashes pointed to extracted text; provenance irreproducible | Separate byte/text hashes and caches, retrieval receipts, immutable extraction prompt/output provenance | Existing legacy raw bytes may be unavailable; migration flags their provenance as unverified |
| Loose models / schema-only checks admitted invalid records | Strict v2 models plus semantic/cross-record validation, exact-span checks, source/dependency integrity and cycles | Generic provenance fields remain extensible; they do not bypass support eligibility |
| Non-atomic file updates and stale derived data | Atomic writes, writer locks, source replacement snapshots, generation commit pointer and mirror checks | Multi-file mirrors are convenience outputs; read `CURRENT` for a transactionally consistent generation |
| Uncertainty/source identity dropped from CSV/exports | Full typed chronology retained as JSON in CSV; evidence references and source-native distributions retained | Consumers must migrate to the v2 contract |
| Review queue omitted low-confidence/unplaced claims | Every unsupported claim included; priority by affected code count, not a confidence score | Queue is large because legacy validation was insufficient; no silent suppression |
| Era parser truncated semicolon stages and erased qualifications | Multi-stage and qualified expressions abstain; raw expression retained | Rich prose requires source review rather than a guessed interval |
| Wikidata precision inferred from trailing zeros | Unknown precision when missing from source snapshot; original QID and statement retained | Source precision/references should be recovered before support |
| IE-CoR prior formulas and default epoch unverified | Removed speculative lognormal/calendar conversion; preserve raw prior parameters and sample identity | Parameterization must be verified against source XML before computing summaries |
| ASJP implicit epoch | New ingress preserves native BP value without assumed calendar conversion | Lexicostatistical dates remain model hypotheses, never general language lifespans |
| Model agreement mistaken for validation | Operational backend probe separated from scientific evaluation; optional independent prediction scoring | Semantic/model accuracy explicitly `not_measured` unless evaluated |
| No realistic cross-pilot benchmark | Three publication-fidelity checks, pinned Phlorest Uralic data, twenty semantic development cases after the follow-up, adversarial contract tests | Not historical truth, not specialist consensus, not a held-out accuracy estimate |
| Methodological assumptions hidden | Per-analysis worksheet records calibration sensitivity, cognacy, contact, rooting, sampling, clock, convergence, adequacy and limits | Current deposited analyses explicitly say where these were not independently reassessed |
| Sign/mixed/creole/contested entities forced into genetic chronology | Export no longer infers dates from ancestry for any category; unresolved mappings remain explicit | Appropriate case-specific historical models remain necessary |

## What the pilots now establish

The deposited summaries yield **225 source-clade results**: 160 IE, 49 Sino-Tibetan and 16 Uralic internal nodes. All are tied to exact source taxon sets and analysis manifests. Root checks reproduce the reported statistics within rounding precision. In the ST deposit, the main 50-tip summary and the 19-tip subset posterior are different analyses; the latter is not silently pooled or substituted for the former.

The total registry has 3,562 evidence records after migration. All currently require at least one unresolved review/conversion step; zero code-specific chronological assertions are promoted. This is deliberate disclosure of the evidence state, not a claim that no usable chronological evidence exists in the literature. The recovered texts and raw study results substantially improve the material available for review, but do not justify thousands of automated language lifespans.

## Methodological standard for subsequent analyses

1. **Define the estimand and taxonomic scope first.** Distinguish first split of a sampled clade, breakup of a dialect network, conventional stage boundaries, cessation of native transmission and survival in learned/liturgical use. State which extant/extinct varieties are absent. Catalog taxonomy is a hypothesis or indexing framework, not a clock calibration.
2. **Treat cognacy and lexical sampling as analytical decisions.** Audit cognate coding, loan identification, synonyms, missingness, semantic alignment and ascertainment. Test conclusions under alternative coding and taxon samples. Contact can affect both tree topology and rate inference; neither “borrowing always invalidates trees” nor “lexical models automatically handle borrowing” is adequate.
3. **Make calibrations auditable.** Identify the event each constraint dates, independent historical grounds, bounds versus point estimates, probability model and dependencies. An earliest attestation is usually not a split date. Historical sample ages are not root calibrations. Inspect effective joint priors and prior-only runs, leave-one-calibration-out sensitivity and alternative defensible bounds.
4. **Keep study scenarios intact.** Compare clocks, tree priors, roots, topology constraints, ancestor assumptions, taxon exclusion and calibrations as whole analyses. Do not assemble a synthetic tree from individually preferred node medians. Publish conditional clade ages alongside support and, when available, joint posterior samples.
5. **Distinguish computational adequacy from historical adequacy.** Multiple chains, ESS and convergence are necessary diagnostics, not proof of the model. Add posterior predictive checks and assess whether linguistic processes represented by the model are adequate for the corpus. Report sensitivity larger than conditional HPDs when appropriate.
6. **Use comparative/contact evidence on its own terms.** Shared innovations, relative sound-change chronology, borrowing direction and strata can constrain sequences without identifying a unique calendar date. Archaeological associations need a separate inferential argument; shared geography or genetic ancestry is not linguistic identity.
7. **Separate extraction evaluation from theory adjudication.** Source fidelity can be tested against publications and curated data. The historical plausibility of those publications is a different question. An unknown or contested answer should survive both stages.

## Research basis

These works motivate the safeguards and evaluation design; their presence here is not endorsement of every inferred chronology.

- [Maurits et al., calibration best practices and Uralic sensitivity](https://doi.org/10.1093/jole/lzz009): calibration events, distributions and sensitivity; a methodological demonstration is not an independently established Uralic chronology.
- [Honkola et al. 2013](https://doi.org/10.1111/jeb.12107) and [Phlorest's pinned derived dataset](https://github.com/phlorest/honkola_et_al2013/tree/e7899550daeafb249182fe509f6c7fae3ecbc393): publication and curated reproduction, explicitly dependent evidence.
- [Chang et al. 2015](https://doi.org/10.1353/lan.2015.0005): ancestry constraints and the consequences of historical/linguistic assumptions; source explicitly defines its BP epoch.
- [Heggarty et al. 2023](https://doi.org/10.1126/science.abg0818): sampled ancestors and analysis-specific IE estimates.
- [Sagart et al. 2019](https://doi.org/10.1073/pnas.1817972116): sampled ST clades, clock/root analyses and reported uncertainty.
- [Grünthal et al. 2022](https://doi.org/10.1075/dia.20038.gru): historical linguistic/contact arguments for Uralic expansion, requiring different representation from a lexical-clock posterior.
- [Greenhill et al. 2009](https://pmc.ncbi.nlm.nih.gov/articles/PMC2677599/): borrowing robustness must be assessed under specified conditions, not assumed universally.
- [Glottolog principles](https://glottolog.org/glottolog/glottologinformation): catalog classification and language identification do not supply temporal equivalence to every dictionary stage.

No paid model reruns, new phylogenetic inference or external specialist review are represented as having been performed. These limits are visible in both the artifacts and commands.
