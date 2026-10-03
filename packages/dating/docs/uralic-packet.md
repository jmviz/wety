# Uralic source review, 19 September 2026

This is the first bounded scientific deliverable from the [agenda](next-steps.md): a comparison of Honkola's lexical-clock analysis, Maurits's calibration demonstration and Grünthal's contact argument. It contains 13 chronological assertions, 18 methodological findings and two explicit relative relations. Five assertions satisfy the registry's source-fidelity checks; eight remain candidates with unnormalized native dates. No historical default or dictionary-stage date is adopted.

The [editorial packet](../../../data/workspaces/dating/_research/packets/uralic-2026-09-19/packet.json) records retained PDF/text hashes, exact text spans, source versions, review scope and five catalog-transfer abstentions. Its companion `claims.json` is the reproducible import. This is a single-agent development review, not independent specialist annotation, exhaustive extraction or a held-out evaluation.

## What the sources actually support

| Source and reviewed material | Chronological interpretation | Material limitations |
|---|---|---|
| [Honkola et al. 2013](https://doi.org/10.1111/jeb.12107), published article, pp. 1244–1253 | Approximate root mean 5,300 YBP, 95% HPD 3,330–7,503; Ugric approximately 3,300, HPD 1,690–4,895, conditional on a clade with probability below 0.5. Three original uniform calibration supports are recorded separately. | No numeric epoch established here. Appendix S1 and Figure S1 remain unreviewed. The deposited summary does not preserve the full scenario comparison or posterior diagnostics. |
| [Maurits et al. 2020](https://doi.org/10.1093/jole/lzz009), published article, pp. 17–38 | Reuses Honkola's data/modelling while changing calibration targets and distributions. The reported mean increase of about 850 years and HPD widening of nearly 3,000 years are differences, not a new absolute root date. | Explicitly a methodological demonstration. Supplementary distribution definition and analysis files still require review. No independent chronological confirmation is claimed. |
| [Grünthal et al. 2022](https://doi.org/10.1075/dia.20038.gru), 62-page prepublication manuscript including supplements | Source argues initial differentiation before the approximately 4,000-BP Indo-Iranian contact episode, with relevant branch-specific sound laws applied to borrowed material afterward. This is a relative historical argument about author-defined stages. | Manuscript not collated with version of record. Numeric BP epoch unspecified. Donor-stage dating, loan stratification and archaeological attribution remain dependent claims requiring review. S4's R script is explicitly absent. |

Honkola uses 17 sampled varieties and 100 meanings selected from 226 combined concepts after excluding meanings with reported loans. This does not demonstrate the absence of contact effects. The article compares strict and relaxed clocks under a Yule prior and four gamma rate categories, and reports 32 runs across four calibration combinations and two prior shapes. Its diagnostics are author reports; this packet does not independently establish convergence or model adequacy. Article root/Ugric assertions reuse the deposited clades' result identities, preventing publication/deposit double counting. Exact correspondence between the deposited summary and individual runs remains unverified.

Maurits Note 4 explicitly defines present as **AD 2000**. That permits normalization of this article's reported priors without assigning an epoch to Honkola or Grünthal:

| Target | Native prior information retained | Normalized source-reported summary, astronomical CE |
|---|---|---|
| Finnic | Normal mean 1.225 ka, SD 0.23 ka; reported 99% range 670–1,850 YBP. Prose rounds mean/mode to 1,230. | Mean 775; reported range 150–1,330. |
| Permic | Reversed lognormal offset 1.85 ka, log-mean −0.18, log-SD 0.18; reported mode 1,041 and 95% range 661–1,263 YBP. | Reported range 737–1,339; no inferred mean. |
| Saamic | Normal mean 1.70 ka, SD 0.153 ka; reported 95% range 1,400–2,000 YBP. | Mean 300; reported range 0–600 (astronomical 0 is historical 1 BCE). |

These are recorded prior summaries, not posterior HPDs or language lifespans. The Finnic discrepancy remains visible rather than silently replacing the explicit parameter. The Permic minus sign was checked on the PDF because extraction loses it; the reversed distribution is not reconstructed without its supplementary definition. Replacing Finno-Saami and Samoyedic targets with Finnic and Saamic changes more than uncertainty width.

Grünthal distinguishes reconstructed Proto-Uralic from subsequently differentiating Common Uralic. The approximately 4,500-year argument additionally depends on an assumed roughly 500-year differentiation timescale; it does not establish a measured 4,500–4,000 interval. The packet records two source-argued relations: differentiation precedes contact; contact precedes the relevant sound changes applied to loans. Their supported status establishes faithful representation of the argument, not the truth of all loan identifications or absolute anchors. Climate, Seima-Turbino dispersal and language spread remain hypothesized associations.

## Entity scope and reproducibility

The five reviewed targets (`urj-pro`, `urj-ugr-pro`, `urj-fin-pro`, `urj-prm-pro`, `smi-pro`) all receive an explicit abstention from chronological transfer. Source clades and author-defined stages have exact local scope, while catalog equivalence remains unresolved. The current catalog generation is recorded; no catalog inputs or historical payload paths were rewritten.

```sh
.venv/bin/python -m harness.packets data/workspaces/dating/_research/packets/uralic-2026-09-19/packet.json --require-imported
uv run --frozen wety-data run dating
```

Packet validation checks source bytes, registered text identity, spans for candidates as well as supported claims, normalization, claim inventories and imported relative relations. Pilot coverage exposes this bounded editorial review separately from model extraction. The research graph records the reviewed calibration/data dependencies and the unreviewed donor/archaeological dependencies. Neither hashes nor successful tests establish historical correctness.

The [supplement follow-up](uralic-supplements.md) now recovers the published Grünthal article and supplementary files, verifies Maurits's prior parameterization, and reviews a concrete loanword ambiguity. The original assertions and this packet's dated review state remain preserved.

## Next scientific work

1. Complete this packet's source gaps: collate Grünthal against the published version; recover Honkola's supplements, Maurits's supplement/posteriors and Grünthal's S4 code with acquisition receipts. Review the donor-stage and loan-stratification anchors before promoting an absolute chronology.
2. Seek an independent review of the disputed entity scopes and source interpretations. Freeze a separate held-out passage set before evaluating extraction; these development assertions cannot serve as independent test data.
3. Build equivalent IE and Sino-Tibetan comparison packets, then evaluate extraction and complete scenario sensitivity. Select one concrete lexeme/contact case to test the usefulness of relative chronology. Broad ingestion and historical defaults remain gated on that work.

## Delivery checks

The root dating workflow completed successfully (run `4c16dc9231b943a4a6a6f5a7100deb39`), including validation and all three published-result reproduction checks. All 130 Python tests passed: 85 dating, 37 catalog and eight workflow tests. Extraction accuracy remains unmeasured.

The resulting registry has 3,575 assertions, including all 3,562 prior assertions unchanged. The entire 8,625-code view is equal to its pre-review value: `spoken` stays null, no code chronology is populated and no default is selected.

The packet was also preserved as an [immutable editorial artifact](../../../data/artifacts/dating-uralic-review/23229584b85b2752c54135d60ef22445adb00c26c899862eaba530373558272b/manifest.json), whose files pass artifact verification. This snapshots the review; it does not recover unknown historical acquisition provenance or replace the durable source PDFs.
