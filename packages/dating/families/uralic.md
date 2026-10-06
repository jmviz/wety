# Uralic

**Summary.** Three studies reviewed at source level: Honkola et al. 2013 (lexical-clock root, mean ≈5,300 YBP, 95% HPD 3,330–7,503), Maurits et al. 2020 (a calibration demonstration on the same data, not an independent date), and Grünthal et al. 2022 (a relative argument placing initial differentiation before the ≈4,000 BP Indo-Iranian contact). 13 assertions, 5 supported as faithful records of their sources; no epoch is established for the YBP figures, and no sampled clade is equated with a Wiktionary stage, so nothing is exported to wety yet. A conditional contact case (“hundred”) shows how borrowing evidence constrains relative chronology without absolute dates.

Packets: `data/workspaces/dating/_research/packets/uralic-*`.

**Shown in wety (2026-10-05).** Honkola et al.'s deposited summary tree (manifest
`honkola2013_beast`) is now read with a labeled conventional epoch: the article writes
YBP with no epoch, so calendar years take BP as AD 1950 (a 2000 epoch would move them
50 years, small against HPDs of 1,000–4,000 years). Three proto ends, 95% HPD:
`urj-pro` until 5554–1381 BCE (root), `urj-fin-pro` 108–1263 CE (north–south Finnic),
`smi-pro` 632–1592 CE (East–West Saami). Not adopted: Permic and Samoyedic (calibrated
nodes, close to their priors), Ugric (support 0.47), Finno-Permic (0.29), Mordvinic
(Erzya only). Grünthal et al. 2022's relative argument stays a recorded alternative
view.

## Uralic source review, 19 September 2026

This packet compares Honkola's lexical-clock analysis, Maurits's calibration demonstration and Grünthal's contact argument. It contains 13 chronological assertions, 18 methodological findings and two explicit relative relations. Five assertions satisfy the registry's source-fidelity checks; eight remain candidates with unnormalized native dates. No historical default or dictionary-stage date is adopted.

The [editorial packet](../../../data/workspaces/dating/_research/packets/uralic-2026-09-19/packet.json) records retained PDF/text hashes, exact text spans, source versions, review scope and five catalog-transfer abstentions. Its companion `claims.json` is the reproducible import. This is a single-agent development review, not independent specialist annotation, exhaustive extraction or a held-out evaluation.

### What the sources actually support

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

### Entity scope and reproducibility

The five reviewed targets (`urj-pro`, `urj-ugr-pro`, `urj-fin-pro`, `urj-prm-pro`, `smi-pro`) all receive an explicit abstention from chronological transfer. Source clades and author-defined stages have exact local scope, while catalog equivalence remains unresolved. The current catalog generation is recorded; no catalog inputs or historical payload paths were rewritten.

```sh
.venv/bin/python -m harness.packets data/workspaces/dating/_research/packets/uralic-2026-09-19/packet.json --require-imported
uv run --frozen wety-data run dating
```

Packet validation checks source bytes, registered text identity, spans for candidates as well as supported claims, normalization, claim inventories and imported relative relations. Pilot coverage exposes this bounded editorial review separately from model extraction. The research graph records the reviewed calibration/data dependencies and the unreviewed donor/archaeological dependencies. Neither hashes nor successful tests establish historical correctness.

The supplement follow-up (below) now recovers the published Grünthal article and supplementary files, verifies Maurits's prior parameterization, and reviews a concrete loanword ambiguity. The original assertions and this packet's dated review state remain preserved.

## Uralic supplement and dependency review

This follow-up to the first packet (above) recovers six source files and records ten source-grounded methodological findings. It adds no date-bearing claims or defaults. The earlier packet and its assertions remain intact; this packet resolves some of its open review questions without rewriting their historical state.

### Recovered material

