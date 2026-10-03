# wety language-dating harness

Implements [docs/language-dating-plan.md](../docs/language-dating-plan.md): a dated
index of every language wety can name, built from existing datasets first and
from the literature second, in a form that can be re-run, audited, and diffed.

```
dating/
  harness/            the code (Python 3.12+, uv project)
    schema.py         Claim / Source / Languoid models → data/schema/*.schema.json
    time_utils.py     calendar convention + parsing of "c. 450 – 1150", "8th century BC", "2450 BP" …
    glotto.py         Glottolog tree + Wiktionary crossref (from core/data/langcodes/)
    bootstrap/        §B1: glottolog, wikidata, wikipedia, iecor, dplace, asjp, curated
    synthesize.py     §A3 rules: claims → adopted records (deterministic)
    checks.py         §B6 mechanical cross-checks (flag, never fix)
    derive.py         wety-facing view: Wiktionary code → dates
    tasks.py          §B2 ranked task list
    discover.py       §B3 OpenAlex + Semantic Scholar search, S2 enrichment / citation walks / recommendations, logged; screening
    s2.py             Semantic Scholar client: global 1 req/s limiter (file lock, shared by all processes), bulk search, batch, citations/references with contexts, snippets, recommendations
    retrieve.py       §B4 open-access full text (PMC XML, PDF, HTML), cached by sha256
    datasets.py       §B4b linked data: Zenodo / GitHub / OSF / Dryad discovery + fetch; deposited trees → node-age claims; tables/READMEs/SI PDFs → extraction input
    llm.py            §B3 screen, §B5 extract, §B6 synthesis review — Claude, structured outputs
    validate.py       everything on disk against the schemas
  data/               the artifact (committed)
    schema/           JSON Schemas
    sources/sources.json
    curated/periods.json     hand-curated periodizations with handbook citations
    <family glottocode>/     claims.jsonl · languoids.json · search_log.jsonl · candidates.json · *_log.jsonl
    isolates/ unclassified/  the two non-family buckets
    _derived/                wiktionary_dates.json/.csv · report.md · tasks.json
  cache/              fetched texts and pages (not committed)
```

## Run

```bash
cd dating && uv sync
uv run dating bootstrap            # all seven ingesters (network: Wikipedia, D-PLACE, ASJP PDF)
uv run dating synthesize           # rules → languoids.json, checks, _derived/, report
uv run dating validate
uv run dating tasks                # ranked task list (network: Wiktionary lemma counts)
uv run dating discover indo1319 --screen keyword   # or --screen llm (needs Claude credentials)
uv run dating retrieve indo1319
uv run dating extract indo1319     # model extraction; claims with program-verified quotes (--batch for the Batch API)
uv run dating synthesize           # re-run the rules over bootstrap + extracted claims
uv run dating llm-synthesize indo1319   # model review: rationale, open questions, justified overrides
uv run dating llm-check            # backend parity probe;  uv run dating costs  # spend from the logs
```

## Model backends and configuration

All model calls go through `harness/llm_backends.py`, configured by the
repo-level [`openrouter.config.json`](../openrouter.config.json). Three
backends implement one interface — `complete(role, system, user, schema) -> LLMResult`:

| backend | what | credentials |
|---|---|---|
| `openrouter` (default) | OpenAI-compatible chat completions on OpenRouter with ZDR routing, strict structured outputs, provider filters (no fp4/int4), caching, fallback models, batch API | `OPENROUTER_API_KEY` from the repo `.env` |
| `anthropic` | the Anthropic SDK directly | `ANTHROPIC_API_KEY` or an `ant auth login` profile |
| `claude-agent` | the Claude Agent SDK (Claude Code on the subscription), structured output through the CLI's JSON-schema mode, no tools | `CLAUDE_CODE_OAUTH_TOKEN`, or the keychain item `claude-code-oauth-token` created with `claude setup-token` + `security add-generic-password` |

