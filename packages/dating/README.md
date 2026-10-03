# Chronological evidence for languages — version 2

This project records **what a source dates, how it dates it, and the conditions under which that date is interpretable**. It does not assign every language a birth and death date. The previous pipeline conflated ancestral splits, documentary samples, attestations and language stages; its adopted outputs have been archived and replaced.

A language is a historically continuous, socially delimited object. A named stage is often a scholarly convention; an attestation dates surviving documentation; a phylogenetic node dates divergence among the particular sampled lineages under a model. These are different estimands. Neither the comparative method nor a lexical clock makes a parent-family crown date the beginning of every descendant language. A reconstructed stage need not occupy the interval between two estimated tree nodes.

## Scientific contract

* `subject` identifies a catalog entity, Wiktionary stage, documentary sample, reconstructed stage, population, or exact sampled clade. Catalog links are proposed correspondences. `exact`, `subset`, `superset`, `overlap`, and `unresolved` are distinct mapping relations. Source taxon lists survive ingestion.
* `event`, `role`, and `method` are separate. Observations, conventional periods, calibration priors, posterior estimates, secondary reports and archaeological/linguistic hypotheses cannot silently substitute for one another.
* `time` distinguishes points, uncertainty about an event, durations, bounds and distributions. Means, medians, HPDs and equal-tailed intervals retain their labels. Original quantities are retained. Calendar conversion is programmatic; BCE has no historical year zero. Source-stated BP epochs and a labeled conventional 1950 interpretation are separate. Calendar/model years and uncalibrated radiocarbon ages are different scales; unknown scale prevents conversion. See [time interpretation](docs/time-interpretation.md), including typed century/millennium expressions and historical compatibility limits.
* `study_id`, `analysis_id`, and `result_id` prevent a paper, its supplement, D-PLACE copy and repository deposit from becoming independent votes. Alternative clock/root/calibration scenarios remain separate analyses. No averaging, newest-paper ranking or source-count confidence determines a default.
* `assessment` separates textual fidelity, semantic interpretation and entity mapping. “Supported” means the recorded assertion satisfies those checks, **not that the historical hypothesis is true**. Source-derived mechanical evidence and editorial assessments identify their basis and limitations.
* New model extraction is a proposal. Verbatim spans, source-native numbers, immutable run histories and content digests make it auditable. A model review cannot overwrite dates. An explicit editorial decision names an eligible claim, reviewer, rationale and current evidence digest.
* `spoken` is always `null` in the compatibility view. Only supported, exact `wikt:<code>` evidence enters that code's `chronology`. Catalog and ancestor evidence are contextual references, never fallback lifespans. Unknown is a valid result.

The [catalog cross-reference audit](../catalogs/critique.md) repairs the upstream identity layer as well. Dating now validates one committed catalog generation, preserves source-specific IE-CoR samples, and treats family/proto associations and catalog links as context. Rebuild the catalog and IE-CoR proposal receipt before dating after changing their sources.

See [the follow-up critique audit](docs/critique-review.md) for the current finding-by-finding disposition, [the next-step agenda](docs/next-steps.md) for the research sequence, and [benchmark documentation](benchmarks/README.md) for validation limits. The [original overhaul audit](docs/overhaul.md) describes the first implementation pass.

## Reproduce locally

For the next family, use the [single frontier researcher](docs/single-frontier.md):
one continuing tool-enabled model, defaulting to **Codex/Astra on your ChatGPT
subscription**, with Claude subscription and explicit OpenRouter API profiles. The [cost/quality assessment](docs/frontier-strategy.md)
explains the shift from repeated multi-model trials to one bounded research task.
The [21 September stocktake](docs/stocktake-2026-09-21.md) records both completed
subscription runs, their bounded source-context corrections, current operating
decisions and remaining limits. Start there rather than replaying older experiment
agendas. Completed model output is a proposal, even when its self-check passes.

