# Status

*Updated 2026-10-04. Local `main` has `wiktionary-refresh`, `stable-links` and the
server deployment automation merged; nothing pushed. Branch
`claude/language-dating-period-9aa9ac` adds dating rules 2 and the Woodard batches.*

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
- **Dates:** 77 codes show a cited period (14 before this branch), including Gothic,
  Old Armenian, Sumerian, the Egyptian stages, Old and Middle Japanese, and varieties
  such as Classical Latin or Koine. `dating period-targets` ranks 1,763 past-stage
  codes by tree use: 76 dated, 1,009 undated ones appear in trees.

## What changed

- Rules 2: varieties (etymology-only codes) are dated as their own codes; a parent
  shows a derived span over its dated varieties beside its own period; a proto code
  may end at a sampled clade's crown age (decision `proto_mapping`).
- `adopt` keeps choices a review did not replace; `readopt` after rule bumps;
  revision reviews re-target earlier assertions; `check` validates times and events.
- Three Woodard batch runs (168 assertions; 46 codes adopted after page checks) and
  revisions of the earlier runs (19 codes). Sync takes supplied items out of Zotero's
  `needs-fulltext`; seven new requests are filed there.

## Uncertainties

- The client's span display and its 409 reload are untested in a browser.
- Reviews are model reviews (Astra researches, Claude reviews), not specialist
  adjudication. Woodard's chapters date from 2004.
- `period-targets` uses ISO's historical/extinct types, so non-ISO stages (Gaulish,
  Proto-Norse) are missed unless added by hand.
- About 1,000 crown-age claims (IE, Sino-Tibetan, Austronesian…) are all candidates;
  no proto code is dated by the new rule yet.
- The server needs ~5 GB; ~380k entries' `{{ety}}`/`{{etymon}}` are not parsed.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)).
2. Proto-language family review, Indo-European first: promote reproduced crown ages
   (analysis, result, taxa) and adopt `ine-pro` and branch ends.
3. Next period batch from `period-targets` (Old Norse, Old French, Middle Dutch,
   Middle Low German, Ottoman Turkish).

## Decisions needed from the maintainer

- May a composition window of orally transmitted texts (Vedic c. 1500–500 BC,
  Avestan) be shown, basis "composition"? It would date `vsn`, `sa-rig` and Avestan.
- Dating generations take ~1.9 GB locally; run `dating gc --apply`?
- Supply handbooks for the next batch, or let the researcher request them?