| Source | Result of this pass | Remaining limit |
|---|---|---|
| Grünthal et al., [published article](https://doi.org/10.1075/dia.20038.gru) | Retained 35-page published PDF, pp.490–524, with acquisition receipt. Selected contact-order, stage and epoch passages agree with the earlier manuscript's argument. | This is a selected collation, not a complete textual edition comparison. Published p.515 explicitly defines BP present as 2000; this convention is scoped to the published version. |
| Grünthal et al., [supplements v1.0](https://zenodo.org/records/6345559) | Retained the 47-page supplement, R script and input workbook. The earlier manuscript included only 29 supplement pages and omitted the script. | Complete supplement/coding adjudication remains open. Original R package versions are not pinned. |
| Maurits et al., [publisher supplement](https://academic.oup.com/jole/article/5/1/17/5688948) | Retained the DOCX defining the reversed lognormal and other calibrations. | It supplies no posterior samples, traces or model configuration for re-running the demonstration. |
| Holopainen, [loan appendix](https://zenodo.org/records/6035900) | Retained the workbook underlying the contact argument; reviewed its dependency metadata and the “hundred” example. | Not a substitute for reviewing the dissertation or all etymologies. The deposit supplies no explicit licence; the workbook's “Open Access” notice is not a specified reuse licence. |

Original bytes have SHA-256 hashes and shared acquisition receipts. The three files in Zenodo6345559 also match its published MD5 checksums. Parsed text retains cell coordinates, formula strings and cached values for spreadsheets. Independent comparison with the bundled `openpyxl` reader matched all 157 stored cells in the S4 workbook and 2,348 in the loan appendix. Original workbooks were not edited or recalculated.

### Numerical checks and their scope

Maurits's supplement defines the reversed lognormal as `X = offset − Y`, with lognormal `Y`. Using the first packet's main-article parameters in millennia gives mode **1041.359 YBP** and central equal-tailed 95% endpoints **661.379–1263.037 YBP**, matching the reported rounded values. Probability of positive age is approximately **0.999995**; the supplement explains optional truncation at zero. This resolves the parameterization question left open in the earlier review. Figure 4’s caption instead says a mode near 1250; that conflicts with its parameters and main text, which give 1041. Preserve this source inconsistency explicitly. It does not validate the historical calibration argument or replace the recorded interval label with an HPD.

All ten results in Grünthal's TableS4.2 reproduce at three-decimal reporting precision using the deposited counts and distances and the stated one-sided Mann–Kendall normal approximation with continuity correction. There are eight branches for PU etyma (Samoyedic excluded) and nine for Indo-Iranian loans (included). The checker uses a separate standard-library implementation. It does **not** run the original R environment, reproduce every sensitivity run or calculate Bayesian chain diagnostics.

The R script expects `PU_I-I_2021-data.xlsx`; the deposited filename includes ` copy`. A future R reproduction must record that filename mapping without altering the original artifact. The script changes Saamic distances by adding281km to Finnic distances, while the supplement rounds this to280km; the published main table uses the original great-circle distances.

Nonsignificant negative-trend tests do not establish the absence of isolation by distance, prove rapid dispersal, or identify a homeland. Power, branch dependence, distance proxies and coding sensitivity need separate evaluation. These tests do not supply a calendar date.

### A concrete etymological disagreement

The loan appendix's metadata says it derives from Holopainen2019, with additions in columnsR,AZ,EQ,ER. It is one dependent source, not an independent confirmation. Its “hundred” column (`Indo-Iranian!DW`) records a proposed donor and recipient reconstruction, but marks the Permic reflex cells `DW41` and `DW42` with question marks. The note in `DW12` considers **parallel borrowing** and a **regular sound-law explanation** of Permic vocalism as alternatives.

This is a useful next wety case: identify the specific borrowing(s), the disputed reflexes, and the proposed sound rule, then express only the chronological order that each interpretation permits. The appendix alone cannot settle which analysis is correct or attach an absolute borrowing date. The supplement also explicitly allows gradual borrowing as an alternative to a drastic contact explanation for Samoyedic phonotactics (p.3).

### Reproduce

From the repository root:

```sh
.venv/bin/python -m harness.packets data/workspaces/dating/_research/packets/uralic-supplements-2026-09-19/packet.json --require-imported
.venv/bin/python -m harness.uralic_checks
uv run --frozen wety-data run dating
```

The [follow-up packet](../../../data/workspaces/dating/_research/packets/uralic-supplements-2026-09-19/packet.json) binds exact source spans and review limitations. Its `numerical-checks.json` records the calculations; `retrieval-attempts.json` records access attempts. The numerical checks also run as dating regressions. Pilot coverage reports methodological findings separately from date-bearing claims, so zero new dates does not erase the review's contribution.

## Uralic “hundred”: conditional contact case

The Permic forms do not, by themselves, decide between an earlier loan with regular development and separate borrowing. The [review packet](../../../data/workspaces/dating/_research/packets/uralic-hundred-2026-09-19/packet.json) retains eleven findings and a [structured case](../../../data/workspaces/dating/_research/packets/uralic-hundred-2026-09-19/case.json). Neither account receives a calendar date or a global registry relation.

### Sources and scope

The [Holopainen loan appendix](https://doi.org/10.5281/zenodo.6035900), column DW, labels the donor *ćata- under the literal row label “PII or ambiguous (PII ~PI)”. The workbook’s Iranian examples (CB4, EH4, EW4) support reading PI as Proto-Iranian, not Proto-Indic; the label does not select one donor stage for this loan. It gives recipient *ći̬ta under “PU, PFU,” evaluates the overall etymology positively, but marks Komi and Udmurt with question marks. DW12 offers parallel borrowing as a possibility and cites a regular sound-law alternative. These are different levels of judgment; an overall positive label must not erase branch uncertainty. Blank Samoyedic cells have not been converted into demonstrated absence.

The journal supplies [Zhivlov’s original PDF](https://www.jolr.ru/files/%28151%29jlr2014-12%28113-148%29.pdf). Appendix 2 (printed pp.124–125) gives a conditioned Permic vowel reflex, including in the environment preceding intervocalic *t. Appendix 3 (p.132) includes “hundred” in the relevant reconstruction class and lists Komi śo and Udmurt śu. The table, note 15 and lexical entry were visually inspected. This establishes what the proposed account says, not the independent adequacy of its correspondence set.

Two source-handling issues matter:

- The journal labels volume 12 as 2014, while the [publisher platform](https://doi.org/10.31826/jlr-2015-120109) labels the article 2015. Both dates survive in the source record; this is one study, not two confirmations.
- The PDF’s legacy fonts yield control characters or wrong glyphs in extracted phonetic strings. Original extraction bytes and exact spans are preserved, with visual review locators. The more differentiated reconstruction in Zhivlov has not been silently replaced with the appendix’s notation. Canonical machine-readable phonetic transcription remains pending.

The appendix derives from Holopainen’s dissertation and explicitly uses Zhivlov for this comment. The research graph records that scoped dependency. The dissertation and underlying UEW entry have not been independently inspected in this pass.

### What follows under each account

| Account | Conditional consequence | What remains open |
|---|---|---|
| Earlier borrowing, regular Permic development | If the borrowed form participated in this change, entry into the relevant recipient lineage precedes the change. This is an editorial logical inference, explicitly conditioned on participation. | The rule alone does not prove one common borrowing, a Proto-Uralic crown, recipient unity, locality or calendar age. Later substitution, analogy and diffusion require separate assessment. |
| Separate acquisitions | The case permits distinct Permic-related and other recipient loans. | The reviewed material supplies no order between them and no justified position relative to the vowel change. Do not manufacture the reverse of the other account’s edge. |

“Before *t” describes phonetic environment. It is not itself a temporal statement. These accounts are neither demonstrated exhaustive alternatives nor necessarily mutually exclusive across all recipient branches. Regular sound change and multiple contact events could coexist.

### Implementation and remaining work

`harness.contact_cases` validates source-finding references, scenario membership and cycles separately for each conditional graph. `harness.packets` binds the case hash to its source packet and prohibits a contact-case packet from simultaneously exporting unconditional claims or relations. This is a small editorial representation, not a new historical inference engine or downstream etymology solver.

Publication coverage inventories every reviewed packet, including sources outside the minimum pilot bibliography. Packet/case changes therefore participate in the generation's input digest. The root `pyproject.toml` and lockfile are also hashed; no removed package-specific environment is assumed.

Run from the repository root:

```sh
.venv/bin/python -m harness.packets data/workspaces/dating/_research/packets/uralic-hundred-2026-09-19/packet.json --require-imported
```

Open: review the correspondence set against counterexamples, verify source-specific transcriptions and donor adaptations, and have another reviewer assess the competing histories. The [annotation protocol](../docs/review-protocol.md) keeps those judgments separate from source fidelity. This case is exposed development material; it supplies neither specialist consensus nor held-out accuracy.
