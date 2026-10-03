# Uralic supplement and dependency review

This follow-up to the [first packet](uralic-packet.md) recovers six source files and records ten source-grounded methodological findings. It adds no date-bearing claims or defaults. The earlier packet and its assertions remain intact; this packet resolves some of its open review questions without rewriting their historical state.

## Recovered material

| Source | Result of this pass | Remaining limit |
|---|---|---|
| Grünthal et al., [published article](https://doi.org/10.1075/dia.20038.gru) | Retained 35-page published PDF, pp.490–524, with acquisition receipt. Selected contact-order, stage and epoch passages agree with the earlier manuscript's argument. | This is a selected collation, not a complete textual edition comparison. Published p.515 explicitly defines BP present as 2000; this convention is scoped to the published version. |
| Grünthal et al., [supplements v1.0](https://zenodo.org/records/6345559) | Retained the 47-page supplement, R script and input workbook. The earlier manuscript included only 29 supplement pages and omitted the script. | Complete supplement/coding adjudication remains open. Original R package versions are not pinned. |
| Maurits et al., [publisher supplement](https://academic.oup.com/jole/article/5/1/17/5688948) | Retained the DOCX defining the reversed lognormal and other calibrations. | It supplies no posterior samples, traces or model configuration for re-running the demonstration. |
| Holopainen, [loan appendix](https://zenodo.org/records/6035900) | Retained the workbook underlying the contact argument; reviewed its dependency metadata and the “hundred” example. | Not a substitute for reviewing the dissertation or all etymologies. The deposit supplies no explicit licence; the workbook's “Open Access” notice is not a specified reuse licence. |

Original bytes have SHA-256 hashes and shared acquisition receipts. The three files in Zenodo6345559 also match its published MD5 checksums. Parsed text retains cell coordinates, formula strings and cached values for spreadsheets. Independent comparison with the bundled `openpyxl` reader matched all 157 stored cells in the S4 workbook and 2,348 in the loan appendix. Original workbooks were not edited or recalculated.

## Numerical checks and their scope

Maurits's supplement defines the reversed lognormal as `X = offset − Y`, with lognormal `Y`. Using the first packet's main-article parameters in millennia gives mode **1041.359 YBP** and central equal-tailed 95% endpoints **661.379–1263.037 YBP**, matching the reported rounded values. Probability of positive age is approximately **0.999995**; the supplement explains optional truncation at zero. This resolves the parameterization question left open in the earlier review. Figure 4’s caption instead says a mode near 1250; that conflicts with its parameters and main text, which give 1041. Preserve this source inconsistency explicitly. It does not validate the historical calibration argument or replace the recorded interval label with an HPD.

All ten results in Grünthal's TableS4.2 reproduce at three-decimal reporting precision using the deposited counts and distances and the stated one-sided Mann–Kendall normal approximation with continuity correction. There are eight branches for PU etyma (Samoyedic excluded) and nine for Indo-Iranian loans (included). The checker uses a separate standard-library implementation. It does **not** run the original R environment, reproduce every sensitivity run or calculate Bayesian chain diagnostics.

The R script expects `PU_I-I_2021-data.xlsx`; the deposited filename includes ` copy`. A future R reproduction must record that filename mapping without altering the original artifact. The script changes Saamic distances by adding281km to Finnic distances, while the supplement rounds this to280km; the published main table uses the original great-circle distances.

Nonsignificant negative-trend tests do not establish the absence of isolation by distance, prove rapid dispersal, or identify a homeland. Power, branch dependence, distance proxies and coding sensitivity need separate evaluation. These tests do not supply a calendar date.

## A concrete etymological disagreement

The loan appendix's metadata says it derives from Holopainen2019, with additions in columnsR,AZ,EQ,ER. It is one dependent source, not an independent confirmation. Its “hundred” column (`Indo-Iranian!DW`) records a proposed donor and recipient reconstruction, but marks the Permic reflex cells `DW41` and `DW42` with question marks. The note in `DW12` considers **parallel borrowing** and a **regular sound-law explanation** of Permic vocalism as alternatives.

This is a useful next wety case: identify the specific borrowing(s), the disputed reflexes, and the proposed sound rule, then express only the chronological order that each interpretation permits. The appendix alone cannot settle which analysis is correct or attach an absolute borrowing date. The supplement also explicitly allows gradual borrowing as an alternative to a drastic contact explanation for Samoyedic phonotactics (p.3).

## Still open

Honkola's named AppendixS1 and FigureS1 downloads return HTTP403; the OUP minimal article response does not supply usable download links. The CORE dissertation download returns404, and the official Helda/URN routes return challenge HTML rather than the thesis. These are recorded access failures, not evidence that the documents do not exist. Kuzmina2007 and the donor-stage calendar/archaeological anchors remain unreviewed. No specialist was contacted, and no independent review or held-out evaluation has been performed.

The next deliverable is a source-specific “hundred” case dossier preserving the two competing explanations, alongside a review protocol and reserved evaluation passages. Complete the missing primary-source checks before proposing absolute contact dates. The IE and Sino-Tibetan comparison packets remain the next family-level work.

## Reproduce

From the repository root:

```sh
.venv/bin/python -m harness.packets data/workspaces/dating/_research/packets/uralic-supplements-2026-09-19/packet.json --require-imported
.venv/bin/python -m harness.uralic_checks
uv run --frozen wety-data run dating
```

The [follow-up packet](../../../data/workspaces/dating/_research/packets/uralic-supplements-2026-09-19/packet.json) binds exact source spans and review limitations. Its `numerical-checks.json` records the calculations; `retrieval-attempts.json` records access attempts. The numerical checks also run as dating regressions. Pilot coverage reports methodological findings separately from date-bearing claims, so zero new dates does not erase the review's contribution.

## Delivery checks

All 88 dating tests pass, including the new source, numerical and uncertainty-preservation checks. Root workflow run `403c9f10d90f46d483d8ccd6b3f39e63` completed synthesis, validation and all three publication-reproduction checks successfully. All 3,575 existing assertions and the complete 8,625-code view remain unchanged. No defaults were selected; `spoken` remains null.

The six acquired artifacts pass hash verification. The [immutable follow-up packet](../../../data/artifacts/dating-uralic-supplements/faec7bd25b10ffc89e1fd086c65c557cd815e0bb52b09196ab30f396c9928a99/manifest.json) also passes artifact verification. Its numerical results are development checks, not held-out accuracy measurements.
