# Agent instructions: dating

These add to the root [AGENTS.md](../../AGENTS.md) for work in `packages/dating/` and the
dating research in `data/workspaces/dating/`.

- The contract is [README.md](README.md). Each family has one page in
  [families/](families/) giving the current reading, with corrections applied.
- Format the package with `ruff format`.
- Use the harness commands for research workflow steps (`dating brief`, `review`,
  `publish`, `run-readme`, `review-summary`, `adopt-clades`, `ledger`); a step done twice
  belongs in the harness, not in a session script. Set `WETY_REVIEWER` to name the
  reviewing model; the researcher is read from the run.
- Editing `harness/` changes the dating generation fingerprint: republish
  (`uv run --frozen python -m harness.cli synthesize`) and validate in the same change.
  Decisions survive code edits; they go stale only when their subject's evidence or
  catalog mapping changes, or `DECISION_RULES` is bumped. Check
  `rejected_decisions.json` after publishing.
- A catalog generation goes stale only when its sources, decisions or mirrors change;
  the workflow run receipt records the code. Code edits need no catalog rebuild; rerun
  `uv run --frozen wety-data run catalogs` when a change should alter the catalog.
  Shared tooling (`workflow_tools`, `source_tools`) belongs wherever it fits best; no code
  outside `harness/` and `uv.lock` is part of a fingerprint.
