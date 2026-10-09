# Published proto-language estimates, round 6, 2026-10-07

[Batch brief](../briefs/proto-estimates-round6-2026-10-07.md).

## Run

- Profile `codex-sol` (gpt-6.1-sol, high effort, Codex CLI on ChatGPT subscription, codex-cli 0.160.0).
  2700 s + 600 s; used 2518 s (completed).
  499 tool actions. Usage 12,428,132 input tokens (11,837,824 cached), 47,859 output.
  `check`: 10 assertions, 6 errors. Requests: Mayan Linguistics: Where Are We Now?; Hawaiki, Ancestral Polynesia: An Essay in Historical Anthropology; Comparative Eskimo Dictionary with Aleut Cognates; Language Relations across Bering Strait: Reappraising the Archaeological and Linguistic Evidence; The Katuic Languages: Classification, Reconstruction and Comparative Lexicon; Bayesian phylogenetic analysis of Philippine languages supports a rapid migration of Malayo-Polynesian languages; Lexicostatistic Classification; Proto-Micronesian Reconstructions—I; Revised Lexicostatistical Classification of Salishan Languages.

## Review (Claude Opus 5.5 (Claude Code))

Quotes exact in the retained texts. 2 of 10 supported; 1 codes adopted.

GPT-6.1 Sol (high), 16 high-value proto codes, each with literature leads. Only
Proto-Mayan was settled: Kaufman 1976's glottochronological 4200 BP, as Atkinson 2006
reports it, replaces ASJP's c. 220 BCE; Swadesh 1967's 3600 BP is the alternative.
Atkinson's own Bayesian roots (means about 6000–6500 years, 95% interval about
4000–9000 BP, 29–30 sampled doculects) are recorded as candidates: per-model means are
not stated separately, and wety shows a Bayesian alternative only beside a sampled-clade
default. Candidates: Marck 2000 and Pawley 1996 on East Polynesian's separation (inside
Nuclear Polynesian per Glottolog 5.3), Proto-Polynesian's disintegration (superset),
Sidwell & Jacq 2003 on West Bahnaric (subset), Collins's Proto-Malay (the Malay branch,
per Adelaar 2004). Nothing citable for the other codes.

Revision (2026-10-08, `coordinator-review/revision-atkinson.json`): Atkinson's two roots
are supported sampled-clade posteriors (taxa checked against Fig. 6.3, analyses
registered in the research graph) and shown beside Kaufman's default for Proto-Mayan,
both labeled by the prose's 95% interval of about 4000–9000 BP.

Correction (2026-10-09): this run's review and its first README named gpt-6-astra; the run used gpt-6.1-sol (its brief.json). Recorded as an erratum in the ledger; the decisions were re-recorded with the right attribution.
