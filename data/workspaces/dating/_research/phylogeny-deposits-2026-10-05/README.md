# Deposited dated phylogenies, 2026-10-05

Find, retain and verify deposited summary trees, so that proto-languages can end at
reproduced crown ages ([brief](../briefs/phylogeny-deposits-2026-10-05.md)).

## Run

- Profile `codex-astra` (gpt-6-astra, high effort, Codex CLI 0.160.0). 1717 s, status
  partial (one repair pass; `check` failed on unstated epochs, which the brief told it
  not to invent). 332 tool actions. Usage 7,207,877 input tokens (6,762,624 cached),
  38,847 output. Two requests filed: Gray et al. 2009 and Bouckaert et al. 2018 (the
  articles, for their main text).
- Retained trees: Kolipakam 2018 (Figshare), Savelyev & Robbeets 2020 (Zenodo),
  Bouckaert 2018 (publisher supplement), and Phlorest redeposits of Kitchen 2009,
  Grollemund 2015 and Lee & Hasegawa 2011. Koile 2022's Trees.zip and Gray 2009's
  GZIP archives are retained but not extracted.

## Review (Claude Opus 5.5)

The nine text assertions are imported as candidates (provenance); the dates come from
six analysis manifests whose trees are hashed and parsed, with roots compared against
the publications (all reproduce, Kitchen approximately). Corrections: codes moved from
`subject.code`; times rewritten in the registry's distribution form; epochs set as on
the manifests. Turkic states AD 2000. Kolipakam (254 BCE = 2250 years ago) and Lee &
Hasegawa (407 YBP = 1603) imply 2000 and 2010 through their own calibrations. The rest
take BP/YBP as AD 1950 by convention, labeled. Gray's abstract figure stays unresolved.

Harness change: summary nodes that TreeAnnotator left without heights (clades below its
posterior limit) now yield no claim instead of rejecting the tree; an unannotated root
still rejects it.

Adopted crown ends (decisions with `proto_mapping`): `dra-pro`, `dra-sdo-pro`,
`dra-sdt-pro`, `dra-nor-pro`, `trk-pro`, `trk-cmn-pro`, `sem-pro`, `sem-wes-pro`,
`jpx-pro`, `jpx-ryu-pro`, `bnt-pro`, `aus-pam-pro`. Readings and caveats per family are
in `packages/dating/families/`. `map-pro`'s Blust default was re-adopted after Gray's
candidate touched it.

## Follow-up: archives

Koile 2022's Trees.zip: the Part 2 summary tree (manifest `koile2022_part2_geographic`)
reproduces the article's root HPD and supplies a reviewed alternative for `bnt-pro`; the
Part 1 tree does not reproduce the article and is not used. Gray 2009's redeposited
MCC (`a400-m1pcv-time.mcct.trees`) carries no node annotations, only branch lengths,
so it gives no HPD; only the posterior sample could, and that needs tip ages and
diagnostics. `map-pro` keeps Blust 2013.
