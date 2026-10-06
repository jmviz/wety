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
- **Dates:** 182 codes show a cited period (14 before this branch). New: Old East
  Slavic, Old Korean, Old Spanish, Ottoman Turkish (16th c. – 1928), Old and Middle
  Georgian, Bactrian, Khotanese, Andalusi Arabic; Sirenik, Ubykh, Norn, Kamassian and
  Akkala Sámi end with their last speakers. 37 proto-languages end at reproduced
  crown ages, now also Dravidian, Turkic, Semitic, Japonic, Bantu, Pama-Nyungan,
  Kra-Dai and Tai, Uto-Aztecan, Tupí-Guaraní ([families/](packages/dating/families/)).
  Checked in the browser: labels, popover evidence, dated varieties and alternatives.
- **Sources:** holdings carry tables of contents, also as Zotero child notes tagged
  `wety:contents`, so Zotero's search finds chapters for any project.

## What changed

- Ten analysis manifests from deposited trees; Koile 2022 as a Proto-Bantu
  alternative. Calibrated nodes whose posteriors restate their priors are left out.
  Summary nodes without heights yield no claim.
- Labels round only years computed from an age; printed years stay as printed. A
  statement imported by two runs is listed once.
- 13 source requests filed in Zotero (e.g. Gray 2009, Bouckaert 2012 and 2018
  supplements, Janhunen's Mongolic Languages, Vajda's Yeniseian, a Pāli handbook).

## Uncertainties

- Reviews are model reviews (Astra researches, Claude reviews). Crown ends rest on
  one analysis each; several nodes are bounded by the authors' calibrations (Bantu
  4,000–5,000 B.P., Dravidian South I and North). Epochs: Turkic stated; Dravidian and
  Japonic implied by their calibrations; the rest a labeled 1950 convention.
- The server needs ~5 GB; ~380k entries' `{{ety}}`/`{{etymon}}` are not parsed.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)).
2. Supply the Zotero requests (`wety:needs-fulltext`), then `dating holdings sync`
   and a batch on what they settle (Pali, Proto-Mongolic, Old Uyghur).
3. Next undated targets need sources rather than runs: Sanskrit and Egyptian as
   wholes, Vulgar and New Latin, Proto-Indo-Aryan, Proto-Hellenic.

## Decisions needed from the maintainer

- None blocking. Approve or reject: reading an epoch from a source's own calendar
  calibrations (Kolipakam: 254 BCE = 2250 years ago, so AD 2000; Lee & Hasegawa:
  407 YBP = 1603, so AD 2010) rather than the 1950 convention.
