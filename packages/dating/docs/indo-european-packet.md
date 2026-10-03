# Indo-European: source and scenario packet

The [packet](../../../data/workspaces/dating/_research/packets/indo-european-2026-09-20/packet.json) records 23 source findings across Chang, Heggarty, the deposited results guide, and Kassian & Starostin. It advances the pilot comparison without importing a new date or selecting a historical default. This is development annotation with adversarial self-audit; the completed separate-session Uralic review does not count as a second review of this packet.

## What was recovered and verified

| Source | Evidence inspected | Limits |
|---|---|---|
| Chang et al. 2015 | Retained author-hosted 51-page article, including in-article appendices; source-native epoch, stages, model/sample scenarios and advergence correction | Online analysis supplement and original chains not reviewed here |
| Heggarty et al. 2023 | Journal-format PDF with summary, twelve article pages and endsheet; title/DOI/page continuity and selected rendered pages checked | Third-party mirror, marked downloaded August 2023; publisher-byte equivalence unverified. Publisher/MPG access failed. Main supplementary methods PDF still missing |
| Heggarty deposited guide | Twelve-page guide; retained bytes match Zenodo's published MD5 | Guide is not the methods supplement. Described full posterior trees and chain histories are not all in the retained Light subset |
| Kassian & Starostin 2025 | Publisher text captured through the web tool: all 655 numbered lines across overlapping windows, with no missing indices | Original HTTP/PDF downloads returned challenge pages. Retained raw object is the tool response, not original HTML. Figures, supplementary trees and individual lexical objections remain incompletely reviewed |

Both [Chang](https://linguistics.berkeley.edu/~garrett/ChangEtAl-2015.pdf) and [Heggarty](https://doi.org/10.1126/science.abg0818) explicitly use **2000 CE** for BP. Heggarty reports a median of about 8120 BP and 95% HPD 6740–9610 BP, rounded to decades. This establishes a source convention and conditional result, not a consensus language age. Existing deposited manifests and claim identities have not been silently renormalized; a versioned import must explicitly reconcile those objects with the new source review.

## The comparison that matters

| Scenario | Important conditions | Interpretation |
|---|---|---|
| Chang A1/A2/A3 | Different narrow/medium/broad IELex-based subsets; ancestry constraints; generalized skyline coalescent; RSC traits; branch- and trait-rate variation | Distinct fitted scenarios, not interchangeable instances of “Chang's date.” The article acknowledges nonrandom missingness and modeling effects. |
| Chang corrected root | Per-tree augmentation using discrepancies between selected historical and fitted subgroup dates | Separate from the ordinary root posterior. Its extrapolation of advergence effects needs assessment; it is not a universal proto-language interval. |
| Heggarty M3 | IE-CoR, 161 varieties and 170 concepts; ancestry enabled; binary covarion and eight rate bins | A different data/model scenario. Documentary samples can be parallel registers or lineages rather than stipulated direct ancestors. |
| Heggarty SA7 | Alternative ancestry constraints on IE-CoR | Authors report relatively limited root-age shifts. This does not prove those constraints correct or all alternative data codings harmless. |
| Heggarty SA8a/SA8b | M3 on Chang's broad IELex subset, with 35 clade constraints; without/with eight ancestry constraints | Reported medians 8629 versus 4997 BP. A useful paired sensitivity comparison, but not an exact reproduction of Chang A1 or a decomposition of every difference between the papers. |

The [deposited guide](https://zenodo.org/records/8147476) explicitly identifies the SA8 dataset reuse. The research graph now records this dependence, the separate Chang subsets and the sensitivity status of SA8. A new DOI, deposited tree or second reviewer is not an independent historical confirmation.

Chang distinguishes PIE from a later non-Anatolian PNIE stage. Heggarty reports weak support for the deepest groupings, including only 25.9% for the Anatolian–Tocharian joint clade. Neither a displayed bifurcation nor a catalog family label settles equivalence to a named reconstructed stage.

## Disagreement and source inconsistency

The [2025 critique](https://www.nature.com/articles/s41599-025-04986-7) examines five of 161 wordlists, roughly the hundred concepts shared with its comparison list, and offers both positive assessments and objections. Root cognacy, derivational drift, borrowing treatment and particular lexical identifications warrant source-by-source assessment. The critique is recorded as a relation to the analysis, not as another independently estimated date.

Its preference for a majority-rule consensus summary does not make unresolved posterior groupings evidence of a historical hard polytomy. Weak topology support is important uncertainty; it is not, by itself, proof that every dated posterior is invalid. No relevant reply was identified in the bounded search; this is not evidence that none exists.

The recovered Heggarty PDF contains an additional numerical conflict: SA7a is described as 331 years younger than 8120 BP, ending at 7889 BP. Those endpoints differ by **231** years; 331 years would give 7789 BP. Keep the reported values and discrepancy separate. The underlying SA7a output and any corrected publication version must decide the result; this packet does not invent a replacement number.

## Next work

Recover the main Heggarty methods supplement and Chang analysis files; inspect SA7a and SA8 inputs/results, not just their prose summaries. Pin chains, resumed-run boundaries, burn-ins and diagnostics before reproducing sensitivity results. The guide explicitly describes combined logs and different burn-ins; a pooled log is insufficient to establish between-chain convergence.

Adjudicate a small set of the critique's lexical examples against original dictionaries and source coding, preserving alternative adaptations and derivations. Reconcile the newly verified epochs with versioned analysis manifests before creating calendar-normalized claims. Continue the Sino-Tibetan packet alongside these source-specific gaps; no human-review dependency is introduced.
