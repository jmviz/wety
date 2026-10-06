# Status

*Updated 2026-10-06. Local `main` has `wiktionary-refresh`, `stable-links` and the
server deployment automation merged; nothing pushed. Branch
`claude/language-dating-period-9aa9ac` adds dating rules 2, eight period batches and
crown ages from deposited trees.*

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
- **Dates:** 167 codes show a cited period (14 before this branch). New: Old East
  Slavic, Old Korean, Ottoman Turkish (16th c. – 1928), Old and Middle Georgian;
  Sirenik and Ubykh end with their last speakers. 32 proto-languages end at reproduced
  crown ages, now also Dravidian, Turkic, Semitic, Japonic, Bantu and Pama-Nyungan
  ([families/](packages/dating/families/)).
- **Sources:** holdings carry tables of contents, also as Zotero child notes tagged
  `wety:contents`, so Zotero's search finds chapters for any project.

## What changed

- Six analysis manifests from deposited trees; Koile 2022 as a Proto-Bantu
  alternative. Summary nodes without heights yield no claim.
- Labels round only years computed from an age; printed years stay as printed.
- Four source requests filed in Zotero (Gray 2009, Bouckaert 2018, Woiwurrung,
  Yukaghir borrowings).

## Uncertainties

- Reviews are model reviews (Astra researches, Claude reviews). Crown ends rest on
  one analysis each; several nodes are bounded by the authors' calibrations (Bantu
  4,000–5,000 B.P., Dravidian South I and North). Epochs: Turkic stated; Dravidian and
  Japonic implied by their calibrations; the rest a labeled 1950 convention.
- The client's span and alternatives display is untested in a browser.
- The server needs ~5 GB; ~380k entries' `{{ety}}`/`{{etymon}}` are not parsed.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)).
2. Review the running batches: top undated targets (Sanskrit, Pali, Egyptian, Old
   Spanish, Old Uyghur, Proto-Tai …) and the second deposits round (Uto-Aztecan,
   Mongolic, Tungusic, Bouckaert 2012).
3. Supply the four Zotero requests (`wety:needs-fulltext`), then `dating holdings sync`.

## Decisions needed from the maintainer

- None blocking. Approve or reject: reading an epoch from a source's own calendar
  calibrations (Kolipakam: 254 BCE = 2250 years ago, so AD 2000; Lee & Hasegawa:
  407 YBP = 1603, so AD 2010) rather than the 1950 convention.
