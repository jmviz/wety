# Chronological evidence for languages

## Purpose

wety will show *when* the languages in its trees were spoken: optionally on etymology
and descendant trees (each node placed in time), and on a separate language-family
page where dating is part of the visualization. This package supplies the data for
that: for each language node, a time interval with its uncertainty, the **basis** of
the dating (conventional period, attestation, or model estimate), and citations; and
for family nodes, the split estimates that underlie them.

It does this by recording **what a source dates, how it dates it, and the conditions
under which that date is interpretable**, then deriving display periods from that
evidence. A language is a historically continuous, socially delimited object. A named
stage is often a scholarly convention; an attestation dates surviving documentation; a
phylogenetic node dates divergence among particular sampled lineages under a model.
These are different estimands. Neither the comparative method nor a lexical clock
makes a parent-family crown date the beginning of every descendant language.

Current state and next steps: [STATUS.md](../../STATUS.md). Per-family results:
[families/](families/).

## Scientific contract

* `subject` identifies a catalog entity, Wiktionary stage, documentary sample, reconstructed stage, population, or exact sampled clade. Catalog links are proposed correspondences. `exact`, `subset`, `superset`, `overlap`, and `unresolved` are distinct mapping relations. Source taxon lists survive ingestion.
* `event`, `role`, and `method` are separate. Observations, conventional periods, calibration priors, posterior estimates, secondary reports and archaeological/linguistic hypotheses cannot silently substitute for one another.
* `time` distinguishes points, uncertainty about an event, durations, bounds and distributions. Means, medians, HPDs and equal-tailed intervals retain their labels. Original quantities are retained. Calendar conversion is programmatic; BCE has no historical year zero. Source-stated BP epochs and a labeled conventional 1950 interpretation are separate. Calendar/model years and uncalibrated radiocarbon ages are different scales; unknown scale prevents conversion. See [time interpretation](docs/time-interpretation.md).
* `study_id`, `analysis_id`, and `result_id` prevent a paper, its supplement, D-PLACE copy and repository deposit from becoming independent votes. Alternative clock/root/calibration scenarios remain separate analyses. No averaging, newest-paper ranking or source-count confidence determines a default.
* `assessment` separates textual fidelity, semantic interpretation and entity mapping. “Supported” means the recorded assertion satisfies those checks, **not that the historical hypothesis is true**.
* New model extraction is a proposal. Verbatim spans, source-native numbers, immutable run histories and content digests make it auditable. A model review cannot overwrite dates. An explicit editorial decision names an eligible claim, reviewer, rationale and current evidence digest.
* Only supported, exact `wikt:<code>` evidence enters that code's `chronology`. Catalog and ancestor evidence are contextual references, never fallback lifespans. Unknown is a valid result. (`spoken` in the compatibility view is `null` until the display export described in STATUS exists.)
* Wiktionary varieties (etymology-only codes such as `ja-mid-ear` under `ojp`, `vsn` and `cls` under `sa`) are subjects of their own. A parent code is matched against the stage it names: Old Japanese 700–800 is exact for `ojp`, Early Middle Japanese for `ja-mid-ear`. The display export adds a derived `span` from a code's own period and its varieties' periods, listing each component; it is never evidence, is shown only beside the code's own period (a span cannot tell a living parent from a historical one), and is `partial` while some varieties are undated. Researchers still never join stage periods into one claim.
* Texts transmitted orally and known only from much later copies (the Vedas, the Avesta) are dated by event `composition`; an adopted composition window is shown, basis `composition`, only when the code has no adopted period or attestation.
* A proto-language (a code Wiktionary names "Proto-…", whether filed as reconstructed, attested like Proto-Norse, or as a variety like Proto-Romance) may end at a crown age: a decision on its code with event `crown_divergence` may select a supported crown claim of a sampled clade (two or more taxa) when its `proto_mapping` says why the sampled taxa cover the family. The clade claim stays about the clade; the export shows the crown interval as the proto stage's estimated end, never as its start or as a daughter's beginning. A source that dates the family's breakup itself (a family age, an article's node age) may also give the crown claim; an article's Bayesian node age names its sampled descendants. Each estimate carries its method. A posterior's label names the central value its source reports (the mean when it reports a mean, else the median), never a midpoint computed here, with the interval beside it; an estimate given only as an interval is labeled by the interval. A crown decision may list reviewed crown ages of trees (`proto_alternatives`, each with its own `proto_mapping`) shown beside the default, whatever kind of estimate the default is.

The [catalog cross-reference audit](../catalogs/critique.md) covers the identity layer. Catalog correspondence never licenses automatic chronological transfer.

## Review

Model reviewers are accepted; human review is optional additional evidence. A supported
assertion needs a named reviewer (model or human), exact evidence and the normal
semantic/mapping checks; an editorial decision can be model-authored if its scenario
and eligibility checks pass. Agreement between sessions of the same model is process
separation, not independent corroboration. Details: [model-review policy](docs/model-review.md),
[annotation and evaluation protocol](docs/review-protocol.md).

## Run

From the repository root (Python 3.12+, dependencies pinned in the root `uv.lock`):

```sh
uv sync --frozen
uv run --frozen wety-data run dating                       # publish + validate
uv run --frozen python -m unittest discover -s packages/dating/tests
uv run --frozen python -m harness.cli validate
uv run --frozen python -m harness.cli benchmark
```

Tests and the benchmark are offline and make no model calls. Several test modules read
local research data under `data/` that is backed up outside git; they fail in a fresh
clone without it. Without separately supplied predictions, extraction accuracy is
explicitly **not measured**.

