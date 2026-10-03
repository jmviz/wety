# Single frontier researcher

This proposal-only variant uses one model in one continuing tool-enabled session.
It does discovery, retrieval, interpretation, incremental writing and one self-audit.
It does not call the old screen/extract/review workers or launch subagents. The
shared [research brief](single-frontier-prompt.md) and tools are the same across
transports; native agent runtimes and context management still differ.

For current results and source-level corrections, see the
[21 September stocktake](stocktake-2026-09-21.md). The two original Japonic reports
are retained model outputs; the [correction packet](../../../data/workspaces/dating/_research/japonic-stocktake-2026-09-21/README.md)
qualifies their use. They are not ready-made imports.
The subsequent [Turkic run](turkic-frontier.md) records the first expansion with the
updated brief, its bounded coordinator review and a specific remaining access gap.

## Start from the repository root

```sh
.venv/bin/python -m harness.frontier_research prepare \
  data/workspaces/dating/_research/my-japonic-run --family Japonic
.venv/bin/python -m harness.frontier_research run \
  data/workspaces/dating/_research/my-japonic-run
.venv/bin/python -m harness.frontier_research check \
  data/workspaces/dating/_research/my-japonic-run
```

Preparation makes no model calls. It freezes the profile, prompt, scientific
contract, limits and retained driver/tool bytes. The first dispatch is once-only.
Claude subscription usage-limit pauses can resume the exact saved session; other
errors cannot silently redispatch. Partial work is retained, with no fresh-session
restart, backend/model fallback or hidden reviewer chain. `STOP` in that directory
prevents further tool actions (an in-flight model/request can still finish). An
outer process deadline terminates the worker process group and descendants.

Profiles live in `single_researcher` within `packages/dating/llm.config.json`, not
in a separate OpenRouter-named file. `DATING_LLM_CONFIG` selects a complete alternate
configuration. Select `--profile NAME` at preparation; legacy role overrides and
`DATING_LLM_BACKEND` do not silently change this explicit choice.

| Profile | Access | Model | Initial effort |
| --- | --- | --- | --- |
| `codex-astra` **default** | Codex CLI, personal ChatGPT subscription | `gpt-6-astra` | `high`, standard service tier |
| `claude-opus` | Claude Agent SDK, personal Claude subscription | `claude-opus-5` | `high`, adaptive thinking |
| `openrouter-astra` | Explicitly selected paid OpenRouter API | `openai/gpt-6-astra` | `high` |
| `openrouter-opus` | Explicitly selected paid OpenRouter API | `anthropic/claude-opus-5` | `high` |

These are known named candidates, not a claim that every account has access or
that Opus 5 is the newest/strongest available Claude. Change model/provider/effort
explicitly in a new configuration when justified. No alias silently upgrades a run.
Unsupported combinations fail rather than dropping reasoning settings.

## Why these thinking settings

