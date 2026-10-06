# Brief: deposited dated phylogenies ({{FAMILY}})

One bounded run that finds and retains the deposited dated trees of published
language phylogenies, so the integrator can build analysis manifests (crown ages
of proto-languages). Proposal only; tools and limits of the single frontier researcher.

```text
Find and retain the deposited dated phylogenies listed below ({{FAMILY}}). Work as one
researcher, without subagents or other model calls. Do not redesign tools.

For each study:
1. Find the published deposit (journal supplementary files, Zenodo, Dryad, OSF, GitHub,
   the authors' lab pages) holding a summary tree with node-age uncertainty (an MCC or
   annotated consensus tree with height_95%_HPD or median/mean heights), or posterior
   tree samples. D-PLACE copies lack uncertainty: use them only to identify the study.
   Acquire the deposit (a ZIP is fine) and say which member is the summary tree.
2. Quote, from the article or its supplement, (a) the root age as published (median or
   mean and 95% HPD) and (b) the epoch of "BP"/"years ago" if stated (e.g. "present taken
   as 1950"), and (c) the unit of tree heights if stated. Never quote from memory.
3. Record what each study samples (number of languages) and its main analysis name.

Studies: Gray, Drummond & Greenhill 2009 (Austronesian, Science); Grollemund et al. 2015
(Bantu, PNAS); Koile et al. 2022 (Bantu, PNAS); Kolipakam et al. 2018 (Dravidian, Royal
Society Open Science); Bouckaert, Bowern & Atkinson 2018 (Pama-Nyungan, Nature Ecology &
Evolution); Kitchen et al. 2009 (Semitic, Proc. R. Soc. B); Savelyev & Robbeets 2020
(Turkic, Journal of Language Evolution); Lee & Hasegawa 2011 (Japonic, Proc. R. Soc. B).

Deliverables (write action; these four names only):
1. assertions.jsonl: for each study, one assertion of its published root age (event
   crown_divergence, role posterior, method phylogenetic, native_time with unit BP or
   ka_BP and present if stated, quote, locator, subject {code: the family's proto code,
   source_label, mapping "unresolved", mapping_basis}) and, where stated, one assertion
   of the epoch (event unspecified, role observation, method scholarly_description,
   native_time {"form": "unknown"}, quote). proposed_status candidate throughout.
2. report.md: a table per study: deposit URL, acquired source_id, summary-tree member
   path, statistic and unit of heights, epoch statement (or none), published root age,
   sample size, access problems.
3. sources.json and 4. self-check.json as usual.
Call check after saving, fix problems once, and finish.
```
