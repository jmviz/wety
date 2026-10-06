# Brief: deposited dated phylogenies, second round ({{FAMILY}})

Second bounded run after `phylogeny-deposits-2026-10-05`: find and retain deposited
dated trees for further proto-language crown ages. Changes from the first brief: the
assertion format matches the registry (top-level `code`, distribution fields), and a
bounded search for families with no known deposit.

```text
Find and retain the deposited dated phylogenies below ({{FAMILY}}). Work as one
researcher, without subagents or other model calls. Do not redesign tools.

Part A. For each study:
1. Find the published deposit (journal supplementary files, Zenodo, Dryad, OSF, GitHub
   including github.com/phlorest raw/ folders, the authors' lab pages) holding a summary
   tree with node-age uncertainty (an MCC or annotated consensus tree with
   height_95%_HPD and median or mean heights). D-PLACE/CLDF exports lack uncertainty:
   use them only to identify the study. Acquire the summary tree (a ZIP is fine; say
   which member). Check that it has height annotations; say so if it does not.
2. Quote, from the article or its supplement, (a) the root age as published (median or
   mean and 95% HPD), (b) the epoch of "BP"/"years ago" if stated, or any calendar
   equation in the calibrations (e.g. "407 YBP, the year ... 1603"), and (c) the unit
   of tree heights if stated. Never quote from memory.
3. Record the sample (number of languages, which subgroups) and the main analysis name.

Studies: Greenhill, Haynie et al. 2023 (Uto-Aztecan); Robbeets et al. 2021 (Nature,
Transeurasian: we need its Mongolic, Tungusic and Koreanic subfamily crowns, not the
contested Transeurasian root); Bouckaert et al. 2012 (Indo-European, Science).

Part B. Bounded search (at most 15 minutes in total): does a published Bayesian dated
phylogeny with a deposited summary tree exist for Kra-Dai or Tai, Austroasiatic,
Tupí-Guaraní or Tupían, Mayan, Eskimo-Aleut, Athabaskan? Acquire any you find and treat
it as in Part A; list the rest as not found, with the queries tried.

Deliverables (write action; these four names only):
1. assertions.jsonl: for each study, one assertion of its published root age and, for
   Robbeets 2021, one per subfamily crown it reports: id, code (the Wiktionary proto
   code: azc-pro, xgn-pro, tuw-pro, qfa-kor-pro, ine-pro, tai-pro, aav-pro, tup-gua-pro,
   myn-pro, esx-pro, ath-pro), source_id, quote, locator, event crown_divergence, role
   posterior, method phylogenetic, native_time {"form": "distribution", "unit": "BP" or
   "ka_BP", "median" or "mean": value, "lower", "upper", "interval_type": "hpd",
   "probability": 0.95, "present": only if the source states it}, subject {code,
   source_label, mapping "unresolved", mapping_basis}, proposed_status candidate.
2. report.md: a table per study: deposit URL, acquired source_id, summary-tree member
   path, statistic and unit of heights, epoch statement (or none), published root age,
   sample size, access problems; then Part B's findings.
3. sources.json and 4. self-check.json as usual.
Call check after saving, fix problems once, and finish. An unstated epoch is not a
problem to fix: leave present out.
```
