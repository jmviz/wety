# Dating model configuration and comparison policy

The active configuration is [packages/dating/llm.config.json](../llm.config.json), schema version 2. It belongs to dating and is neutral about backend. Its settings are a baseline, not an endorsement. An older, pre-v2 configuration is kept as a test fixture (`tests/fixtures/llm-config-2026-09-20.json`).

The structure separates three concerns:

- `default_backend` and `roles.<role>.backend` select a backend; `DATING_LLM_BACKEND` overrides per-role selection.
- `roles.<role>.backends.<backend>` contains that backend's model, effort/reasoning and supported generation/routing settings. `anthropic` and `claude-agent` have independent role profiles.
- `backends.<backend>` contains connection/runtime policies. OpenRouter URL, headers, provider defaults, retries and plugins live under `backends.openrouter`; subscription limit policy lives under `backends.claude-agent`; executable/auth-store/timeout settings live under `backends.codex`.

Set `DATING_LLM_CONFIG` to a complete alternate v2 JSON file for an experiment. Relative paths resolve from the repository root. For example, after creating a reviewed trial configuration:

```sh
DATING_LLM_CONFIG=data/workspaces/dating/_research/evaluation/trial.json DATING_LLM_BACKEND=codex uv run --frozen dating llm-check codex
```

This example performs a live subscription-backed probe if run; the configuration itself is validated by offline tests. `llm-check` tests structured transport on one constructed case; it does not evaluate dating quality. Backend instances are cached by both name and config digest, with a copied configuration, so sequential experiments cannot accidentally reuse another cell's settings. Missing backend/model selections fail explicitly. Old dictionary shapes remain accepted only by programmatic compatibility accessors; on-disk configs require v2.

Request snapshots retain the selected backend's role settings, bounded runtime policy and whole-config digest. They do not record credentials or mix the other backends' model settings into the effective role profile. Responses distinguish requested identity from provider-reported identity when available. Codex's requested model remains labeled requested: the current adapter does not establish a resolved model from its event stream.

OpenRouter responses retain both the initial and local schema-repair request/response when repair succeeds, with provider identities and native usage in the execution trace. Reported charges and token totals include both returned responses; missing charges remain unknown. Transport retries and provider-internal transformations may still be unreported. Claude Agent costs are SDK estimates, not per-call subscription cash expenditure. A completely failed completion can still lack a full response/charge record; evaluation must report that limitation. No historical cost logs are rewritten.

The Claude Agent adapter uses a fresh temporary working directory for each attempt, with its tool and settings restrictions, so the repository is not exposed as task context. The Codex subscription/auth isolation is covered by its own regression tests.

## Comparisons

Backend and model comparison campaigns are paused. The single frontier researcher
([single-frontier.md](single-frontier.md)) is the default. A new comparison needs the
maintainer's approval (see the repository `AGENTS.md`).
