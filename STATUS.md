# Status

*Updated 2026-10-17. Client audit fixes are on `codex/client-ux-audit`; not deployed.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Site:** wety.org and api.wety.org. Deployment follows `main` and `production`
  ([deploy/README.md](deploy/README.md)); only the maintainer merges/deploys.
- **Graph:** `dab06f67…`, the 2026-09-01 dump; 8,101,979 entries with an etymology,
  homographs and stable word URLs.
- **Dates:** 348 codes have cited periods; 135 proto-languages have estimated ends.
  Alternatives and their methods remain separate.
- **Research store:** private artifacts, evidence and runs live in `data/`, backed up
  with `wety-data store`. Git retains code, app inputs, briefs and run READMEs;
  [provenance](docs/architecture/provenance.md) describes recovery and syncing.
- **Local review:** `http://127.0.0.1:8000` serves the built client against the real
  local API on port 3000 while the audit's processes remain running.

## What changed

- Failed word loads show an error and Retry; pending loads have a status. Search
  distinguishes unavailable service from no matches, rejects failed HTTP responses,
  and prevents outdated responses from reopening dismissed suggestions.
- Navigation closes word details, preventing a crash when returning to a missing or
  ambiguous word. Missing-word links select the URL's search language.
- Clearing descendant languages preserves the word's own language; direct links
  also include it consistently.
- Long language chips leave room for the word field on small screens. Mobile zoom
  is allowed. Citation popovers scroll within short viewports, and pinned details
  receive keyboard focus. Search arrow navigation wraps and reveals its selection.
- Single-word/language counts use singular labels; language expansion accepts Afar's
  valid zero ID.
- Production build and TypeScript checks pass. Browsed real searches, homographs,
  all three views, filters, dates, glosses, expansion/collapse, Back navigation and
  narrow/short viewports. API-unavailable feedback was checked separately.

## Uncertainties

- Touch hardware and other browser engines were not tested.
- The UX recommendations remain proposals: descendant-language discovery, clearer
  relationship semantics, a compact filter panel, readable homograph choices and
  progressive disclosure of date evidence. No design overhaul was applied.
- Research reviews remain model reviews; 566k graph citations were resolved by
  meaning alone (about 85% accurate offline). R2 pruning still needs a live dry run.

## Next three actions

1. Review the audit recommendations in this chat and choose the next design change.
2. Maintainer: inspect the R2 pruning dry run before the next server deployment.
3. Resume approved dating research from `dating period-targets`.

## Decisions needed from the maintainer

- Prioritize the proposed UI changes; none are implemented without that decision.
