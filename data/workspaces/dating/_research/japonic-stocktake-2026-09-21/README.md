# Japonic: combined inventory and targeted corrections

This is an editorial companion to the two completed single-researcher runs, not a
third autonomous discovery run or an approved import. Start with this summary when
using their results. The original assertions/reports/self-checks remain preserved;
`review.json` binds them by hash and identifies the corrected fields. The two
`*-targeted-revisions.jsonl` files contain only changed proposals, with new IDs and
links to their original records. Unchanged fields are not thereby approved.

## Combined source inventory

| Work | Retained run / source ID | What it contributes |
| --- | --- | --- |
| Lee & Hasegawa 2011, 10.1098/rspb.2011.0518 | Both; Astra `51bacff9fdb05e3e`, Claude `6fb4a1d54f6f6a6d` | Two sampled-root analyses, documentary-tip and Tokyo–Kyoto priors, topology/model conditions |
| Pellard 2015, 10.1515/9781614511151.13 | Astra `f297953a983f4335` | Comparative separation argument, named Sino-Japanese loan layer, contact and migration synthesis, critique of archaeological correlation |
| Jarosz et al. 2022, 10.1017/ehs.2022.1 | Both; Astra `421bcfc1f1c594e2`, Claude `d0d868c828b71df1` | Kyushu–Ryukyuan scenario, dispersal synthesis, secondary phylogenetic reports and cereal archaeology |
| Huisman, McLean & Wu 2025, 10.1093/jole/lzaf002 | Astra `f3c8ff27f1c38fa3` | Broader sampled-model alternatives, contemporary Japanese versus Old Japanese scope, unresolved numeric/configuration details |
| Robbeets et al. 2021, 10.1038/s41586-021-04108-8 | Claude `20258589f97ad607` | Proposed macro-family results, Japonic calibration prior, conditional aggregate lexical-stratum interpretation |
| de Boer et al. 2020 | Claude `dd8211723c2b5c91` | Archaeology/dialect review, feature chronology and conditional migration scenarios |

Two copies of the same study are not independent evidence. This inventory describes
acquired/read portions, not complete source packets or exhaustive literature coverage.
See the original `sources.json` files for body, figure and supplement gaps.

## Corrected synthesis

Lee–Hasegawa's main analysis reports median **2182 YBP**, mean **2398**, and **95%
HPD 1239–4190** for its sampled root. Its nine-language analysis is a separate
same-study scenario. The numerical epoch is not explicitly established in the read
text. The 407 YBP historical calibration suggests an epoch question, not a documented
publication-year origin. Its mean 407 and SD 135.2 imply approximate normal 95%
endpoints 142–672, whereas the source reports 142–549. This arithmetic flags a
conflict; it does not authorize a replacement prior or corrected root chronology.

The sampled root separates mainland Japanese and Ryukyuan in the reported topology.
That event must not be confused with internal Ryukyuan diversification, whose
primary numeric estimate was not recovered in the Claude packet. Nor does either
event automatically equal island dispersal.

Robbeets' **2100 bp ± 175** is a Japonic prior specification, with dispersion type
and its underlying literature unresolved in the retrieved body. It is not an
independent posterior confirmation of Lee–Hasegawa. This does **not** mean the
calibrated node cannot have a posterior. Proposed macro-family node estimates and
their marginal intervals do not date a named Proto-Japonic stage.

Huisman 2025 supplies additional sampled-model results, so the combined literature
cannot be summarized as having only one Japonic dating study. Its model-specific
outputs remain a gap. The printed **12,000–880 BCE** range of older interval
endpoints is anomalous and retained as reported; it is neither silently changed to
1,200 nor pooled with other scenarios into a universal interval.

Jarosz' **AD 800–1300** is retained here as the discussion's approximate dispersal
window within its historical scenario. Its table's break-up terminology does not
prove equality of dispersal and divergence. Its attributed contact continuation to
at least the ninth century does not establish that every borrowing happened after
that century or that continued contact logically requires later migration.

The de Boer statement places most cited grammatical differences in the **fourteenth
century or later**, not strictly after that century. It dates features, not every
mainland dialect node. Robbeets' collective list of later subsistence vocabulary
does not establish every item in Japonic; item-level and genealogical premises
remain unreviewed without the lexical supplement.

## Concrete etymology handoff

Pellard's Table 1.10 and adjacent argument are already retained in Astra's source
at text offsets **23212–24304**, with page 11 rendered in that original run. They
offer the following three candidates for a later wety alignment:

| Reconstructed recipient item as printed | Meaning | Evidence-dependent operation |
| --- | --- | --- |
| `*bau C` | stick | Identify the corresponding Japanese-mediated borrowing edge and recipient reconstruction |
| `*sjaugwatu C` | first month | Identify the complete compound and its donor stage; avoid applying the window to unrelated component histories |
| `*zjauzu B/C` | skillful | Preserve the reconstruction/tone qualification while aligning the lexical nodes |

The author's argument combines Sino-Japanese entry with Japanese `aũ` versus
Ryukyuan/Miyako `au` and later Japanese monophthongization. The source writes a
borrowing window between the **“8/9th” and “13/15th” centuries**. Keep the slash
expressions unresolved; do not interpret them as discrete choices that exclude
intervening time. The later synthesis says contact until at least the 8/9th and at
most the late thirteenth century. Both formulations remain separate. No exact
calendar bounds, graph node matches or automatically executable constraints are
authorized by this packet. This is a concrete lexical follow-up, not a requirement
to settle the age of Japonic first.

## Reproduce the editorial artifact

From the repository root:

```sh
.venv/bin/python data/workspaces/dating/_research/japonic-stocktake-2026-09-21/build_corrections.py
```

The script reads existing local runs, writes only this packet, verifies exact quote
membership and revised exact-offset locators, and records source contexts/hashes.
It performs no acquisition, model calls or import. `review.json` lists 11 findings
affecting 12 revised records. These are targeted findings, not a denominator for
backend precision. Complete sample mapping, historical truth, supplement coverage
and every background/methodological claim remain unassessed.
