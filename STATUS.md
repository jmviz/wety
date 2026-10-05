# Status

*Updated 2026-10-04. Local `main` has `wiktionary-refresh`, `stable-links` and the
server deployment automation merged; nothing pushed. Branch
`claude/language-dating-period-9aa9ac` adds dating rules 2 and five period batches.*

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
- **Dates:** 112 codes show a cited period (14 before this branch), e.g. Gothic, Old
  Norse, Old French, Sumerian, Old and Middle Japanese, Classical Latin; Vedic and
  Avestan by composition; 14 IE proto-languages end at Heggarty et al.'s crown ages
  (PIE until 7614–4736 BCE). `dating period-targets` ranks 1,763 past-stage codes by tree use: 99
  dated, 988 undated ones appear in trees.

## What changed

- Rules 2: varieties (etymology-only codes) are dated as their own codes; a parent
  shows a derived span over its dated varieties beside its own period; a proto code
  may end at a sampled clade's crown age (decision `proto_mapping`).
- `adopt` keeps choices a review did not replace; `readopt` after rule bumps;
  revision reviews re-target earlier assertions; `check` validates times and events.
- Five batch runs (212 assertions; 65 codes adopted after review), revisions of the
  earlier runs (19 codes), composition windows (4). Sync takes supplied items out of
  Zotero's `needs-fulltext`; 16 requests are filed there. Old generations collected.

## Uncertainties

- The client's span display and its 409 reload are untested in a browser.
- Reviews are model reviews (Astra researches, Claude reviews), not specialist
  adjudication. Woodard's chapters date from 2004; Old and Middle French rest on a
  paper's passing statement until Rickard 1989 arrives.
- `period-targets` uses ISO's historical/extinct types, so non-ISO stages (Gaulish,
  Proto-Norse) are missed unless added by hand.
- IE proto ends rest on one analysis (Heggarty M3), conditional on its constraints;
  Chang et al. 2015 is unreviewed. Sino-Tibetan, Uralic and other crown ages remain
  candidates.
- The server needs ~5 GB; ~380k entries' `{{ety}}`/`{{etymon}}` are not parsed.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)).
2. Proto-language reviews: Chang et al. 2015 as an IE alternative; Sino-Tibetan and
   Uralic (deposits reproduced) next.
3. Supply the 16 filed requests (`holdings sync`), then the next batches from
   `period-targets` (Ottoman Turkish, Old East Slavic, Old Spanish, proto-languages).

## Decisions needed from the maintainer

- Running the deployment.
