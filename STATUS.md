# Status

*Updated 2026-10-07. Local `main` has `wiktionary-refresh`, `stable-links` and the
server deployment automation merged; nothing pushed. Branch
`claude/language-dating-period-9aa9ac` adds dating rules 2, regional period batches and
published proto-language estimates.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Graph** (`etymology-graph/bf5c28d2…`, 2026-09-01 dump, in `deploy/graph.json`):
  homographs are separate items; Descendants sections are read.
- **Stable links:** URLs name a word (`/cognates/en/lime?ety=3&lang=la`) and survive
  rebuilds; language codes replace numeric IDs.
- **Deployment:** pushing `main` deploys the server, then the website
  ([deploy/README.md](deploy/README.md)). Untested on the host.
- **Dates:** 348 codes show a cited period (14 before this branch); 95% of the tree
  items in past-stage languages are dated. 135 proto-languages carry an estimated end,
  labeled with its method and the source's reported mean or median: 52 Bayesian
  phylogenies, 50 lexicostatistical (21 still ASJP points, replaced as family-specific
  estimates are found), 33 expert, comparative or archaeological estimates. Revived
  languages end at their last native speakers with a cited "being revived" beside.
  Checked in the browser.
- **Sources:** holdings carry tables of contents, also as Zotero child notes tagged
  `wety:contents`.

## What changed

- Research runs now default to GPT-6.1 Sol at high effort (`codex-sol`); Astra stays
  selectable. Fixes: JSONL split on newlines only (a U+2028 had marked
  finished runs failed); "since X" revivals start at X; counted-back years print to
  the decade ("c. 50 BCE").
- Sol rounds 5C–6, three tail runs and a retry: 21 new codes and 5 ASJP points
  replaced, e.g. Proto-Mayan (Kaufman 4200 BP,
  replacing ASJP), Proto-Hellenic (after c. 1900 BCE), Proto-Algic, Proto-Lezgian,
  Eyak, Mochica, Old Tibetan, Raetic, Phrygian.
- Uncalibrated, exactly mapped subclades of imported trees: Bodo-Garo, Naish, Ersuic,
  Central Naga (Zhang 2019), Sabaki (Grollemund; Koile alternative), Central New South
  Wales (Bouckaert 2018). Loloish, Karen, Numic, Takic, Kra, Be and Kam-Sui stay out:
  their nodes are calibrated.

## Uncertainties

- Reviews are model reviews (Sol, earlier Astra, researches; Claude reviews). ASJP
  points carry large errors (Proto-Karen c. 350 BCE); better-argued estimates replace them as
  defaults when found. Epochs: stated, from a source's own calibrations (approved),
  or a labeled 1950 convention.
- The server needs ~5 GB; ~380k entries' `{{ety}}`/`{{etymon}}` are not parsed.

## Next three actions

1. Proto rounds now yield 1–5 codes each: target codes with known literature, and
   Bayesian alternatives beside non-tree defaults (Atkinson's Mayan roots are recorded
   but cannot be shown yet).
2. Supply the Zotero requests (`wety:needs-fulltext`), then `dating holdings sync`.
3. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)).

## Decisions needed from the maintainer

- None. Applied 2026-10-06: revived languages end at their last native speakers with a
  "rev" mark and cited revival beside; "years ago" estimates read before 1950 under a
  labeled convention; proto labels name the source's reported mean or median.
