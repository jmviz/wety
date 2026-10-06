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
- **Dates:** 137 codes show a cited period (14 before this branch), e.g. Gothic, Old
  Norse, Old and Middle French, Middle Persian, Old Javanese, Medieval Latin, the Celtic
  and Egyptian stages; 25 proto-languages end at reproduced crown ages (IE, Sino-Tibetan,
  Uralic); PIE also lists the steppe-compatible scenario. `dating period-targets`: 954
  undated past-stage codes appear in trees.
- **Sources:** 33 holdings with tables of contents (searched by `library`); Zotero
  conventions for books, volumes, chapters and abstracts in
  [single-frontier.md](packages/dating/docs/single-frontier.md#supplied-sources-and-requests).

## What changed

- Holdings record contents and volume; abstracts become abstract holdings; a whole book
  answers chapter requests; trashed requests are recorded as declined; sync shelves
  items in `wety/holdings`.
- Proto-languages are codes named "Proto-…"; crown decisions may list reviewed
  alternatives; manifests may carry a labeled conventional epoch.
- Runs: supplied requests (9 codes), IE Handbook (10); Zhang 2019 and Honkola 2013
  imported (8 proto ends). Two new requests are filed in Zotero.

## Uncertainties

- The client's span and alternatives display and its 409 reload are untested in a
  browser.
- Reviews are model reviews (Astra researches, Claude reviews). Proto ends rest on one
  analysis each, conditional on its constraints; Honkola's epoch is a labeled 1950
  reading. Chang et al.'s own trees are not reproducible here.
- `period-targets` misses non-ISO stages unless added by hand.
- The server needs ~5 GB; ~380k entries' `{{ety}}`/`{{etymon}}` are not parsed.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)).
2. Next batches from `period-targets`: Ottoman Turkish and Old Spanish (need a source
   for the whole stage), Old East Slavic, Proto-Bantu, Proto-Tai, Austronesian protos.
3. More crown ages: Sagart 2019 (epoch), Austronesian and Bantu deposits.

## Decisions needed from the maintainer

- Show an end of native transmission (last speaker: Dalmatian 1898, Negerhollands
  1987, Polabian) as a period's end, basis "last speaker"?