**Astra high:** source interpretation combines uncertain events, borrowed dates,
priors, sample definitions and conditional models. High is a reasonable initial
choice when one model must own the whole dossier and correct its own work. OpenAI
recommends comparing medium/high for complex research, reserving xhigh for measured
benefit. We choose high provisionally; medium is the first cost-reduction candidate
if source fidelity holds. No effort sweep is a prerequisite for useful research.
[Official reasoning guidance](https://developers.openai.com/api/docs/guides/reasoning).

**Opus 5 high with adaptive thinking:** this follows the model's documented starting
point for complex work; adaptive thinking permits simpler actions to require less
reasoning. Medium is a future economy option when observed fidelity supports it.
Max spends unconstrained reasoning and xhigh targets more demanding agentic work;
neither has demonstrated a benefit in this dating workflow.
[Anthropic effort guidance](https://platform.claude.com/docs/en/build-with-claude/effort).

**OpenRouter:** effort belongs to the chosen model, not the billing backend. Keep
the same initial choice for the same Astra/Opus model. The gateway translates
reasoning controls; identical labels do not imply equal token budgets between
models/providers. Require parameter support, disable provider fallback, retain the
reported provider/model and preserve reasoning blocks across tool calls. There is
no global temperature override, output repair or model fallback. A new model needs
an explicit supported reasoning choice; do not copy a budget blindly.
[OpenRouter reasoning](https://openrouter.ai/docs/guides/best-practices/reasoning-tokens).

Keep effort constant within this first session. Dynamic effort routing would add
another unmeasured controller and can alter caching. Ask for concise operational
steps and spend reasoning on source ambiguities. Standard speed, finite source
scope, incremental notes and a single correction pass control cost more directly
than assuming a low setting makes a repeated failed workflow cheap.

## Authentication and accounting

Codex requires `codex login status` to confirm ChatGPT authentication. API-key and
endpoint overrides are removed, user/project configurations are ignored, and the
OpenAI provider plus ChatGPT login method are explicit. No OpenRouter key is needed
for this profile. It uses the existing configured credential store without copying
authentication tokens. [Codex authentication](https://learn.chatgpt.com/docs/auth).

Claude accepts the repository's existing subscription OAuth setup or a verified
`claude.ai` CLI login. API keys, alternate endpoints and cloud-provider overrides
are removed from the isolated worker. Personal settings, hooks/plugins and other
model agents are not enabled. Subscription access currently covers Agent SDK use:
the **June 15 update at the top** of Anthropic's page pauses the older billing
change described below it. Do not cite the obsolete lower section as current policy.
[Claude subscription SDK policy](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan).
Availability and account-side extra-usage settings remain provider-controlled;
the runner never switches to an API key or a paid fallback itself.

OpenRouter requires `OPENROUTER_API_KEY` only for an OpenRouter profile. Each request
and response is retained; cost is accumulated from provider usage, not inferred
from subscription counters. Missing cost stops further paid calls. The $15 reported
cost stop is checked **between calls**, not a hard invoice cap; a call can overshoot
and an interrupted call may have unknown cost. Per-response output and overall
turn/time bounds supplement it. There are no runner-level retries; native clients
may apply their own transport retries.

Codex token counts and Claude SDK usage/model usage are retained when returned.
Subscription `cost_usd` stays null; Claude's nominal SDK estimate is labeled
separately. If no terminal usage arrives, it is unknown, never zero. A task's worker
usage excludes the coordinating task that implemented or inspected it. Weekly
account usage does not supply reliable per-family attribution.

## Claude usage-limit halt and resume

Use the same research budget while allowing a reset wait:

```sh
.venv/bin/python -m harness.frontier_research prepare \
  data/workspaces/dating/_research/my-claude-run --family Japonic --profile claude-opus
.venv/bin/python -m harness.frontier_research run \
  data/workspaces/dating/_research/my-claude-run --wait-for-reset
```

The runner captures the session ID from initialization, rate-limit events or an
error result, rather than depending on successful completion. Allowed/warning
events do not halt research. A rejected usage event, recognized 429/error result,
or explicit usage-limit exception ends the SDK stream and records
`paused_rate_limit`. Overage use also stops this subscription-only run. API fallback
and fresh-session restart remain disabled.

With `--wait-for-reset`, the local process waits without model calls until the
reported reset plus 15 seconds, for at most six hours and two continuations. It
then uses the SDK's exact `resume` session ID and a continuation prompt, not the
original research prompt. The SDK persists history on local disk; keep its session
storage together with the workspace. A missing session, unknown reset, exhausted
budget or unexpected failure is not repaired by creating another research session.
[SDK session persistence and resume](https://code.claude.com/docs/en/agent-sdk/sessions).

`wait.json` describes the waiting state. This is a running local process, not a
remote scheduler: it must remain alive. After a process shutdown, inspect `run.json`
and `claude-state.json`, and resume an eligible pause after the recorded reset:

```sh
.venv/bin/python -m harness.frontier_research resume \
  data/workspaces/dating/_research/my-claude-run --wait-for-reset
```

The active research/delivery clock stops during reset waits. It does not replenish
used research seconds, model turns, searches, acquisitions or tool actions. Each
attempt gets its own receipt, dispatch, usage and worker log; Claude events append
with attempt numbers. Existing checkpointed evidence/output hashes, the frozen
prompt/profile and implementation must still match. `STOP` cancels waiting and
further tool actions. No automatic resume is allowed after ordinary failures or a
completed dossier.

Turn accounting is conservative when rejection prevents a terminal result. Missing
usage remains unknown, and terminal SDK usage from resumed attempts is retained
separately rather than assumed additive. A natural live rejection/resumption and a
simulated regression test must be reported separately; do not consume extra
allowance merely to force a rejection after useful work completes.

## Research tools and limits

Default limits are 20 minutes acquisition/research plus 5 minutes delivery, 8 search
calls, 12 acquisition attempts, 120 tool actions, and 64 model turns for Claude and
OpenRouter. Codex has the shared action/time bounds, not a claimed native turn cap.
Preparation supports explicit time overrides up to 45+15 minutes. Failed network
attempts consume allowances. Avoid more than two alternate routes per blocked
source; that per-source policy is in the prompt, whereas aggregate caps are enforced.

The shared tool exposes Crossref, OpenAlex and Europe PMC discovery; exact-byte
acquisition through the shared provenance utility; bounded text windows with
character offsets; rendered retained PDF pages; four dossier outputs; and checks.
This is a scholarly-index search baseline, **not unrestricted web browsing**.
Index access/rate limits can leave gaps. HTML figure images, OCR, archive extraction
and arbitrary computation are not currently provided; report those needs explicitly.
PDF text extraction does not imply that figures, tables or supplements were inspected.

Original HTTP bytes and acquisition receipts use the existing artifact registry.
Parsed text and PDF renders retain hashes in the research workspace. Model output
can only write the four named dossier files, with prior revisions preserved. It
cannot edit scientific inputs or products, execute shell commands, read arbitrary
private files, spawn agents, or import claims. MCP exposes these tools to the native
agents; the OpenRouter loop exposes the same function and converts page images to
multimodal messages. Public scientific source content is evidence, not instructions.

The check reports missing files, invalid JSON, assertion IDs, known sources, exact
quote matches, text integrity and optional existing compact-schema errors. It does
**not** establish semantic/number correctness, exhaustive coverage, complete
self-review or publication eligibility. Flexible proposals deliberately avoid a
large output schema that would recreate the prior failure mode. Unsupported visual
or structural evidence remains a located gap, not a fabricated exact quote.

Useful partial dossiers are valid research outcomes. A transport success or empty
structural error list is not a measured improvement in historical-linguistic quality.
Compare delivered evidence and later correction effort before adopting this mode
for broad expansion. No human reviewer is required to finish a proposal dossier.

## First live subscription run: 20 September 2026

[The Japonic dossier](../../../data/workspaces/dating/_research/single-frontier-japonic-2026-09-20-v2/report.md)
completed in **849.8 seconds (14.2 minutes)** using Codex/Astra high on the personal
ChatGPT subscription. It produced 20 proposals from four substantive works, a
source inventory and one correlated self-audit. Six searches, twelve acquisition
attempts, fifteen text-window reads and five PDF-page inspections are retained.
Three acquisition failures and metadata/empty responses are recorded as gaps.
The final structural/quote checks pass, and the self-audit covers all twenty IDs
and identifies the final assertion-file hash correctly.

The run used **1,864,485 input tokens including 1,740,928 cache hits**, and **25,237
output tokens including 4,841 reasoning tokens**. The illustrative standard API
equivalent is **$4.24**, assuming short-context requests; aggregate Codex telemetry
cannot establish every request's context tier. This is not an invoice or weekly
allowance percentage, and excludes this coordinating/development task. The initial
permission-blocked launch adds about **$0.14** equivalent and remains separately
retained. A missing per-tool MCP permission setting was corrected before the
successful run; no evidence was acquired in the blocked attempt. The explicit
permission now covers only the bounded local research tool.

[Accounting and output hashes](../../../data/workspaces/dating/_research/frontier-strategy-2026-09-20/single-frontier-result.json)
bind the assessment to the completed files. The model-written report could not see
terminal usage while it was running; `usage.json` is the authoritative later runtime
record. Its generic frozen 64-turn field is not an enforced Codex turn limit.

This demonstrates operational usefulness, not a measured accuracy advantage over
the previous harness or a validated high-versus-medium comparison. The family,
source set and flexible proposal output differ from earlier trials. Remaining
source gaps, incomplete sampled-taxon lists, unknown epochs and scenario-specific
outputs remain explicit; no claims are imported. Claude and OpenRouter transports
have offline coverage, but were not newly live-tested in this continuation.

Validation: **236 dating tests**, the managed rebuild and all three publication
reproduction checks pass. All **3,575 claims and 8,625 code views remain byte-identical**,
with `spoken` null throughout. Next use this mode for bounded proposal dossiers;
resolve named evidence gaps locally and measure correction effort before making
broader accuracy or effort-efficiency claims. Do not restart a family just to remove
honest unknowns.

## Live Claude limit test: halt and resume verified

The first Claude-subscription Japonic run uses Opus 5/high, without Astra's dossier.
The user reported 83% five-hour utilization before launch. SDK events subsequently
reported 84%, 90% and a **rejected 102%** five-hour window. These are account-wide
observations, not an attribution of every percentage point to this run.

The runner halted after **401.9 active seconds**, retaining **22 unfinished proposals**
and the exact SDK session. The SDK session was independently found in its local
session store. At that point three deliverables and one quote correction remained
pending, and no successful terminal usage had been received.

The local `--wait-for-reset` process resumed the **same session** at **20 September
2026, 14:40:15 America/New_York** (the reported reset plus 15 seconds), and completed
at **14:46:46**. It retained 798.1 seconds of total active allowance and 498.1 seconds
of research allowance, without replenishing the original budgets. Both attempts
together used **793.5 active seconds (13.2 minutes)**; elapsed time including the
wait was 86.5 minutes. No model calls occurred while waiting.

All four deliverables are present. The final **22 proposals** pass structural and
exact-quote checks, with a self-audit entry for each ID; the pending quote issue was
corrected in the continued session. All checkpoint hashes and both publication
baseline hashes match. These checks do not certify semantic accuracy or coverage.
No proposals were imported. The resumed terminal SDK result reports a nominal
**$3.43** estimate under subscription billing; missing first-attempt terminal usage
and uncertain cumulative scope prevent treating it as a full-run cost or invoice.

[Halt assessment](../../../data/workspaces/dating/_research/single-frontier-japonic-claude-2026-09-20-v2/halt-test-assessment.json),
[run state](../../../data/workspaces/dating/_research/single-frontier-japonic-claude-2026-09-20-v2/run.json),
and [wait state](../../../data/workspaces/dating/_research/single-frontier-japonic-claude-2026-09-20-v2/wait.json)
retain the evidence. The [completion assessment](../../../data/workspaces/dating/_research/single-frontier-japonic-claude-2026-09-20-v2/completion-test-assessment.json)
records the session continuity, remaining budgets, cumulative tool counters and
output verification. Preserve the frozen implementation and attempt receipts.

The frozen run reserved turns conservatively: multiple SDK assistant frames can
represent one API message. Do not count those frames as separate billed requests.
The 21 September driver now deduplicates stable assistant message IDs across frames
and resumes; unidentified frames and terminal turn counts remain conservative.
Regression tests cover the change. This completed experiment's frozen implementation
and counters remain intact; its allowances were not retroactively replenished.

All **242 dating tests** and the managed rebuild/reproduction checks pass; published
claims and chronology views remain unchanged. The natural live continuation now
also verifies the halt/resume path. A controlled accuracy comparison remains open;
different source selections do not support ranking Claude against Astra.
