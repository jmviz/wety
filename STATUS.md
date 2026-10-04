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
  mid-7th c. BCE (Wallace in Kapović) instead of Koch; `hit` attested c. 19th century –
  1200 BCE (Kapović, Fortson) instead of the Glottolog window. Export checked, not
  viewed in a browser this time (same schema).
- Decisions can be withdrawn (`dating withdraw`, or a review's `withdraw` list), kept
  with the reason in `_decisions/withdrawals.json`.
- **Supplied sources:** `dating holdings zotero KEY…` / `sync` retains a PDF from
  Zotero with acquire's provenance (artifact, receipt, hashed text, OCR for scans).
  Researchers read holdings with `library` and `open`, and ask for missing works with
  `request`. Seven handbooks are ingested (`data/registry/holdings.json`).
- **Acquisition queue:** `dating requests list|enrich|push`. Six works are open and
  filed in Zotero under `wety/needs-fulltext` and `wety/dating/periods`, with full
  records (DOI via Crossref, books via K10plus: publisher, place, edition, ISBN, pages,
  editors) and a note on what each would settle.
- Codex runs the pinned CLI from `openai-codex-cli-bin`; Claude the SDK-bundled CLI.
  Research searches on Semantic Scholar now send `S2_API_KEY` and share `s2.py`'s
  machine-wide 1 request/s clock.

## What changed

Holdings, requests, Zotero client and bibliographic lookup (`harness/holdings.py`,
`zotero.py`, `bibliography.py`); broader discovery; a handbook retry run. See the
[retry README](data/workspaces/dating/_research/period-retry-2026-10-04/README.md).

## Uncertainties

- The review was done by the session that built the route, not a separate one.
- Still no period: `non got cu sa ojp ine-pro`. Several are scope gaps, not access
  gaps (Frellesvig dates Old and Middle Japanese separately; Lunt's OCS excludes Middle
  Bulgarian). Catalog scope is now recorded: `cu` narrower than ISO `chu`/Glottolog
  `chur1257`, `sa` broader than `san`/`sans1269`.
- Zotero records can be wrong; reviews cite title pages. Corrected records refresh a
  holding's metadata on the next `holdings zotero`/`sync`.

## Next three actions

1. Supply the six queued works (attach PDFs to the Zotero items), then `dating holdings
   sync` and a continuation run for `ang enm goh gmh la grc sa och ltc`.
2. Wiktionary refresh, if approved: retain Kaikki's raw data (2026-09-02 dump) and the
   matching 20260901 Wiktionary dump before either rotates out; export language data;
   review code changes; rebuild catalogs, dating and the graph.
3. Re-check `non` scope against the refreshed export (Old Gutnish now has its own code).

## Decisions needed from the maintainer

- Merging into `main` (deploys).
- Whether to acquire the refresh inputs now (about 4.3 GB) and run the coordinated
  refresh (graph rebuild and server update included).