Which model each role uses, and why, is in the config (`roles.screen / extract /
synthesize`, each with `model`, `fallbacks`, `batch_model`, `reasoning`, and a
`claude` entry for the two Claude backends). Current choices, from live
OpenRouter pricing/ZDR data plus the Artificial Analysis and Arena
leaderboards (all fetched 2026-09-17, recorded under `rationale.evidence`):

| role | openrouter | claude backends | reasoning |
|---|---|---|---|
| screen | `z-ai/glm-5.3-flash` → `openai/gpt-5.6-luna` | `claude-haiku-4-5` | thousands of tiny calls; cheapest model with a real reasoning score on fp8 ZDR endpoints |
| extract | `google/gemini-3.8-flash` → `openai/gpt-5.6-sol` | `claude-sonnet-5` | input-dominated; Arena 1493 at $0.75/M in, implicit caching, 1M context; quote verification catches errors |
| synthesize | backend `claude-agent` (Opus 5, effort high) — API fallback `openai/gpt-6-astra` | `claude-opus-5` | one heavy call per family; Opus 5 on the subscription costs nothing per token; ZDR Opus endpoints on OpenRouter lack strict structured outputs |

`DATING_LLM_BACKEND=openrouter|anthropic|claude-agent` overrides the per-role
`backend` for everything. `.env` is loaded by the harness (never printed).

**Keeping the backends in sync.** Prompts and output schemas exist once, in
`llm.py`; a backend only ever sees `(role, system, user, schema)` and must
return a schema-validated object. `dating llm-check` sends one identical
structured probe through every backend and reports whether the parsed outputs
agree — run it after touching a backend or the config. Every call is logged
with backend, model, message id, usage and cost; `dating costs` totals them.

**Linked datasets (plan §B4b).** `dating retrieve` also looks for each included
paper's deposited data — links in the paper's text/abstract first, then the
publisher landing page, then Crossref relations, and only then a Zenodo search
by DOI; GitHub repositories are fetched only when they look like language data —
fetches allowed files under size caps into `cache/datasets/<source>/`, records
everything on the candidate (`datasets`, with skipped files and errors), and
`dating extract` first parses any dated tree files mechanically into
`phylo_node_age` claims (unit inferred from root height, taxa mapped via a
Glottocode column or by name), then attaches the readable files (README,
tables, supplementary PDFs) to the extraction prompt as `DATASET FILE`
sections. Optional `GITHUB_TOKEN` in `.env` raises the GitHub API limit.

**Subscription usage limits (Claude Agent SDK).** The `claude_agent` block of
the config decides what happens when the subscription window is exhausted
(`RateLimitEvent` status `rejected` or a 429 result): `wait` (default) sleeps
until `resets_at` (capped at `max_wait_seconds`) and retries; `fallback`
continues on OpenRouter for that call; `fail` stops. Every stage writes its
progress after each item, so a killed run resumes where it stopped.

**Efficiency (from the OpenRouter docs).** Stable system prompt first, per-item
content last; explicit `cache_control` on Anthropic models, implicit caching on
Gemini/DeepSeek/GLM; `session_id = family` so sticky routing keeps a task on one
endpoint; `models` fallbacks avoid a client round trip on 429/5xx;
`provider.require_parameters` keeps strict JSON on endpoints that honour it
(and `temperature` is deliberately not sent — reasoning endpoints do not list it
and would be filtered out); `--batch` on `discover --screen llm` and `extract`
submits the same request bodies to the Batch API on the `:batch` variants at
50% price with a 24h window, `dating batch-poll FAMILY` ingests results through
the same post-processing.

**Semantic Scholar.** With `S2_API_KEY` in `.env`, discovery adds S2 bulk
search, enriches every candidate in one batch call (S2 ids, PMCIDs, OA links,
TL;DRs), walks citations/references of the top seeds keeping the citation
contexts (shown to the screener), and after screening runs a recommendation
round (includes as positives, excludes as negatives) whose hits are screened
too (`--no-walk`, `--no-recommend` to skip). Retrieval uses S2 full-text
snippets for papers with no open text. The key's limit — one request per
second across all endpoints — is enforced by `harness/s2.py` with a lock file
in `cache/`, so parallel runs on this machine share one clock; do not call the
API from anywhere else without going through it. Without the key, S2 is skipped
(the free pool returns 429s and each retry costs minutes).

