# Status

*Updated 2026-10-04. Local `main` has `wiktionary-refresh`, `stable-links` and the
server deployment automation merged; nothing pushed.*

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
- **Dates:** 14 of 20 pilot codes have a cited period.

## What changed

- Merged the deployment automation; its instructions no longer mention the legacy
  graph, and its first rollout holds the website on `production` until the new API
  serves. `deploy/` no longer supports legacy graphs.
- The root README describes serving the selected graph only.

## Uncertainties

- The host setup has not been run. The server needs about 5 GB of memory and about
  two minutes (M1 Pro) to load this graph.
- Trees are still smaller than production in places (PIE *méh₂tēr, English+Latin:
  142 nodes vs 173). About 380k entries (64k English) give their etymology only in
  `{{ety}}`/`{{etymon}}`, which the processor does not parse.
- The client's reload on 409 was not exercised in a browser.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)),
   including the first push of `main`.
2. Parse `{{ety}}` and `{{etymon}}`; rebuild; publish the graph with
   `deploy/publish-graph.sh`.
3. Supply the six queued works in Zotero, then `dating holdings sync`.

## Decisions needed from the maintainer

- None beyond running the deployment.
