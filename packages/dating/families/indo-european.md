# Indo-European

**Summary.** 23 source findings across Chang et al. 2015, Heggarty et al. 2023 and Kassian & Starostin 2025. Both Chang and Heggarty state a 2000 CE epoch for BP; Heggarty's root is ≈8120 BP (95% HPD 6740–9610). Chang's A1–A3 and the sensitivity analyses are distinct scenarios, not one “date of PIE”. No new claims imported; the deposited Heggarty summary is reproduced, but existing claims have not been renormalized to the verified epoch.

Packet: `data/workspaces/dating/_research/packets/indo-european-2026-09-20`.

**Shown in wety (2026-10-04).** Under the crown-age rule (dating rules 2), 14 proto
codes end at a clade age from Heggarty et al. 2023's main analysis M3 (deposited MCC
tree, reproduced by hash; epoch 2000 CE as the article states, now set in the
`heggarty2023_main_m3` manifest, which makes its 160 clade claims supported). Each
decision's `proto_mapping` says why the sampled taxa cover the branch; 95% HPD shown:

| Code | Clade (sampled) | End, 95% HPD |
|---|---|---|
| `ine-pro` | root, incl. Hittite, Luvian, Tocharian | 7614–4736 BCE (median 8120 BP) |
| `iir-pro` | Indo-Aryan + Iranian + Nuristani | 4797–2536 BCE |
| `ine-ana-pro` | Hittite + Luvian | 3621–1858 BCE |
| `ine-bsl-pro` | Baltic + Slavic | 3035–532 BCE |
| `ira-pro` | Avestan, West and East Iranian | 2895–1465 BCE |
| `itc-pro` | Latin/Romance + Oscan + Umbrian | 2287–772 BCE |
| `ine-toc-pro` | Tocharian A + B | 316 BCE – 505 CE |
| `sqj-pro` | Gheg, Standard (Tosk), Arbëresh | 118–1532 CE |
| `bat-pro` | Old Prussian + East Baltic | 1485 BCE – 474 CE |
| `roa-pro` | Romance without Latin (Sardinian, Romanian, Dalmatian, Italo-Western) | 248 BCE – 527 CE |
| `gmw-pro` | Old English, Old Frisian, Old Saxon, OHG and descendants | 35 BCE – 528 CE |
| `gmq-pro` | West and East Norse (Old Gutnish unsampled) | 474–950 CE |
| `gem-pro` | incl. Gothic (alternative; Koch 2020 shown) | 866 BCE – 69 CE |
| `sla-pro` | East, West, South incl. OCS (alternative; Fortson shown) | 163–778 CE |

