# Proto-language ages from published dated phylogenies, second round, 2026-10-06

[Batch brief](../briefs/proto-estimates-phylogeny-articles-round2-2026-10-06.md).

## Run

- Profile `codex-astra` (gpt-6-astra, high effort, Codex CLI 0.160.0). 2700 s + 600 s;
  used 2590 s (failed). 570 tool actions. Usage not reported.
  `check`: 9 assertions, 0 errors. Requests: Language trees with sampled ancestors support a hybrid model for the origin of Indo-European languages; A Bayesian approach to the classification of Tungusic languages; From Species to Languages: a phylogenetic approach to human prehistory; Bayesian phylogenetics illuminate shallower relationships among Trans-Himalayan languages in the Tibet-Arunachal area.

## Review (Claude Opus 5.5)

Quotes exact in the retained texts. 4 of 9 supported; 4 codes adopted.

The run used its whole time and failed the check: its times were in a non-registry
form (numbers under `point`/`interval`); the review rewrote them, numbers unchanged.
Each adopted text claim names the node's sampled languages (`taxa`), and its analysis
is registered in `_research/graph.json`.

Adopted (Bayesian node ages; proto_mapping in each decision):
- `tuw-pro`: Oskolskaya et al. 2021 via Robbeets & Oskolskaya 2022, 21 doculects,
  median c. 1500 BP (AD 450), 95% HPD 737 BC – AD 1154 (before 1950, stated). Replaces
  the ASJP point (kept).
- `aav-khs-pro`: Nagaraja, Sidwell & Greenhill 2013, Khasi, Pnar, Lyngngam, War;
  median 1350, 95% HPD 1028–1737 (1950 convention labeled).
- `sit-khw-pro`, `sit-bdi-pro`: Wu, Bodt & Tresoldi 2022, Western Kho-Bwa (node 49)
  and East Bodish + Tibetan (node 18), 95% HPDs (1950 convention labeled).
- From trees already parsed: `qfa-lic-pro` from Tao et al. 2023's Hlai clade (loose
  0–3000 prior); `tbq-pro` from Zhang et al. 2019's non-Sinitic clade (support 0.68;
  replaces the ASJP point, kept).

Not adopted: Proto-Kra (posterior restates its prior), Proto-Tani (node prior),
Gerardi's Mawé/Awetí/TG root (not all Tupian).
