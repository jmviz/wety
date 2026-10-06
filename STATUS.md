# Status

*Updated 2026-10-06. Local `main` has `wiktionary-refresh`, `stable-links` and the
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
- **Dates:** 300 codes show a cited period (14 before this branch); 93% of the tree
  items in past-stage languages are dated. 114 proto-languages carry an estimated
  end, labeled with its method and the source's reported mean or median: 46 Bayesian
  phylogenies, 46 lexicostatistical (ASJP points give way to family-specific
  estimates as found), 22 expert or archaeological estimates. Revived languages end at
  their last native speakers with a cited "being revived" beside. Checked in the browser.
- **Sources:** holdings carry tables of contents, also as Zotero child notes tagged
  `wety:contents`.

## What changed

- A published family age can date a proto-language; an article's Bayesian node age
  becomes a claim about its sampled clade; the popover shows each estimate's method.
- Regional batches: European stages, Coptic and Aramaic, East and Southeast Asia,
  fragmentary ancient languages, the Americas, Australia and the Andamans.
- Labels: printed years stay as printed; a lower-bound-only end prints "after"; one
  dated record is not a start.

## Uncertainties

- Reviews are model reviews (Astra researches, Claude reviews). ASJP points carry
  large errors (Proto-Mayan c. 221 BCE); better-argued estimates replace them as
  defaults when found. Epochs: stated, from a source's own calibrations (approved),
  or a labeled 1950 convention.
- The server needs ~5 GB; ~380k entries' `{{ety}}`/`{{etymon}}` are not parsed.

## Next three actions

1. Further proto rounds for the ~125 codes still undated or ASJP-only (each round
   adds 5–10), and the historical/extinct tail under 15 tree items.
2. Supply the Zotero requests (`wety:needs-fulltext`), then `dating holdings sync`.
3. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)).

## Decisions needed from the maintainer

- None. Applied 2026-10-06: revived languages end at their last native speakers with a
  "rev" mark and cited revival beside; "years ago" estimates read before 1950 under a
  labeled convention; proto labels name the source's reported mean or median.
