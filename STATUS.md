# Status

*Updated 2026-10-04. Local `main` has `wiktionary-refresh`, `stable-links` and the
server deployment automation merged; nothing pushed. Branch
`claude/language-dating-period-9aa9ac` adds the supplied-handbook period round.*

## What the project promises

wety shows Wiktionary etymologies as trees. Behind a toggle, language nodes show when
the language was spoken: an interval, its basis and citations, from reviewed evidence
in the `packages/dating` registry.

## What is usable today

- **Graph** (`etymology-graph/bf5c28d2…`, 2026-09-01 dump, selected for local use and
  for deployment in `deploy/graph.json`): homographs are separate items (`en lime`
  has its five etymologies) and Descendants sections are read.
- **Stable links:** URLs name a word, e.g. `/cognates/en/lime?ety=3&lang=la`, and
  survive rebuilds. `/resolve` finds the word; a chooser or a switcher handles
  homographs; a stale tab reloads (409). Language codes replace numeric IDs.
- **Deployment:** pushing `main` builds the server in CI, deploys it to the host via
  R2 with a health check and rollback, then moves `production`, from which Pages
  builds the website. One-time setup, going straight to the new graph and website:
  [deploy/README.md](deploy/README.md). Untested on the host.
- **Dates:** 14 of 20 pilot codes have a cited period. Old and Middle English now
  come from Baugh & Cable (450–1150, 1150–1500), Old and Middle High German from
  Schmidt (500–1050, 1050–1350), replacing ISO names and a start-only attestation.

## What changed

- The six requested works and five Woodard *Ancient Languages* volumes are holdings;
  all requests are closed.
- A third period run read them (41 assertions, 13 supported, every quote page
  checked). Adopted: `ang enm goh gmh`; nine older decisions re-adopted unchanged.
  [Run README](data/workspaces/dating/_research/period-holdings-2026-10-04/README.md).

## Uncertainties

- The host setup has not been run. The server needs about 5 GB of memory and about
  two minutes (M1 Pro) to load this graph.
- Trees are still smaller than production in places (PIE *méh₂tēr, English+Latin:
  142 nodes vs 173). About 380k entries (64k English) give their etymology only in
  `{{ety}}`/`{{etymon}}`, which the processor does not parse.
- The client's reload on 409 was not exercised in a browser.
- `gmh`: Schmidt's 1050–1350 replaces the ISO period you had kept over Wright (ISO's
  runs into Early New High German, filed under `de`).
- `non got cu sa ojp ine-pro` still lack a whole-code source (only stages, witnesses
  or bounds are dated). Nothing further is requested.
- Dating in a worktree needs main's ignored `data/`; cloned in, new bytes copied back.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)),
   including the first push of `main`.
2. Parse `{{ety}}` and `{{etymon}}`; rebuild; publish the graph with
   `deploy/publish-graph.sh`.
3. Dating: decide how the six remaining codes may be shown (below), or leave them
   without a period.

## Decisions needed from the maintainer

- Running the deployment.
- For `ojp` (Old + Middle Japanese) and `sa` (Vedic to learned Sanskrit), sources give
  only per-stage dates. Show nothing, or allow a display built from stated stage
  periods (a rule change to `DECISION_RULES`)?
- Use the Woodard volumes for codes beyond the 20 pilot codes (a new research
  direction)?