The [bounded autonomous queue](docs/autonomous-research.md) now includes optional
[native-time repair](docs/native-repair.md), explicit prior/duration/offset proposals,
and source-context review. These remain research proposals; see the current
[next steps](docs/next-steps.md) for measured backend limitations and the next run.

Python 3.12+; dependencies are pinned in the repository root `uv.lock`. Run all commands from the repository root:

```sh
uv sync --frozen
uv run --frozen wety-data run dating
.venv/bin/python -m unittest discover -s packages/dating/tests -v
.venv/bin/python -m harness.cli benchmark
.venv/bin/python -m harness.schema
.venv/bin/python -m harness.cli bootstrap dplace
.venv/bin/python -m harness.cli synthesize
.venv/bin/python -m harness.cli validate
```

`bootstrap dplace` now ingests **only explicitly declared analysis manifests**, including non-D-PLACE deposits. It never infers units from root magnitude, posterior status from a file suffix/tree count, or a burn-in fraction. The three checked-in manifests refer to the existing local IE/ST deposits and a pinned Phlorest Uralic fixture. A missing or changed file fails reproducibility checks; its expected path, SHA-256 and retrieval URL are in the manifest. Do not invent a replacement epoch to make ingestion succeed.

The tests and benchmark are offline and make no model calls. `benchmark --predictions FILE` scores separately supplied semantic predictions against the small development set. Without those predictions, extraction accuracy is explicitly **not measured**.

## LLM backends

[`packages/dating/llm.config.json`](llm.config.json) configures `openrouter`, `anthropic`, `claude-agent`, and
`codex`. Set `roles.<role>.backend` to choose per role, or use
`DATING_LLM_BACKEND=codex` to override every role. Existing defaults are unchanged.
Use `DATING_LLM_CONFIG` for a complete alternate experiment configuration. See the [configuration layout and required backend evaluation](docs/model-configuration.md).
The legacy stateless role backends use the same application prompts, strict JSON
schemas, and Pydantic validation. The single frontier variant instead uses a continuing
research session, shared tools and incremental proposals; see its separate instructions. Model-role assignments remain provisional. The [first live comparison](docs/live-evaluation.md) tests all three required backends and records a verified production-schema fix. The [full-source follow-up](docs/full-source-evaluation.md) exposes additional omissions and semantic errors on a Bantu article/critique cluster; general held-out accuracy remains unmeasured.

