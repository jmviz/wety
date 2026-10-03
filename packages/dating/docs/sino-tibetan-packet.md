# Sino-Tibetan source and scenario packet

The [packet](../../../data/workspaces/dating/_research/packets/sino-tibetan-2026-09-20/packet.json) records 31 findings across nine source files for Sagart 2019, Zhang 2019 and Zhang 2020. It recovers supplementary methods, a BEAST configuration and a previously omitted posterior file. This is same-session development review with a numerical self-audit, not held-out evaluation or historical adjudication. It imports no chronological claims or defaults.

## Sources and access

| Study | Evidence inspected | Remaining limits |
|---|---|---|
| [Sagart et al. 2019](https://doi.org/10.1073/pnas.1817972116) | Retained complete main JATS XML; newly recovered 38-page supplement; existing dated summary remains available | Selected passages reviewed; calendar epoch, original chain diagnostics and historical calibration grounds remain unverified |
| [Zhang et al. 2019](https://doi.org/10.1038/s41586-019-1153-z) | Article abstract with explicit AD 1950 epoch; complete 19-page supplement; publisher ZIP containing lexical Nexus, XML and MCC tree | Subscription main body remains an access gap. Exact correspondence between deposited XML and all reported runs is unverified |
| [Zhang et al. 2020](https://doi.org/10.1038/s41598-020-77404-4) | Published-version PDF from Reading repository; publisher DOCX supplement; [2021 correction](https://doi.org/10.1038/s41598-021-85112-w) and its tree file via Europe PMC | Input matrix/XML, alternative-run outputs and independent-chain traces missing; calendar offsets and specification discrepancies unresolved |

Acquisition receipts retain original response bytes and hashes. Client-challenge pages are recorded as failed scientific access. New source versions leave old source records and claim identities unchanged. The Word supplement was read structurally and rendered: overlapping tables in the render are disclosed; the original cells and document are preserved without editorial repair.

## Compare complete scenarios

| Scenario | Sample and coding | Native root result and conditioning |
|---|---|---|
| Sagart relaxed covarion | 50 varieties, 180 concepts; expert cognate coding, loan exclusions and coverage filters | **Mean 7184 BP**, 95% HPD 5093–9568; fossilized birth-death prior. Sinitic is the basal group in only 33% of posterior trees |
| Zhang 2019 relaxed covarion | 109 varieties, 949 root-meaning traits from STEDT; missingness and sampling filters | **Average 5871 BP**, 95% HPD 4183–7817; AD 1950 epoch. Abstract rounds these numbers to about 5900 and 4200–7800 |
| Zhang 2020 relaxed covarion | 131 varieties, 110 concepts, 1726 cognate sets from Tower of Babel/Peiros–Starostin | **Mean 7983 BP**, 95% HPD 4778–11285; fossilized birth-death prior and final Sinitic outgroup constraint. Calendar epoch unresolved |

These are sampled clade results under different data and priors. None is a language's start or an agreed proto-language interval. Sagart and Zhang 2019 both use STEDT material, with different curation; bibliographic separation does not establish independent lexical evidence. The dependency graph now records datasets, reported calibration sets, alternative analyses and the correction's relation to the original result.

Sagart constrains the modern Chinese dialect subgroup and its MRCA to 2000–2200 BP. That is distinct from the Old Chinese tip prior of 2300–2800 BP, which targets the classical texts sampled. Neither is an earliest-attestation or extinction observation. Its contact-adequacy test uses only 15 leaves and a trait holdout; the reported preference for a network is relevant, but does not settle adequacy for every subgroup or contact process.

Zhang 2020's Figure S2 gives means **7878 BP without** and **7983 BP with** the Sinitic outgroup constraint. Its broader sample is the authors' proposed explanation of the older cross-study date, not a controlled demonstration of that cause. Separate sensitivity analyses give 9758 BP with a strict clock and 7972 BP with only 100 concepts, with different intervals. Preserve those scenarios rather than pooling their endpoints.

## Additional issues found

* **Zhang 2019 calibration provenance:** Table 2's Tibetan row pairs a mid-thirteenth-century rationale with a normal mean of 1150 BP. The XML uses that numerical mean too. Karen's standard deviation is 100 in the table but 75 in the deposited XML. These are unresolved source/configuration discrepancies, not authorization to repair the priors by guessing.
* **Geographic result versus interpretation:** Zhang 2019's Brownian geographic model locates the origin near the eastern Tibetan Plateau. The authors argue that extinction and directional migration bias that result southwest. Their northern-origin interpretation therefore is not simply the output of their geographic model.
* **Zhang 2020 calibration semantics:** The CE rows in Table S1 use offsets consistent with 2020; the Old Chinese BCE row approximately uses 2000. The article calls the internal calibration Lolo-Burmese while the table calls it Lolo-Qiangic. Its “extinction” and “last-seen” labels need documentary/entity review. An unbounded uniform written in a table is not enough to reconstruct an implemented prior.
* **Posterior versus description:** The correction supplies 1001 states, including burn-in, rather than an already filtered 900-tree sample. Old Chinese remains fixed at 2550 years in every tree although the supplement describes a uniform 2300–2800 range. This does not identify whether the discrepancy lies in the description, XML, operators or deposited run.
* **Other numerical/reporting discrepancies:** Zhang 2020 Table S4 says an upper root bound of 11.8 ka, whereas the main text, Figure S2 and recovered sample support 11285 years. Table S2 says 64 stepping-stone steps; prose says 100. Table S5's labeled “Bayes Factor” 20.18 equals twice the difference of its log marginal likelihoods, not a raw probability ratio. Preserve the source labels and computational distinctions.

## Reproducible checks

Run from the repository root:

```sh
.venv/bin/python -m harness.sino_tibetan_checks
.venv/bin/python -m harness.packets data/workspaces/dating/_research/packets/sino-tibetan-2026-09-20/packet.json --require-imported
```

The checker binds source bytes by hash, verifies all 1001 state names and 131-taxon sets, checks dated tips and the constrained basal split, and explicitly selects states **after** 5,000,000 iterations. The resulting 900 trees yield mean **7982.55 years** and the shortest interval containing 855 observations, **4778.19–11284.63 years**, reproducing the reported rounded statistics. This calculation preserves joint trees but does not establish between-chain convergence, a calendar epoch or adequate priors. No production posterior manifest is added to bypass those requirements.

The packet pins the numerical report by hash. The regression suite covers the source-file mismatch checks, burn-in selection, interval convention, fixed tips, XML discrepancy and abstention. Publication reproduction and extraction accuracy remain distinct.

## Next work

Recover Zhang 2020's actual XML, input matrix, alternate runs and diagnostic traces; resolve the fixed-tip and calendar-offset issues before considering a production import. Inspect original historical sources behind the calibrations, with exact taxon sets. Complete Zhang 2019's main-text access and reconcile table/XML variants. Broader sensitivity work and source-level corrections may proceed with explicit abstention; neither a human reviewer nor an adopted historical date is required to continue.
