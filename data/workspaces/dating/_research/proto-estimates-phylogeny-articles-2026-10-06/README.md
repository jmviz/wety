# Proto-language ages from published dated phylogenies (partial), 2026-10-06

[Batch brief](../briefs/proto-estimates-phylogeny-articles-2026-10-06.md).

## Run

- Profile `codex-astra` (gpt-6-astra, high effort, Codex CLI 0.160.0). 2700 s + 600 s;
  used 1040 s (failed). 351 tool actions. Usage not reported.
  `check`: 71 assertions, 0 errors. Requests: Cultural and climatic changes shape the evolutionary history of the Uralic languages.

## Review (Claude Opus 5.5)

Quotes exact in the retained texts. 3 of 71 supported; 3 codes adopted.

The run stopped at the ChatGPT subscription's usage limit (partial delivery); most of
its 71 rows repeat Holman et al. 2011's ASJP table (already imported by run
`proto-estimates-asjp-2026-10-06`) and King et al. 2024; those stay candidates.

Harness change: an article's node age becomes a claim about its sampled clade when the
review lists the descendants the source names (`taxa`); adoption then carries a
`proto_mapping`, as for a parsed tree's clade. The two analyses are registered in the
research graph (`_research/graph.json`, analysis_of), as supported posteriors require.

Adopted (Bayesian node ages; the ASJP points stay as alternatives):
- `ccs-pro`: Gavashelishvili et al. 2023, Svan against Karto-Zan (sample Georgian, Old
  Georgian, Megrelian, Laz, Svan), mean 7641 BP, 95% HPD 18,626–1169 BP (before 1950,
  stated). The interval is very wide; its late end postdates Old Georgian's texts.
- `ccs-gzn-pro`: same analysis, Georgian against Zan, mean 2617 BP, 4323–1178 BP.
- `mkh-asl-pro`: Dunn et al. 2013 (Human Biology 85), all four Aslian branches, mode
  4305 BP, 95% HPD 2943–6570 BP (no root prior), 1950 convention labeled.
