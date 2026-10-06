# Deposited dated phylogenies, second round, 2026-10-06

Further deposited summary trees for proto-language crown ages
([brief](../briefs/phylogeny-deposits-2026-10-06.md)).

## Run

- Profile `codex-astra` (gpt-6-astra, high effort, Codex CLI 0.160.0). 2126 s; the
  harness records the run as failed because no terminal usage event arrived, but all
  four deliverables were written; usage unknown, not zero. 474 tool actions. `check`:
  9 assertions; every error is the unstated BP epoch, which the brief said to leave
  out. Requests: the supplements of Bouckaert et al. 2012 and Robbeets et al. 2021.
- Retained trees: Greenhill 2023 (author MCC), Tao 2023 (journal ZIP; MCC extracted
  here), Gerardi 2023 (OSF MCC), Robbeets 2021 (author GitHub), Bouckaert 2012
  (Phlorest, 2013 corrected trees).

## Review (Claude Opus 5.5)

The nine text assertions are imported as candidates with labeled epochs (1950
convention; Robbeets' README sets AD 2000). Repeated quotes in the Robbeets tree cite
the first occurrence. Three manifests: `tao2023_kd_covarion_relaxed` and
`gerardi2023_full_relaxed` reproduce their articles; `greenhill2023_ua_covarion_relaxed`
approximates its article, whose figure is the MRCA time over all posterior trees.
The Kra-Dai calibration workbook (Supplementary Data 2) was decoded to see which nodes
carry priors.

Adopted crown ends: `qfa-tak-pro`, `tai-pro`, `tai-swe-pro`, `azc-pro`,
`tup-gua-pro` (family pages `kra-dai.md`, `uto-aztecan.md`, `tupian.md`). Not adopted:
calibrated nodes (Proto-Kra, Nahuan, Numic; Robbeets' Tungusic), Robbeets' Mongolic
and Koreanic (contested macro-family analysis, tree-to-publication link unresolved),
Bouckaert 2012 (deposit holds only the corrected trees). `ine-pro`'s Heggarty default
was re-adopted after the Bouckaert candidate touched it.
