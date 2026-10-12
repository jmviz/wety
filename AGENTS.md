# Agent instructions

These rules apply to every agent working in this repository (Claude, Codex, others).
The maintainer supervises with limited time: work must stay easy to track and digest.

## Orientation

Read [STATUS.md](STATUS.md) first. Layout: `crates/` (Rust graph, processor, server),
`client/` (frontend), `packages/` (Python: catalogs, dating, source/workflow tools),
`workflows/`, `data/` (the private research store, `wety-data store`; git
tracks only the few files listed in `.gitignore`, see
[docs/architecture/provenance.md](docs/architecture/provenance.md)). The dating contract is
[packages/dating/README.md](packages/dating/README.md); per-family results are in
`packages/dating/families/`.

## Scope and approval

Every task states its question, budget, expected output and stopping condition. If the
budget runs out or the task stalls, stop and report the precise obstacle.

You may proceed without asking on: source research within an approved family or
pilot, bug fixes, tests, and doc updates that follow the rules below.

Ask the maintainer before: starting a new research direction or family, any
model/backend comparison or harness evaluation, changes to repository layout or
provenance machinery, new workflow infrastructure, and deleting anything under `data/`.

## Git

- Work on a branch, never directly on `main`. Do not push or open PRs unless asked.
- Pushing `main` deploys the API server and then the website
  ([deploy/README.md](deploy/README.md)). Never merge into or push `main` or
  `production`; the maintainer does that.
- Commit at coherent checkpoints and always before ending a session. Leave no
  uncommitted work behind in the main checkout or a worktree.
- Stage specific paths and inspect what is staged; never commit source PDFs or other
  publisher content, logs, model transcripts, caches or generation bundles.
- Research outputs (evidence, ledger, runs, packets, receipts, retained texts) never
  enter git: they live in the research store. Git gets code, docs, the files the app is
  built from, run READMEs and briefs. Changing that list is a maintainer decision.
- At each checkpoint, also bring a worktree's new data into the main checkout:
  `uv run --frozen wety-data store sync data <main>/data --apply` (it never
  overwrites; report its conflicts). Then snapshot from the main checkout
  (`wety-data store snapshot`) and commit `data/SNAPSHOT.json`; if the store is not
  configured in this session, say so.

## Worktrees and `data/`

Most of `data/` is ignored, so a git worktree (or any fresh checkout) lacks the
research store: artifacts, caches, evidence and runs. They live in the main checkout:
the first entry of `git worktree list`. Without that checkout, restore `data/` from the
store's snapshot (`wety-data store restore`, then `sync`;
[provenance](docs/architecture/provenance.md#working-with-it)); if the store is not
configured either, report it rather than re-acquiring sources. Do not pipe restic into
`head` (it can leave a stale lock); use `sed -n 1,5p`.

- Reading only: use the main checkout's files by absolute path.
- Running a workflow from a worktree: the runner rejects inputs outside the repository,
  symlinks included, so hard-link each file named in the workflow's `inputs` (`ln`, same
  volume, no copy). Paths passed only as command arguments may be symlinks, e.g.
  `data/cache/embeddings/current` (one graph build at a time may use it).
- Commit first: the receipt records the revision and whether the tree was dirty.
- Afterwards copy the run directory and artifact into the main checkout's `data/`
  (`cp -Rc`, an APFS clone). The runner also rewrites the tracked
  `data/products/*/CURRENT.json`; commit that only to adopt the output, else restore it.
- Serving another graph: set `WETY_GRAPH_PATH` to its file; one graph needs about 5 GB.

## Documents

- `STATUS.md` is overwritten at the end of each task, never appended to. Keep it under
  500 words: promise, what is usable, what changed, uncertainties, next three actions,
  decisions needed.
- Do not write dated reports, session logs or "next steps" documents. Next actions
  live only in `STATUS.md`; history lives in git.
- A new document must replace or merge an existing one. Each family has one page in
  `packages/dating/families/` giving the current reading, with corrections applied.
- Run artifacts belong in the run's directory under `data/`, not in docs.
- Link only to files git tracks; name a research-store path as code text
  (`data/workspaces/dating/_research/packets/...`), since links to it are dead on GitHub.

## Code

- Match the surrounding style. Format the dating package with `ruff format`.
- Run the relevant tests before committing (commands in the root README).
- Use the harness commands for research workflow steps (`dating brief`, `review`,
  `publish`, `run-readme`, `review-summary`, `adopt-clades`, `ledger`); a step done twice belongs in the harness, not in a session script. Set
  `WETY_REVIEWER` to name the reviewing model; the researcher is read from the run.
- Editing `packages/dating/harness/` changes the dating generation fingerprint:
  republish (`uv run --frozen python -m harness.cli synthesize`) and validate in the
  same change. Decisions survive code edits; they go stale only when their subject's
  evidence or catalog mapping changes, or `DECISION_RULES` is bumped. Check
  `rejected_decisions.json` after publishing.
- A catalog generation goes stale only when its sources, decisions or mirrors change;
  the workflow run receipt records the code. Code edits need no catalog rebuild; rerun
  `uv run --frozen wety-data run catalogs` when a change should alter the catalog.
  Shared tooling (`workflow_tools`, `source_tools`) belongs wherever it fits best; no
  code outside `packages/dating/harness/` and `uv.lock` is part of a fingerprint.

## Reporting

Report results in wety terms: what a user of wety can now see or do (for example, "N
language codes gained a displayable, cited period"). Separate what is established from
what is proposed. State failures and skipped steps plainly.