Other keys: `OPENALEX_API_KEY` (in `.env`; the free pool allows a few searches a
day), `GITHUB_TOKEN` optional.

## Status (2026-09-17)

Run and verified in this environment:

| stage | result |
|---|---|
| bootstrap: Glottolog documentation years | 1,189 claims (attestation / conventional_period) |
| bootstrap: Wikidata temporal statements | 123 claims |
| bootstrap: Wikipedia infobox `era` / `extinct` | 611 claims from 7,781 articles; redirects to another language's article are skipped; 300 fields unparseable (prose) |
| bootstrap: IE-CoR tip-date priors | 52 claims |
| bootstrap: D-PLACE dated trees | 515 crown-age claims from 16 trees, nodes mapped to Glottolog by MRCA + monophyly check (e.g. Bouckaert 2012 IE root 6467 BCE, Chang 2015 4009 BCE) |
| bootstrap: Holman et al. 2011 ASJP | 289 rows parsed from the PDF, 128 placed on Glottolog family nodes (generic subgroup names left unplaced) |
| bootstrap: curated periodizations | 16 claims (English, French, German, Latin, Greek stages) from 5 handbooks |
| synthesize + checks | 9,059 languoid records, 575 crown ages, 37 records with flagged inconsistencies |
| derived view | 5,372 Wiktionary codes with a dated start (regular 4,959; etymology-only 298; reconstructed 115) |
| tasks | 467 tasks, Indo-European → Sinitic → Germanic → … |
| discover (Indo-European, keyword screen) | 129 candidates from 4 OpenAlex queries, 57 included |
| retrieve (Indo-European) | 30 full texts (Bouckaert 2012 and Chang 2015 via Europe PMC), 27 abstract-only |

Indo-European pilot (plan §C2), run 2026-09-17 through the configured backends:

| stage | backend / model | result | cost |
|---|---|---|---|
| model screen, 129 candidates | openrouter / glm-5.3-flash | 64 included, 65 excluded | $0.02 |
| retrieve | — | 30 full texts (PMC XML, PDF, HTML), 27 abstract-only | — |
| extract, 55 documents | openrouter / gemini-3.8-flash (3 fell back to gpt-5.6-sol) | 69 claims, 59 with program-verified quotes, 31 placed on Glottolog nodes; Bouckaert 2012 root extracted as 6764 BCE, HPD −8410…−5116 (paper: 7,116–10,410 BP; D-PLACE tree: 6467 BCE) | $1.03 |
| rules synthesis | — | Indo-European crown adopted from Heggarty 2023's own estimate (6121 BCE ≈ 8120 BP) with 35 alternatives | — |
| review, 80 languoids | claude-agent / claude-opus-5 | rationales referencing claim ids, 1 justified crown override, 3 spoken adjustments, open questions that name harness gaps (no override slot for `extinction`; HPDs missing from MCC trees; tree provenance mixed across parent/child nodes) | subscription (107k in / 41k out) |

`dating costs` totals spend from the logs; the whole pilot cost $3.43 in API
calls plus one subscription call.

Uralic pilot (2026-09-18), every role on the Claude Agent SDK (subscription):

| stage | result |
|---|---|
| discover | 1,212 candidates: OpenAlex + S2 bulk search, S2 enrichment of 224, citation walk from the 5 most-cited *included* papers (670 hits, 612 removed by the family-name gate before the model screen); 65 included by Haiku |
| retrieve | 29 full texts, 6 via S2 snippets (S2 has no body text for most Finno-Ugric venues), 1 language dataset (37 files) — GitHub tool repositories cited by aDNA papers were rejected by the relevance gate |
| extract (Sonnet 5) | 155 claims, 129 quote-verified, 73 placed; 48 second-hand (`reported_from`) |
| usage limit | the session limit hit mid-extraction ("You've hit your session limit · resets 2:30am"); the backend parsed the reset time, waited 184 min, and resumed — 9 papers had failed before the detector knew that message shape, all re-run |
| review (Opus 5) | 24 languoids; overrode the rules on Uralic, Finnic, Saami, Samoyedic with reasons (formation windows mistaken for crown splits; population dates mis-scoped to the family node) |

