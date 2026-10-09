# Published proto-language estimates

A batch brief for collecting published estimates of when proto-languages were spoken or
broke up. `dating brief` fills `{{IMPORTED}}` (the works whose estimates are already
recorded, from the registry) and `{{CODES}}` (one line per code: name, tree items,
whether its only date is an ASJP point, and optional leads). The research harness fills
`{{FAMILY}}`. Maintainer decisions 2026-10-06: published estimates at scale; "years ago"
readable under a labeled convention.

```text
Collect every published estimate of when each proto-language below was spoken or broke
up ({{FAMILY}}). Work as one researcher, without subagents or other model calls. Do not
redesign tools.

Kinds of estimate, each its own claim with its method: a Bayesian or other dated
phylogeny's node age for the named family or subgroup (method bayesian_phylogenetic; list
the node's sampled descendants in subject.taxa); lexicostatistical or glottochronological
dates (lexicostatistic); a specialist's dating from reconstructed vocabulary, loans or
sound changes (comparative_reconstruction); archaeological correlations
(archaeological_correlation); other expert statements (expert_estimate). Event
crown_divergence for a breakup or family age, conventional_period for when the stage was
spoken. Record ages stated as "years ago" or "BP" as printed (unit years_ago or BP),
with present only if the source states it. Map exact only if the dated group is the
family the code reconstructs (its first split sampled); otherwise subset, superset or
overlap with the reason. Keep calibration priors apart from posteriors. Never quote from
memory; quote exactly from retained text with a locator.

Already recorded, do not collect again: {{IMPORTED}}. Look further: specialist grammars'
and handbooks' introductions, regional journals (JSEALS, Diachronica, IJAL, Oceanic
Linguistics), regional archaeology syntheses.

Codes (Wiktionary proto code, name, number of wety tree items; "ASJP only" marks codes
whose only date is an ASJP point). Leads are pointers, not evidence; read and quote the
work itself.
{{CODES}}

Deliverables (write action): assertions.jsonl (id, code, source_id, quote, locator,
event, role, method, native_time in the registry's form: {"form": "point" | "event_range" |
"duration" | "distribution" | "bound", "unit": ..., "value" or "lower"/"upper",
"median"/"mean" for posteriors, "interval_type", "probability", "present" if stated},
subject {code, source_label, mapping, mapping_basis, taxa}, proposed_status, citation),
report.md (table: code, study, estimate, method, mapping; then codes with nothing found),
sources.json, self-check.json. Call check after saving, fix problems once, and finish.
An unstated epoch is not a problem to fix.
```
