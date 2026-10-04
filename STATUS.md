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
- **Dates:** 14 of 20 pilot codes have a cited period; `ang enm` now from Baugh &
  Cable, `goh gmh` from Schmidt.

## What changed

- The six requested works and five Woodard *Ancient Languages* volumes are holdings;
  all requests are closed, and sync now also takes supplied items out of Zotero's
  `wety/needs-fulltext`.
- A third period run read them; adopted `ang enm goh gmh`
  ([run README](data/workspaces/dating/_research/period-holdings-2026-10-04/README.md)).

## Uncertainties

- The host setup has not been run. The server needs about 5 GB of memory and about
  two minutes (M1 Pro) to load this graph.
- Trees are still smaller than production in places (PIE *méh₂tēr, English+Latin:
  142 nodes vs 173). About 380k entries (64k English) give their etymology only in
  `{{ety}}`/`{{etymon}}`, which the processor does not parse.
- The client's 409 reload is untested in a browser.
- `gmh`: Schmidt's 1050–1350 replaced the ISO period you had kept over Wright.
- `non got cu sa ojp ine-pro` lack a whole-code source. For `ojp` and `sa` this is the
  whole-code rule: Wiktionary files stages under them as variety codes (`ja-mid-ear`,
  `vsn`…; 716 varieties under 177 codes), so Frellesvig's OJ 700–800 is a subset.
- Proto-language dating from phylogenetic articles (family pages) stopped at
  reproduced analyses: no rule yet maps a clade estimate to a `*-pro` code.
- Worktrees lack main's ignored `data/`; dating work cloned it in.

## Next three actions

1. Maintainer: one-time deployment setup ([deploy/README.md](deploy/README.md)),
   including the first push of `main`.
2. Parse `{{ety}}` and `{{etymon}}`; rebuild; publish the graph with
   `deploy/publish-graph.sh`.
3. Dating, after the decisions below: varieties as subjects, a clade-to-proto rule,
   automatic re-adoption; then family-batch runs (Woodard first).

## Decisions needed from the maintainer

- Running the deployment.
- Varieties: date variety codes directly; judge a parent against its own core (code
  minus its varieties); export a derived span over core and varieties. Approve?
- Proto-languages: may a crown age over sampled descendants date a `*-pro` stage's
  end (estimate) when the taxa cover the family?