The `codex` option runs the supported Codex CLI non-interactive interface with
your **ChatGPT subscription login**. It consumes your plan's Codex allowance;
it is not general OpenAI API credit. Available models and usage limits depend on
your account. See OpenAI's [authentication documentation](https://learn.chatgpt.com/docs/auth),
[Codex SDK](https://learn.chatgpt.com/docs/codex-sdk), and
[non-interactive mode](https://learn.chatgpt.com/docs/non-interactive-mode).

Install a current Codex CLI (with `exec --ignore-user-config`, `--ephemeral`,
and `--output-schema` support), then sign in and run a small probe:

```sh
npm install -g @openai/codex
codex login                       # browser sign-in with ChatGPT
codex login status                # must say Logged in using ChatGPT
uv run --frozen dating llm-check codex
DATING_LLM_BACKEND=codex uv run --frozen dating extract indo1319
```

On a headless host, use `codex login --device-auth`. The harness leaves OAuth
storage and refresh to Codex; no token is copied into `.env`. `CODEX_HOME` is
respected. If your login uses the OS keychain, match it with
`backends.codex.auth_credentials_store: "keyring"` (or `"auto"`) in the harness config;
the default is `"file"`. `backends.codex.executable` can name an explicit CLI path.

`roles.<role>.backends.codex` supplies the model and effort; the initial settings use
`gpt-5.6-terra` with low/medium/high effort for screen/extract/synthesize.
`backends.codex.timeout_seconds` bounds each completion (default 600 seconds). API-key
logins are rejected, API-key environment overrides are removed from the child
process, and subscription limits/errors stop that call with no paid fallback.
Resume the harness after the limit resets. Batch submission remains OpenRouter-only.

Each call starts an ephemeral thread in a temporary directory, replaces Codex's
default coding instructions with the role's system prompt, ignores personal CLI
configuration and project instructions, and disables shell, web, apps, plugins,
hooks, browser/computer use and subagents. The sandbox is read-only. Codex still
applies its platform instructions and account data policies; OpenRouter's ZDR
settings do not apply to this backend. Role `max_tokens` is not a Codex CLI
output cap; its supported controls here are model, effort and timeout.

Logs retain the thread ID and input/cached/output token counts. `cost_usd` is
unknown (`null`), not a claim of zero subscription cost. The model field records
the requested model because `codex exec` does not report a resolved model version.

## Artifact layout and migration

| Path | Meaning |
|---|---|
| `data/archive/dating/pre_v2/` | Original artifact snapshot with file hashes; not current results |
| `data/workspaces/dating/_research/graph.json` | Curated study/analysis/data/calibration dependencies and critique/reply relations; absent edges do not imply independence |
| `data/workspaces/dating/_research/packets/*/` | Bounded editorial source packets with hashes, spans, review scope and importable assertions |
| `data/workspaces/dating/_research/pilot_sources.json` | Named minimum source roster for completeness auditing |
| `data/products/dating/pilot_coverage.json` | Per-source retrieval, cache integrity and extraction state, including missing inputs |
| `data/workspaces/dating/_analyses/*.json` | Explicit analysis identity, file hash, taxa, units, epoch, sampling and methodological limitations |
| `data/workspaces/dating/_evidence/*.json` | Typed v2 source evidence; legacy family `claims.jsonl` files remain conservative ingress |
| `data/workspaces/dating/_constraints/relations.json` | Optional explicit relative event relations with evidence references; strict cycles rejected |
| `data/workspaces/dating/_decisions/decisions.json` | Reviewed claim selections; no arbitrary numerical overrides |
| `data/workspaces/dating/_requests/` | Immutable application prompts, schemas, frozen extraction context and linked model responses; durable local evidence |
| `data/workspaces/dating/_reviews/` | Immutable model proposals; never adopted automatically |
| `data/artifacts/dating-generations/<digest>/bundle.json` | Complete, validated, content-addressed generation |
| `data/products/dating/CURRENT.json` | Atomic commit pointer: authoritative readers start here |
| `data/products/dating/evidence.json` | Compatibility evidence mirror, including source-native results and uncertainty |
| `data/products/dating/wiktionary_dates.{json,csv}` | Safe code-specific view; CSV retains chronology as JSON |
| `data/products/dating/review_queue.json` | Every candidate/quarantined claim, including previously omitted low-confidence cases |
| `data/products/dating/benchmark.json` | Reproduction checks and explicitly measured/unmeasured metrics |
| `data/workspaces/dating/retrieved/raw/`, `data/workspaces/dating/retrieved/texts/` | Original bytes and parsed UTF-8 text under their own hashes, with retrieval/parser metadata |

Use `harness.registry.current_bundle()` for consistent reads. Mirrors are individually atomic, but a reader opening several mirrors during publication could mix generations. `validate` checks them against the committed bundle and detects changed evidence, source metadata, code, taxonomy and analysis manifests. Republish after changes; stale decisions are withheld. Claims are preserved verbatim during legacy migration and require review; migration is not retroactive endorsement.

## Research and editorial workflow

```sh
.venv/bin/python -m harness.cli discover indo1319 --screen llm
.venv/bin/python -m harness.cli retrieve indo1319
.venv/bin/python -m harness.cli extract indo1319
.venv/bin/python -m harness.cli synthesize
.venv/bin/python -m harness.cli llm-synthesize indo1319
.venv/bin/python -m harness.cli import-evidence reviewed-claims.json
.venv/bin/python -m harness.cli decide reviewed-decision.json
.venv/bin/python -m harness.cli synthesize
.venv/bin/python -m harness.cli validate
```

Discovery includes subgroup names, methodological critiques, calibration and relative-chronology work. Whole-family papers can mention any descendant, regardless of task partitioning. Search and model screening remain fallible: inspect excluded papers and coverage before interpreting silence. The policy version invalidates old screen exclusions on the next screening run.

Retrieval distinguishes likely full texts from landing pages, partial texts, snippets and abstracts; even `fulltext_candidate` is a diagnostic, not a guarantee of document identity or completeness. Follow publisher full-text links, inspect the document title, and retain licensing/access metadata. `repair-pilot-sources` recovers five identified primary texts (Honkola, Maurits, Grünthal, Chang, Sagart); it does not fabricate successful extractions from them.

An import is a JSON list (or an object with `claims` and `relations`, to validate relative evidence together) conforming to `packages/dating/schemas/claim.schema.json`, validated with Pydantic **and** cross-record checks. Changed evidence needs a new ID with the previous assertion referenced in provenance. Supported claims need exact subject scope, checked semantics/mapping, named reviewer and rationale, reproducible temporal normalization and exact text spans bound to the registered source, or reproduced mechanical provenance. Relative-only evidence may retain unknown absolute time when backed by a structured relation. Source-specific subject exactness and catalog correspondence are separate fields. This is an evidence review workflow, not a requirement to hire an external specialist. A contributor may perform and document a limited review; broader unresolved judgments remain candidates.

A decision conforms to `packages/dating/schemas/decision.schema.json`: `subject_id`, `event`, `claim_id`, `evidence_digest`, `reviewer`, `rationale`, optional `dependencies`. Obtain the current digest from the bundle. The decision selects evidence; it cannot change its numerical content. Calibration priors and registered methodological demonstrations/sensitivity analyses are contextual evidence, not historical defaults. All alternatives remain accessible.

For posterior-tree ingestion, supply explicit units/epoch, prior/result role, exact taxa, tip ages, justified burn-in and trace-linked convergence/ESS metadata. The importer checks the referenced trace hash and declared diagnostics; **it does not itself calculate convergence diagnostics**. It retains all post-burn-in ages with shared sample indices and tree names, clade support, conditional age distributions and alternative parent sets. Summary trees explicitly lack joint samples and cannot establish the absence of topology uncertainty. Further inference requires independent chain diagnostics and model adequacy assessment.

For etymology, inheritance and borrowing should constrain source-specific events and contact sequences. Do not reject an etymological edge because its endpoints lack adopted whole-language lifespans. Relative relations are assertions requiring their own evidence, not inferred automatically from the catalog tree. Sign languages, mixed languages, creoles, isolates and contested groupings require appropriate historical models; catalog membership alone supplies none.

Extraction inputs now inventory every known candidate attachment, including retrieval skips, unreadable files, tree files outside the text prompt and character-budget omissions/truncation. Input coverage is retained with extraction records and shown to the model, separately from quotable evidence. [Request snapshots](docs/model-request-replay.md) preserve the exact submitted prompts, schemas and extraction context; ingestion uses the frozen evidence. Structured gaps/conflicts survive zero-claim results, and synthesis reports input and response omissions without merging subjects through catalog IDs. This describes supplied candidates, not complete upstream studies. See the [harness improvement and model-role evaluation recommendations](docs/harness-recommendations.md) for the operational endpoint of the review agenda.

The optional [section workflow](docs/section-extraction.md) partitions retained text
into bounded source windows with exact offsets, headings/context and omissions.
It retains valid proposals from partial responses and supports one bounded repair
per failed section under the same ledger. These remain proposals; ordinary failed
extractions receive recovery sidecars without replacing successful historical output.

## Current scientific limits

The rebuilt pilots add 225 exact source-clade results across the three deposited summaries. Source-native root medians/means and HPDs reproduce the papers within their reporting precision. The deposited manifests still retain unverified BP epochs. Subsequent article reviews establish epochs for specific versions but do not silently renormalize those earlier claims. Those clades have not been adjudicated as equivalent to Wiktionary stages. Consequently, they do not produce calendar lifespans or code-specific dates.

The legacy corpus remains available for review. No all-family specialist gold standard exists here, and none is required to operate the registry honestly. The substitute is a layered benchmark: published-result reproduction, pinned curated data, twenty semantic development cases (source-grounded paraphrases plus labeled constructed controls), and adversarial software regressions. It establishes mechanical fidelity and safety properties, not extraction accuracy across the literature or the true age of a language family. The next scientific work is staged evidence review, calibration/contact/root sensitivity analysis and independent evaluation, not maximizing populated date fields.

The follow-up audit also repairs legacy text-hash keys without claiming missing raw provenance. Retrieval does not imply completed extraction: the recovered central Uralic texts now have a [bounded source review](docs/uralic-packet.md), with 13 assertions and five supported source records, and a completed separate-session model review; exhaustive extraction and project-held-out evaluation remain open. Packet validation (`python -m harness.packets PATH --require-imported`) checks retained evidence and imported relations; pilot coverage reports editorial review separately from model extraction. Batch outputs with missing or stale request fingerprints must be resubmitted. The research graph records dependence and debate; it neither computes independence scores nor decides which historical account is true.

The [Uralic supplement follow-up](docs/uralic-supplements.md) adds recovered source versions, exact methodological findings and narrow numerical checks (`python -m harness.uralic_checks`). It preserves loanword disagreements and existing claims. These checks are part of the dating regression suite; they do not execute the original R environment or select a historical default.

The [“hundred” contact case](docs/uralic-hundred.md) demonstrates separate conditional event graphs bound to source findings. Contact-case packets cannot export unconditional claims or relations. Its alternatives remain unresolved, with no absolute dates. Follow the [review and evaluation protocol](docs/review-protocol.md) before labeling new material held out; all existing fixtures and packets are development material, and independent evaluation remains unmeasured.

Human review is optional. A fresh model session may perform a source-first second review; a documented adversarial self-audit is the fallback and does not block the next pilot. See the [model-review policy](docs/model-review.md) and [ready-to-use handoff](docs/model-review-prompt.md). Shared-model agreement is not independent historical corroboration, and all normal evidence/decision checks still apply.

The [completed Uralic review integration](../../data/workspaces/dating/_research/packets/uralic-review-integration-2026-09-20/README.md) preserves corrections, frozen-review receipts and shared-model limitations. The [first Indo-European packet](docs/indo-european-packet.md) records 23 source/scenario findings, verified article epochs and explicit retrieval gaps. It adds no date claims or historical defaults.

The [Sino-Tibetan packet](docs/sino-tibetan-packet.md) adds 31 findings and recovers Zhang 2020's corrected posterior-file deposit. `python -m harness.sino_tibetan_checks` reproduces its published root mean and HPD with explicit state selection, while retaining calibration/configuration discrepancies and withholding a production posterior import.

The opt-in [compact assertion and semantic-review workflow](docs/compact-trial.md)
now has a bounded Dravidian development trial across all three backends. It retains
partial proposals, source-context reviews, provider failures, deadline/cost stops
and incomplete coverage without importing scientific evidence. See the
[operating recommendations](docs/harness-recommendations.md#operating-package-after-the-dravidian-trial)
for the concrete next autonomous discovery milestone and measured limitations.

The [autonomous research queue](docs/autonomous-research.md) now runs bounded discovery,
model-selected acquisition, section extraction and source-context review from a frozen
brief. Its first Austronesian run completed with explicit failures and coverage gaps.
It remains proposal-only; see the report for the six-call transport controls, reviewer
limitations, resume guarantees and the next fidelity improvements.