Code changes anywhere in `harness/` change the generation fingerprint, so republish
(`python -m harness.cli synthesize`) after editing harness code, then validate.
Editorial decisions bind to what they reviewed: the subject's claims, their sources
and the code's catalog record (`python -m harness.cli decision-digest wikt:ang`), plus
`DECISION_RULES` in `registry.py`. Bump that constant when adoption rules or evidence
semantics change. Code edits elsewhere no longer make decisions stale; a change to
the subject's evidence or catalog mapping does.

## Research

`dating period-targets` lists the codes that name past stages (proto-languages,
varieties, ISO historical or extinct languages), ranked by how many items of the
selected graph use them, with their current period and evidence counts
(`data/products/dating/period_targets.json`); research batches are chosen from it.

New family research uses the [single frontier researcher](docs/single-frontier.md): one
continuing, tool-enabled model session that discovers, retrieves, interprets and writes
proposals, followed by a self-audit. Its output is a proposal dossier under
`data/workspaces/dating/_research/`. Proposals enter the registry only through import.
A reviewer writes `<run>/coordinator-review/review.json`: a status and any corrections
for every assertion, registry metadata for each source, and the editorial defaults to
adopt (format in `harness/research_packet.py`; example: the
[period pilot](../../data/workspaces/dating/_research/period-pilot-2026-10-03/README.md)).
Then:

```sh
uv run --frozen python -m harness.cli build-packet RUN RUN/coordinator-review/review.json
uv run --frozen python -m harness.cli import-evidence data/workspaces/dating/_research/packets/ID/claims.json
uv run --frozen python -m harness.cli synthesize
uv run --frozen python -m harness.cli adopt RUN/coordinator-review/review.json
uv run --frozen python -m harness.cli synthesize
uv run --frozen python -m harness.cli validate
uv run --frozen python -m harness.cli export-periods
```

`build-packet` registers the run's sources with their parsed-text hashes and copies
the texts to the retrieved-text store. `decide FILE` records a single decision by hand;
`withdraw SUBJECT EVENT --reviewer R --rationale TEXT` (or a review's `withdraw` list)
removes one, keeping it with the reason in `_decisions/withdrawals.json`, e.g. so an
attestation window can replace a database documentation window in the display.
`adopt` also records again, unchanged, any choice of a code the review touched but did
not replace (new evidence makes it stale); `readopt [SUBJECT...] --reviewer R --note
TEXT` does this after a `DECISION_RULES` bump. A revision review (`revises`: an earlier
packet id) covers only the assertions it changes, e.g. moving a stage period from a
parent code to its variety code with `correct: {"code": ...}`.

Sources a researcher cannot reach (paywalled handbooks and papers) are supplied by the
maintainer, usually through Zotero: `dating holdings zotero ITEMKEY...` retains the PDF
and its parsed text with the same provenance as a download, and runs read it with the
`library` and `open` tool actions. Researchers record what they still need with
`request`; `dating requests list|push` shows the queue and files it in Zotero. See
[supplied sources and requests](docs/single-frontier.md#supplied-sources-and-requests).

An import conforms to `schemas/claim.schema.json` and passes Pydantic and cross-record
checks. Changed evidence needs a new ID referencing the previous assertion. A decision
conforms to `schemas/decision.schema.json`; it selects evidence and cannot change its
numbers. Calibration priors and methodological demonstrations are contextual, never
historical defaults. Posterior-tree ingestion requires explicit units/epoch, exact taxa,
justified burn-in and trace-linked diagnostics; the importer does not compute
convergence. Backend configuration: [model-configuration.md](docs/model-configuration.md).

For etymology, inheritance and borrowing constrain source-specific events. Do not reject
an etymological edge because its endpoints lack adopted lifespans. Relative relations
need their own evidence and are never inferred from the catalog tree.

## Data layout

| Path | Meaning |
|---|---|
| `data/products/dating/CURRENT.json` | Pointer to the current generation; readers start here (tracked) |
| `data/artifacts/dating-generations/<digest>/bundle.json` | Complete, validated, content-addressed generation (local) |
| `data/products/dating/*.json` | Mirrors of the current generation: evidence, review queue, Wiktionary view, benchmark (local) |
| `data/workspaces/dating/<family>/claims.jsonl` | Legacy per-family claims, conservative ingress (tracked) |
| `data/workspaces/dating/_evidence/`, `_analyses/`, `_constraints/` | Typed source evidence, analysis manifests, relative relations (tracked) |
| `data/workspaces/dating/_research/packets/` | Editorial source packets: spans, provenance, importable claims (tracked) |
| `data/workspaces/dating/_research/<run>/` | Research runs; only `report.md`, `assertions.jsonl`, revisions and `README.md` are tracked |
| `data/workspaces/dating/retrieved/` | Retrieved source bytes and parsed text (local; never committed) |
| `data/artifacts/dating-local-sources/<sha256>/` | Bytes of maintainer-supplied sources (local; never committed) |
| `data/registry/holdings.json`, `source-requests.json` | Supplied sources and the acquisition queue: metadata only (tracked) |

Use `harness.registry.current_bundle()` for consistent reads.

## Current limits

Three deposited analyses are reproduced (IE, Sino-Tibetan, Uralic); their clades are not
adjudicated as equivalent to Wiktionary stages, so they produce no code-specific dates.
No all-family gold standard exists. The benchmark establishes mechanical fidelity and
safety properties, not extraction accuracy or the true age of any family.