Sino-Tibetan pilot (2026-09-18), every role on the Claude Agent SDK:

| stage | result |
|---|---|
| discover | 286 candidates, 30 included; one usage-limit wait (246 min) during screening |
| retrieve | 8 full texts, 3 via S2 snippets, 19 abstract-only; 4 papers with deposited data — Sagart et al. 2019's Zenodo supplement (76 files, found by the DOI search, `isSupplementTo`), three Zenodo records and a lexibank repository from the landing page of the Tibetic paper |
| extract (Sonnet 5) | 101 model claims (92 verified) + 110 node ages read mechanically from the deposited tree files: summary trees as points, posterior `.trees` files as medians with 95 % intervals over a burn-in-trimmed subsample (root 7160 BCE [−8940, −5973] on the relaxed-clock FBD run; MCC 5186 BCE = the paper's 7184 BP); one usage-limit wait (237 min) |
| review (Opus 5) | 77 languoids; overrode the rules' Sino-Tibetan crown (it had adopted a Southwest-Tibetic-only root reported in a later paper) for the primary PNAS estimate, flagged calibration priors re-reported as evidence, and noticed the 1950-vs-2000 "present" offset between text and tree ingests |

What the reviews exposed, and what changed as a result: the first tree read
from a posterior sample file was a pre-burn-in state (root 38457 BCE) — tree
files with many trees are now treated as posteriors (10 % burn-in, ≤ 200
evenly-spaced trees, per-node median and 2.5–97.5 % interval, nodes kept only
when monophyletic in ≥ 50 % of trees); extraction now refuses
population / culture / migration dates unless the text equates them with a
language stage, treats regional language shift as *not* an extinction of the
family, requires numeric bounds, and gives node-level claims a role
(crown / stem / interval — an existence window is never midpointed into a
crown); the rules set aside period/extinction claims on family nodes unless
the source is curated; screening excludes genetics papers unless the abstract
itself dates a language; second-hand claims' `reported_from` strings become
S2 follow-up searches for the primary source; the S2 recommendation round is
opt-in (zero yield in two pilots); citation walks are seeded from included
papers and gated on the family name. LLM reviews are kept across `dating
synthesize` re-runs while the claim set they saw is unchanged, and marked stale
otherwise.

## How the numbers should be read

- `spoken.basis` says where each interval comes from. "conventional period" and
  "attestation window" are the strong ones. "start derived from the crown age of
  X" is plan rule 4: a living language's start is its nearest dated ancestor's
  first split — a lower bound of unknown looseness, always `confidence: low`.
- `confidence` on a family node: `high` = two independent phylogenetic studies,
  `medium` = one, `low` = only ASJP or an expert estimate.
- `checks` lists contradictions between sources or across the tree; they are
  reported, not resolved. The plan's human-review queue is `_derived/report.md`
  § Check violations.
- Everything unaccepted by Glottolog (Niger-Congo, Altaic, …) has no node and
  therefore no record; see the crossref README.

## Reproducibility record

- Inputs: `core/data/langcodes/` (hashes in its report), `data/sources/sources.json`
  (retrieval dates, SHA-256 of retrieved texts), `cache/` for the bytes.
- Every search: `<family>/search_log.jsonl` (provider, verbatim query, hit ids).
- Every screen decision: `<family>/screen_log.jsonl`; every model call:
  `extract_log.jsonl` / `synthesis_log.jsonl` with model id and message id.
- Every record carries `run` and `claims_considered`; every claim carries
  `extracted_by`. Re-running `synthesize` on the same claims is deterministic.
- `uv run dating validate` must pass before committing `data/`.