Not adopted: Celtic (Celtiberian, probably the first split, is unsampled), Indo-Aryan
(M3 nests Nuristani in it), Armenian (the crown is the modern dialect split), Greek
(the crown, 1609–1218 BCE, is bounded by the Mycenaean tip, so it would end
Proto-Greek after Mycenaean is attested). A proto-language is any code Wiktionary
names "Proto-…", whatever its `kind`: Proto-Norse is filed as attested, Proto-Baltic and
Proto-Romance as varieties. All clades
have support 1.0, which reflects M3's constraints. These are conditional on M3. For
`ine-pro` the display also lists two reviewed alternatives, Heggarty et al.'s
sensitivity runs on Chang et al. 2015's broad dataset (manifests
`heggarty2023_sa8a_chang_broad`, `heggarty2023_sa8b_chang_broad_ancestry`; medians
reproduce 8629 and 4997 BP): without ancestry constraints until 7814–5301 BCE, with
Chang's eight ancestry constraints until 3817–2289 BCE, the steppe-compatible
scenario. Chang et al.'s own posterior trees are not available in reproducible form
(D-PLACE's copy has no HPDs or scenario label).
Bouckaert et al. 2012's published root (7,116–10,410 BP, no centre given) is recorded as
a candidate: its Phlorest deposit holds the 2013 corrected trees, which do not reproduce
it (TREE1 7066–10342), so it is not shown.

## Indo-European: source and scenario packet

The packet (`data/workspaces/dating/_research/packets/indo-european-2026-09-20/packet.json`) records 23 source findings across Chang, Heggarty, the deposited results guide, and Kassian & Starostin. It advances the pilot comparison without importing a new date or selecting a historical default. This is development annotation with adversarial self-audit; the completed separate-session Uralic review does not count as a second review of this packet.

### What was recovered and verified

| Source | Evidence inspected | Limits |
|---|---|---|
| Chang et al. 2015 | Retained author-hosted 51-page article, including in-article appendices; source-native epoch, stages, model/sample scenarios and advergence correction | Online analysis supplement and original chains not reviewed here |
| Heggarty et al. 2023 | Journal-format PDF with summary, twelve article pages and endsheet; title/DOI/page continuity and selected rendered pages checked | Third-party mirror, marked downloaded August 2023; publisher-byte equivalence unverified. Publisher/MPG access failed. Main supplementary methods PDF still missing |
| Heggarty deposited guide | Twelve-page guide; retained bytes match Zenodo's published MD5 | Guide is not the methods supplement. Described full posterior trees and chain histories are not all in the retained Light subset |
| Kassian & Starostin 2025 | Publisher text captured through the web tool: all 655 numbered lines across overlapping windows, with no missing indices | Original HTTP/PDF downloads returned challenge pages. Retained raw object is the tool response, not original HTML. Figures, supplementary trees and individual lexical objections remain incompletely reviewed |

Both [Chang](https://linguistics.berkeley.edu/~garrett/ChangEtAl-2015.pdf) and [Heggarty](https://doi.org/10.1126/science.abg0818) explicitly use **2000 CE** for BP. Heggarty reports a median of about 8120 BP and 95% HPD 6740–9610 BP, rounded to decades. This establishes a source convention and conditional result, not a consensus language age. Existing deposited manifests and claim identities have not been silently renormalized; a versioned import must explicitly reconcile those objects with the new source review.

### The comparison that matters

| Scenario | Important conditions | Interpretation |
|---|---|---|
| Chang A1/A2/A3 | Different narrow/medium/broad IELex-based subsets; ancestry constraints; generalized skyline coalescent; RSC traits; branch- and trait-rate variation | Distinct fitted scenarios, not interchangeable instances of “Chang's date.” The article acknowledges nonrandom missingness and modeling effects. |
| Chang corrected root | Per-tree augmentation using discrepancies between selected historical and fitted subgroup dates | Separate from the ordinary root posterior. Its extrapolation of advergence effects needs assessment; it is not a universal proto-language interval. |
| Heggarty M3 | IE-CoR, 161 varieties and 170 concepts; ancestry enabled; binary covarion and eight rate bins | A different data/model scenario. Documentary samples can be parallel registers or lineages rather than stipulated direct ancestors. |
| Heggarty SA7 | Alternative ancestry constraints on IE-CoR | Authors report relatively limited root-age shifts. This does not prove those constraints correct or all alternative data codings harmless. |
| Heggarty SA8a/SA8b | M3 on Chang's broad IELex subset, with 35 clade constraints; without/with eight ancestry constraints | Reported medians 8629 versus 4997 BP. A useful paired sensitivity comparison, but not an exact reproduction of Chang A1 or a decomposition of every difference between the papers. |

The [deposited guide](https://zenodo.org/records/8147476) explicitly identifies the SA8 dataset reuse. The research graph now records this dependence, the separate Chang subsets and the sensitivity status of SA8. A new DOI, deposited tree or second reviewer is not an independent historical confirmation.

Chang distinguishes PIE from a later non-Anatolian PNIE stage. Heggarty reports weak support for the deepest groupings, including only 25.9% for the Anatolian–Tocharian joint clade. Neither a displayed bifurcation nor a catalog family label settles equivalence to a named reconstructed stage.

### Disagreement and source inconsistency

The [2025 critique](https://www.nature.com/articles/s41599-025-04986-7) examines five of 161 wordlists, roughly the hundred concepts shared with its comparison list, and offers both positive assessments and objections. Root cognacy, derivational drift, borrowing treatment and particular lexical identifications warrant source-by-source assessment. The critique is recorded as a relation to the analysis, not as another independently estimated date.

Its preference for a majority-rule consensus summary does not make unresolved posterior groupings evidence of a historical hard polytomy. Weak topology support is important uncertainty; it is not, by itself, proof that every dated posterior is invalid. No relevant reply was identified in the bounded search; this is not evidence that none exists.

The recovered Heggarty PDF contains an additional numerical conflict: SA7a is described as 331 years younger than 8120 BP, ending at 7889 BP. Those endpoints differ by **231** years; 331 years would give 7789 BP. Keep the reported values and discrepancy separate. The underlying SA7a output and any corrected publication version must decide the result; this packet does not invent a replacement number.

## IE sensitivity deposit check

20 September 2026; development self-audit. The complete 161,771,042-byte Light ZIP from [Zenodo 8147476](https://zenodo.org/records/8147476) is retained through the shared acquirer. SHA-256 is `ec6cdfc18b9e3ae96599e71ce2bd81588e397d6921a2db04ff6c2c404a9d3482`; MD5 matches the deposit's `d8b8ba1fdca572bcd40fde8465e4d908`. Its original bytes, selected XML/log/MCC members, acquisition pointer, audit and provenance are retained. No production claims or defaults are added.

Run `.venv/bin/python -m harness.ie_sensitivity_checks` from the repository root. It validates the archive hash before reading named members. It retains native heights and an explicit factor-1000 comparison with the published year-BP summaries; it does not assign an epoch or authorize production normalization. All 27,003 rows of each author-combined log are used. The guide §§4.2/5.4 describes combination, three independent initial chains and their burn-in; applying another generic 10% to the combined logs would be unjustified.

| Scenario | Taxa in XML alignment | Explicit ancestry constraints | Combined median native height | Comparison in years BP |
| --- | ---: | ---: | ---: | ---: |
| SA7a Latin only | 161 | 1 | 7.788517082879079 | 7,788.517 |
| SA8a Chang broad, ancestry enabled | 94 | 0 | 8.629179873141495 | 8,629.180 |
| SA8b Chang broad, ancestry constrained | 94 | 8 | 4.996552741145007 | 4,996.553 |

SA7a rounds to 7,789: this agrees with the article's stated 331-year reduction from 8,120 and 4.08%, but conflicts with its printed endpoint 7,889. Preserve both the article assertion and deposited result; do not silently correct a quote. This resolves what the deposited log reports, not the origin of the publication error.

SA8a and SA8b reproduce the reported rounded medians and have identical hashed taxon/sequence alignments. Their XML explicitly contains zero versus eight ancestry constraints. This supports a controlled comparison within Heggarty's deposited analyses, not equivalence to Chang's complete original inference. Other configuration differences, chain-level mixing, software/runtime and prior sensitivity still need assessment before attributing the entire numerical effect to one isolated intervention.

Both SA8 XMLs contain 43 distributions whose IDs start `monophyly(` (44 non-tip monophyletic MRCA distributions altogether). The guide describes 35 clade constraints. This is a configuration-count reconciliation question: raw element counts are not necessarily counts of distinct nonredundant historical constraints. The audit preserves the 43 identifiers; it does not declare the guide wrong or infer 43 independent constraints.

The Light deposit lacks the full posterior-tree and individual-chain material required for stronger diagnostics. Combined traces alone do not verify independent-chain convergence, and MCC trees do not preserve joint topology uncertainty. The main supplementary methods and Chang's original files remain access/recovery work. Existing packets and claim identities are unchanged.

Harness implications: XML/log files are methodological evidence; do not discard them as “dateless.” Enumerate files blocked by acquisition limits (the Light ZIP exceeds the current automatic cap), route them to explicit recovery and specialized parsers, compare reported versus computed quantities, preserve discrepancies, and record which diagnostics were actually checked. These are concrete requirements for future autonomous family runs.

The editorial file set is preserved as `dating-ie-sensitivity-review/31087201b696c9a6596716449d4a6a07ce373cabdbc8ec63ec8d6c0f4696f3d5`. This adoption records a reproducible review artifact, not scientific approval.
