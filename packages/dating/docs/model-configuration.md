# Dating model configuration and comparison policy

The active configuration is [packages/dating/llm.config.json](../llm.config.json), schema version 2. It belongs to dating and is neutral about backend. The former repository-root `openrouter.config.json` has been removed from active use; its exact bytes are preserved in [the historical archive](archive/llm-config-2026-09-20.json). Model assignments, effort, transport policies and existing backend defaults were preserved during migration. This preservation establishes a baseline, not an endorsement.

The structure separates three concerns:

- `default_backend` and `roles.<role>.backend` select a backend; `DATING_LLM_BACKEND` overrides per-role selection.
- `roles.<role>.backends.<backend>` contains that backend's model, effort/reasoning and supported generation/routing settings. `anthropic` and `claude-agent` have independent role profiles.
- `backends.<backend>` contains connection/runtime policies. OpenRouter URL, headers, provider defaults, retries and plugins live under `backends.openrouter`; subscription limit policy lives under `backends.claude-agent`; executable/auth-store/timeout settings live under `backends.codex`.

Set `DATING_LLM_CONFIG` to a complete alternate v2 JSON file for an experiment. Relative paths resolve from the repository root. For example, after creating a reviewed trial configuration:

```sh
DATING_LLM_CONFIG=data/workspaces/dating/_research/evaluation/trial.json DATING_LLM_BACKEND=codex uv run --frozen dating llm-check codex
```

This example performs a live subscription-backed probe if run. It is not required to validate the config migration, which has offline tests. `llm-check` tests structured transport on one constructed case; it does not evaluate dating quality. Backend instances are cached by both name and config digest, with a copied configuration, so sequential experiments cannot accidentally reuse another cell's settings. Missing backend/model selections fail explicitly. Old dictionary shapes remain accepted only by programmatic compatibility accessors; on-disk configs require v2.

Request snapshots retain the selected backend's role settings, bounded runtime policy and whole-config digest. They do not record credentials or mix the other backends' model settings into the effective role profile. Responses distinguish requested identity from provider-reported identity when available. Codex's requested model remains labeled requested: the current adapter does not establish a resolved model from its event stream.

OpenRouter responses now retain both the initial and local schema-repair request/response when repair succeeds, with provider identities and native usage in the execution trace. Reported charges and token totals include both returned responses; missing charges remain unknown. Transport retries and provider-internal transformations may still be unreported. Claude Agent costs are SDK estimates, not per-call subscription cash expenditure. A completely failed completion can still lack a full response/charge record; evaluation must report that limitation. No historical cost logs are rewritten.

The Claude Agent adapter now uses a fresh temporary working directory for each attempt, with its existing tool and settings restrictions. This removes a stale pre-reorganization `dating/cache` path without exposing the repository as task context. The Codex subscription/auth isolation behavior is preserved and remains covered by its own regression tests.

## Required live evaluation

The [backend evaluation plan](../benchmarks/backend-evaluation-plan.json) requires **OpenRouter, Claude Agent SDK and Codex** across screening, extraction and synthesis. Include the direct Anthropic API as an additional comparison where accessible; report missing access explicitly. Test more than one appropriate capability/effort configuration per required backend rather than equating backend with model strength. A backend cannot be declared tested merely because another provider hosts a similarly named model.

Treat every OpenRouter model/provider/settings choice as adjustable. Recheck current model availability, endpoint capabilities, account/privacy constraints and charges before selecting cells. Isolate model/effort and provider effects where possible; test operational routing/fallbacks separately. Existing price-sorted routing, quantization preferences, response-healing plugins, output caps and batch policy are hypotheses to evaluate, not fixed experimental facts.

Use frozen complete inputs for role comparisons, then end-to-end family tasks for retrieval/context performance. Keep exposed pilot cases in development. Freeze configs, reference clusters, metrics, matching rules, repetitions, budgets and targets before the scored held-out run. Record actual missing/failed cells rather than substituting another backend silently. Capability probes, development agreement and held-out semantic performance must remain separately labeled. The plan is design work; no new live model evaluation has been run by this migration.


The subsequent [live evaluation](live-evaluation.md) completed development comparisons and production-schema probes on all three required backends. Its complete frozen configurations are retained with each run. They explicitly disable fallback and pin tested OpenRouter endpoints; production defaults remain provisional. Baseline versus stronger settings did not establish a general upgrade benefit. Native-time schema repair, rather than model replacement, resolved the demonstrated production failure.
