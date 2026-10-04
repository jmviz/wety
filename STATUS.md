# Status

*Updated 2026-10-04. Branch `local-sources` (from `dating-pilot`), not pushed; nothing
merged into `main`.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **In the app:** the "dates" toggle. **14 of the 20 pilot codes have a cited period**
  (was 11). New: `akk` attested 26th c. BCE – 1st c. CE (Huehnergard), `goh` attested
  from c. 600 (Fortson), `sla-pro` estimated until c. 800/1000 (Fortson). Changed:
  `ang` 5th–11th century (Slocum & Lehmann) instead of ISO; `la` first inscriptions
  mid-7th c. BCE (Wallace in Kapović) instead of Koch. Export checked, not viewed in a
  browser this time (same schema).
- **Supplied sources:** `dating holdings zotero KEY…` / `sync` retains a PDF from
  Zotero with acquire's provenance (artifact, receipt, hashed text, OCR for scans).
  Researchers read holdings with `library` and `open`, and ask for missing works with
  `request`. Seven handbooks are ingested (`data/registry/holdings.json`).
- **Acquisition queue:** `dating requests list|push`. Six works are open and filed in
  Zotero under `wety/needs-fulltext` and `wety/dating/periods`, each with a note on what
  it would settle (write access granted, verified in the live library).
- Codex runs the pinned CLI from `openai-codex-cli-bin`; Claude the SDK-bundled CLI.

## What changed

Holdings, requests and Zotero client (`harness/holdings.py`, `harness/zotero.py`);
discovery adds Semantic Scholar, author homepages, Open Library and optional web search
(codex-astra: cached); a handbook retry run, reviewed with page-image checks. See the
[retry README](data/workspaces/dating/_research/period-retry-2026-10-04/README.md).

## Uncertainties

- The review was done by the session that built the route, not a separate one.
- Still no period: `non got cu sa ojp ine-pro`. Several are scope gaps, not access
  gaps (Frellesvig dates Old and Middle Japanese separately; Lunt's OCS excludes Middle
  Bulgarian). Catalog scope for `cu`/`sa` stays unrecorded: handbooks define stage
  names, not the ISO or Glottolog entities.
- Zotero records can be wrong (Huehnergard's says Brill 2018; the file is Eisenbrauns
  2011). The review reads title pages; holdings keep Zotero's metadata as is.

## Next three actions

1. Supply the six queued works (attach PDFs to the Zotero items), then `dating holdings
   sync` and a continuation run for `ang enm goh gmh la grc sa och ltc`.
2. Refresh the Wiktionary export behind the catalog (pending since 2026-10-03).
3. Record `cu`/`sa` catalog scope once a source states what ISO `chu`/`san` cover.

## Decisions needed from the maintainer

- `hit` has handbook attestation evidence (names 19th c. BCE, texts to c. 1200 BC) but
  shows the Glottolog window: switching needs a way to withdraw a decision, which the
  registry lacks. Add one?
- `gmh`: keep ISO c. 1050–1500, or prefer Wright 1917 (1100–1500)? Kept ISO.
- Merging into `main` (deploys) and the Wiktionary export refresh.
