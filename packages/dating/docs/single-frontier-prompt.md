# Single frontier researcher brief

Shared by Codex subscription, Claude subscription and OpenRouter API. The executable
runner substitutes the family and freezes the selected model/effort and limits.
See [operation and effort choices](single-frontier.md).

```text
Research chronological evidence for {{FAMILY}}. Work autonomously as one frontier agent, without subagents
or additional model/API calls. Deliver the research packet in this run. Do not
redesign the harness, run model comparisons, or start a new evaluation loop.

The runner supplies the scientific contract, frozen brief, time limits and retained
acquisition tools. Use research_action for search/acquire/read/page/write/check/status.
The tool description lists exact arguments. All output stays in this new proposal
workspace; do not publish, modify the harness or start another model run.

Scope and stop conditions:
- Aim for 3–6 substantive primary sources, not dates for every language. At most
  8 searches, 12 total acquisitions and two extra routes per blocked central source.
  Crossref, OpenAlex and Europe PMC searches are available; follow original sources
  and correction/supplement links with acquire. Failed requests consume the limits.
- Check status periodically. The default is 20 minutes research plus 5 for delivery;
  actual frozen limits prevail. Save partial deliverables early, before limits expire.
- Prefer 10–25 useful assertions, never a quota. One self-check and one correction
  pass, then finish with explicit gaps. Missing evidence does not justify a rerun.
- Focus reasoning on event/role, native dates, sampling and source dependence.
  Routine searches/file operations need short decisions, not extensive planning.

Research and source semantics:
Search across appropriate scholarly discovery sources, not just Europe PMC.
Retain exact acquired bytes, hashes, source URLs and acquisition receipts. Identify
abstract/catalog pages, article bodies, tables, figures and supplements separately;
never claim to have read unavailable material. Inspect relevant tables/figures
directly where necessary. Record included, excluded, blocked and unread sources.
Use a source inventory rather than assuming a search was comprehensive.
Scope conclusions to sources actually read: “one study found in this run” does not
mean “the only study.” Reserve one search for recent work and one for substantive
critiques or corrections within the existing search allowance. Before declaring a
central source absent, check the bibliography of the sources already read.

For each assertion identify the literal dated entity and sampled descendants,
event, method, epistemic role, original date expression, units/statistic/epoch,
source location and supporting source span. Keep table headers, prior definitions
and source-defined epochs with the assertions that need them. Copy evidence from
retained source text or its exact offsets; do not regenerate quotes from memory.
If visual/OCR evidence is required, retain page/image coordinates and label the
transcription and uncertainty; do not invent a text-span match.

Distinguish fitted crowns/stems from named reconstructed stages, first surviving
texts from language origins, documentary sampling from earliest attestation, and
population history from language divergence. Catalog correspondence does not
transfer dates. Do not export spoken intervals; spoken remains null.
Preserve analysis alternatives, calibration priors, posterior results, secondary
quotations, critiques/replies and source dependencies separately. References that
repeat one analysis are not independent confirmations. Do not splice different
scenarios' endpoints or interpret a marginal interval as a joint posterior.
A calibrated node can have both a prior and a fitted posterior: identify which
quantity is quoted; do not infer that the posterior does not exist. Distinguish a
family split from subsequent diversification inside one daughter branch. A paragraph
aggregating several families does not assign every listed item to every family.
Preserve source-native numbers even when calendar conversion is unresolved. BP
convention is 1950 unless a source states otherwise; label convention separately
from a source-stated epoch and distinguish calendar from radiocarbon scales. Bare
years ago does not itself authorize BP1950. Never transfer a paper's model epoch
to a secondary quotation. Priors need family/location/SD/support; elapsed lengths
and relative offsets are not absolute dates. Preserve unsupported shapes as text
with an explicit gap. Do not change the scientific contract to force validation.
Historical arithmetic may suggest an epoch but does not make it source-stated.
If reported prior parameters and intervals conflict, preserve both and flag the
conflict. Do not force ambiguous slash dates into discrete alternatives. “Not older
than century X” permits that century; it does not mean strictly after century X.
For etymological utility, preserve source-argued contact or before/after constraints;
language-level dates alone do not date borrowing/inheritance edges or lexemes.
Contact continuing until a date does not put every borrowing after that date or
require migration to follow it. Word-specific bounds need the actual donor/recipient
innovation and reflex argument. Keep those premises explicit.

Deliverables, written using write(name, content):
1. report.md: source-specific synthesis, competing models, utility for wety, gaps
   and actual coverage. No universal adopted age. Distinguish useful partial work.
2. sources.json: source identity, dependencies, coverage (body/table/figure/supplement),
   exclusions and access gaps. Reuse source_id from acquire; raw/text hashes and
   HTTP provenance are retained by the tools. Retention alone does not mean read.
3. assertions.jsonl: one JSON object per line, with unique id, source_id, exact quote,
   locator (prefer read offsets), subject, event, role, method, native_time (source-native
   quantities/statistic/units/epoch; unresolved values explicit), qualifications,
   scenario, sampled_taxa, reported_from, relative_constraints. Do not force a shape
   that loses meaning. Optionally include compact when an existing compact Assertion
   is already available; do not spend the run learning a large schema. These are
   unimported proposals. Visual-only readings without exact text belong in explicit
   report gaps with page/coordinates, not fabricated quote matches.
4. self-check.json: each assertion's ID, checks of subject/event, role/dependency,
   native time, sample/scenario and evidence, revisions and unresolved issues.
   This is correlated self-audit, not independent review. Call check after saving,
   fix affected records once, then finish. Passing checks is not semantic validation.
   Re-read supporting spans for consequential constraints and corrections; do not
   invent line breaks or accept a matching fragment that omits the relevant date.
   Check that report conclusions retain the qualifications of the assertion records.

The runner records task usage when supplied by the backend; never invent usage or
convert account-wide counters to a per-family weekly percentage. Include tool and
source limitations in the report. Finish with the dossier location and useful
findings; deliver partial results instead of redesigning tools or seeking a reviewer.

```
