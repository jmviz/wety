# Agent instructions

These rules apply to every agent working in this repository (Claude, Codex, others).
The maintainer supervises with limited time: work must stay easy to track and digest.

## Orientation

Read [STATUS.md](STATUS.md) first. Layout: `crates/` (Rust graph, processor, server),
`client/` (frontend), `packages/` (Python: catalogs, dating, source/workflow tools),
`workflows/`, `data/` (local data, mostly untracked). The dating contract is
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
- `main` deploys the website automatically (Cloudflare). Never merge into or push
  `main`; the maintainer does that after updating the host server.
- Commit at coherent checkpoints and always before ending a session. Leave no
  uncommitted work behind in the main checkout or a worktree.
- Stage specific paths and inspect what is staged; never commit source PDFs or other
  publisher content, logs, model transcripts, caches or generation bundles.
- `data/` is ignored by default. To version a new kind of small, human-meaningful
  record, add an explicit rule to `.gitignore` in the same commit.

## Documents

- `STATUS.md` is overwritten at the end of each task, never appended to. Keep it under
  500 words: promise, what is usable, what changed, uncertainties, next three actions,
  decisions needed.
- Do not write dated reports, session logs or "next steps" documents. Next actions
  live only in `STATUS.md`; history lives in git.
- A new document must replace or merge an existing one. Each family has one page in
  `packages/dating/families/` giving the current reading, with corrections applied.
- Run artifacts belong in the run's directory under `data/`, not in docs.

## Code

- Match the surrounding style. Format the dating package with `ruff format`.
- Run the relevant tests before committing (commands in the root README).
- Editing `packages/dating/harness/` changes the dating generation fingerprint:
  republish (`uv run --frozen python -m harness.cli synthesize`) and validate in the
  same change. Decisions survive code edits; they go stale only when their subject's
  evidence or catalog mapping changes, or `DECISION_RULES` is bumped. Check
  `rejected_decisions.json` after publishing. Catalog generations likewise fingerprint `packages/catalogs`,
  `packages/source_tools`, `packages/workflow_tools` and the root `pyproject.toml`;
  changing those requires a catalog rebuild, so avoid incidental edits there.

## Reporting

Report results in wety terms: what a user of wety can now see or do (for example, "N
language codes gained a displayable, cited period"). Separate what is established from
what is proposed. State failures and skipped steps plainly.
