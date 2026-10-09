# Published proto-language estimates, round 5, part C, 2026-10-07

[Batch brief](../briefs/proto-estimates-round5-c-2026-10-07.md).

## Run

- Profile `codex-sol` (gpt-6.1-sol, high effort, Codex CLI on ChatGPT subscription).
  2700 s + 600 s; used 2586 s (failed).
  713 tool actions. Usage not recorded.
  `check`: 12 assertions, 1 errors. Requests: California Indian Languages; Subgrouping in a ‘dialect continuum’: A Bayesian phylogenetic analysis of the Mixtecan language family; Somaloid history; Lexicostatistical comparison of Omotic languages; Northeast Asian Linguistic Ecology and the Advent of Rice Agriculture in Korea and Japan; Classifying the Austroasiatic languages: history and state of the art; Reconstruction of Proto-Batak phonology; Language Ecology and Linguistic Diversity on the African Continent; First Farmers: The Origins of Agricultural Societies; A Grammar of the Great Andamanese Language; Kekerabatan Bahasa Tolaki dan Bahasa Mori (Pendekatan Linguistik Historis Komparatif); Archaeology, Language, and the African Past; A Reconstruction of the Proto-Iwaidjan Phoneme System; Kekerabatan Bahasa Batak Mandailing dan Bahasa Batak Angkola (Tapanuli) di Kabupaten Pasaman; Central Sudanic.

## Review (Claude Opus 5.5 (Claude Code))

Quotes exact in the retained texts. 4 of 12 supported; 3 codes adopted.

GPT-6.1 Sol (high). The run finished its deliverables but was marked failed: the worker
broke on a U+2028 inside a Codex event (fixed in fe18b2c); usage was not recorded. It
found nine studies, mostly Indonesian glottochronology papers, for 7 of 68 codes.
Adopted: Proto-Omotic, North/South split at the beginning of the 5th millennium BC
(Blažek 2008); Proto-Batak, the Karo–Toba (Northern/Southern) split in 703 CE
(Rajagukguk & Widayati 2022), with Ginting et al. 2024's 1028 CE as alternative (their
paper prints both 995 and 955 years); Proto-Bungku-Tolaki, the Tulambatu–Tolaki
(Eastern/Western) split 1507 ± 129 years before 2015 (Hanna et al. 2018). Mappings
checked against Glottolog 5.3. Kept as records: Southern-only Batak splits, Ternate–
Tidore (a low North Halmahera subgroup), Róna-Tas's Oghur/Oghuz split (Proto-Turkic).

Correction (2026-10-09): this run's review and its first README named gpt-6-astra; the run used gpt-6.1-sol (its brief.json). Recorded as an erratum in the ledger; the decisions were re-recorded with the right attribution.
