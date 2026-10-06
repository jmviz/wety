# Batch brief: Indo-European subgroup proto-languages

Generated from `packages/dating/prompts/period-verification.md` for run `period-ie-protos-2026-10-06`;
codes ranked by `dating period-targets`.

```text
Verify when each of the Wiktionary language codes below was spoken, for display on wety's
etymology trees ({{FAMILY}}). Work autonomously as one researcher, without subagents or
other model/API calls. Do not redesign tools, run comparisons or start other runs.

What wety will show. For each code, one interval: start and end, each as a range of
years; a basis (conventional period, attestation window, or estimate); and citations.
You are checking the evidence for that interval, not writing it. spoken stays null in
your records; the integrator derives display intervals only from reviewed claims.

The codes are listed at the end, most-used in wety's trees first. Work in that order;
if time runs short, deliver what is verified rather than covering every code thinly.
Unknown is a valid result for a code.

For each code:
1. Look for a scholarly statement of the period or attestation window of the entity the
   code names: a conventional period from a handbook or reference grammar, or the
   first/last surviving texts. Quote it exactly from retained text with a locator
   (printed page, and PDF page). Never reproduce a quotation from memory.
2. Mapping. Map each dated entity to the most specific Wiktionary code. Varieties
   (etymology-only codes) are codes of their own: Classical Latin is exact for "la-cla",
   Early Middle Japanese for "ja-mid-ear". A parent code is matched against the stage
   it names, not the union of its varieties: Old Japanese 700–800 is exact for "ojp"
   even though Middle Japanese is filed under it. A dated entity not listed below but
   with its own Wiktionary code (a variety named in the notes, for example) may be
   recorded under that code. Say subset, superset or overlap otherwise, and why. Never
   join several stage periods into one interval yourself; the integrator derives spans
   over varieties.
3. Record conflicts between sources as separate claims. Do not average or choose by
   recency.
4. Propose at most one default per code and event, with a rationale, and keep the
   others as alternatives. Prefer a scholarly handbook statement over ISO 639-3 names
   and Glottolog documentation windows when both have exact scope.

Proto-languages and reconstructed codes: record every published estimate of when the
proto stage was spoken or broke up, each as its own claim with its method; never merge
them. A date for the stage itself ("Proto-Slavic was spoken c. ...") is a
conventional_period or bound with role hypothesis. A family's age or breakup date (a
family age from ASJP or glottochronology, a node age an article reports for a named
family or subgroup) is event crown_divergence about the proto code, role posterior for
a Bayesian analysis and hypothesis otherwise; give the source's name for the family and
its sample (which subgroups) in source_label and mapping_basis, and map it exact only if
the sample spans the family's first split. For a Bayesian posterior, list the node's
sampled descendants in subject.taxa (the study's tip labels, or the subgroups it names
as the node's members) and acquire a deposited tree if there is one. Set method to one of bayesian_phylogenetic,
lexicostatistic (incl. ASJP and glottochronology), comparative_reconstruction (dated
from reconstructed vocabulary, loans or sound changes), archaeological_correlation,
expert_estimate. Keep a calibration prior apart from a posterior. Never use a
daughter's first attestation or an archaeological culture's dates as the proto
stage's date unless the source itself dates the stage with them.

Source access, in this order.
1. Supplied sources first. Call library (no query lists everything). For this batch:
Supplied handbooks first (library searches tables of contents): the Handbook of Comparative and Historical Indo-European Linguistics (3 vols.; Indic, Iranian, Greek, Armenian chapters), Fortson, Kapović's The Indo-European Languages, The Iranian Languages (Windfuhr), Woodard's volumes, Horrocks's Greek. Otherwise open scholarly sources (Encyclopaedia Iranica, open-access papers, dated phylogenies with Indo-Iranian or Greek nodes). The maintainer supplies full texts on request; request a work only when nothing readable settles a code, and keep going.
   Open a holding with open, then use find (language names with "century", "BC",
   "period", "attested", "texts", "inscriptions", "dated") rather than paging through
   it with read. find gives the PDF page and the printed page label; cite the printed
   page. Check the title page of each holding you cite (identity and edition). Where
   find or page flags OCR or a scanned text layer, view the page image with page before
   relying on the quote, and say so in self-check.json.
2. Openly reachable sources: the ISO 639-3 code tables and Glottolog CLDF (already in
   the registry: use them only for a new question), open-access chapters and articles.
   search indexes and web search (if enabled) give leads, never evidence: acquire a
   source before quoting it.
3. A work you need but cannot read: try distinct routes once each (publisher page,
   open-access locations from openalex or semanticscholar, institutional repositories,
   the author's homepage via s2author or web search, Open Library public scans). Do not
   retry a blocked route. Then call request: say exactly what the work would settle, for
   which codes, which part is needed, and the routes tried. Request only works that
   would change a code's result, not every search hit.
Check status early and periodically; save partial files early; deliver partial work
rather than exceeding limits.

Time semantics. Record native_time exactly as the source states it:
{"form": "duration" | "event_range" | "point" | "bound",
 "unit": "CE" | "BCE" | "century_CE" | "century_BCE" | "millennium_BCE",
 "lower": n, "upper": n, "value": n, "relation": "before|after|by|since",
 "precision": "year|decade|century|millennium", "approximate": true|false,
 "expression": "the source's own words for the dates"}
A period "c. 450–1150" is form duration, unit CE, lower 450, upper 1150, approximate
true. "5th–12th century" is unit century_CE, lower 5, upper 12. A single century or
millennium is never a point: "the seventh century BC" is form event_range (or duration
for a period), unit century_BCE, lower 7, upper 7; keep "middle of"/"end of" in the
expression, not in the numbers. BCE years are positive numbers with unit BCE (lower =
the earlier endpoint as written, e.g. 2600 for "2600–500 BC"). If the endpoints are in
different eras or units (e.g. "1250 BC – AD 220"), give {"form": "duration", "start":
{"value": 1250, "unit": "BCE"}, "end": {"value": 220, "unit": "CE"}, ...}; an endpoint
has only value and unit. Never convert to other years yourself; conversion is
programmatic. "To 900" is an end with no stated start: say so; do not invent a start.
check normalizes every native_time as the registry will and reports what it rejects;
a supported assertion needs a stated time.

Events and roles: conventional_period (role convention) for a named stage's
periodization; first_attestation / last_attestation (role observation) for dated
surviving texts; documentary_sampling (role observation) for an attestation window
stated as one span ("attested from the 3rd c. BCE to the 5th c. CE") or a database
documentation window; conventional_period with role hypothesis and method expert_estimate for a
proto-stage dating; composition (role hypothesis) for when texts transmitted orally or
known only from much later copies were composed (the Vedas, the Avesta), and
manuscript_production for the date of a surviving copy; native_transmission_end
(role hypothesis) for when a language stopped being spoken natively; literary_use for
a written or learned register. Attestation is not the spoken period: say so in
qualifications. Use only these event names; check rejects others.

Deliverables (write via the write action; these four names only):
1. assertions.jsonl, one JSON object per line, with: id; code (the Wiktionary code);
   source_id (from acquire or open); quote (exact substring of the retained text, long
   enough to include the dates and the entity name, no invented line breaks); locator
   ({"page": printed page label, "pdf_page": n} for books; read offsets or table row
   otherwise); subject {"code", "source_label", "mapping":
   "exact|subset|superset|overlap|unresolved", "mapping_basis"}; event; role; method;
   native_time; qualifications (list); verifies (list of {"claim": existing claim id
   prefix, "verdict": "confirmed|corrected|contradicted", "note"}) or [] for new
   evidence; proposed_status ("supported" only if quote, numbers, event and exact
   mapping are all checked, else "candidate"); proposed_decision ({"default": true,
   "rationale"} on at most one assertion per code and event, else null); citation (full
   bibliographic citation of the source as published, with edition; for a chapter,
   its author and title within the volume).
   Scope assertions (what a source says an entity includes) use the same fields with
   event "unspecified", role "observation", method "scholarly_description", native_time
   {"form": "unknown"}, plus target (e.g. "iso639-3:san", "glottolog:sans1269") and
   what_it_shows (one sentence).
2. report.md: a table with one row per code (best evidence, its mapping, proposed
   default, alternatives, conflicts, remaining gap), then scope findings, then sources
   requested from the maintainer and why.
3. sources.json: source_id, citation, URL or holding, what was read (pages), access
   problems, and sources attempted but not read.
4. self-check.json: per assertion, checks of quote, numbers and era, event/role,
   mapping, title-page identity of the source, and page-image checks for OCR text.
Call check after saving, fix problems once, and finish. Passing check does not
establish correctness. Finish with a short summary of what is verified per code.

Codes (most-used in wety's trees first). Each note gives the Wiktionary name and kind,
the parent a variety is filed under or the varieties filed under a code, what wety
shows now, and where to look. Dates in a note's pointer are unverified leads, not
evidence: verify them in a source or leave them out.

inc-pro — Proto-Indo-Aryan (reconstructed). Varieties with their own codes: inc-mit
  Mitanni. Tree items: 3005. Now: none. Proto-Indo-Aryan: every published estimate of
  when the stage was spoken or broke up (handbook statements, family ages, article
  node ages), each with its method. Heggarty et al. 2023's M3 nests Nuristani inside
  Indo-Aryan, so its Indo-Aryan clade is not this code without a mapping argument;
  record Indo-Aryan node ages from other analyses and handbook dates for
  Proto-Indo-Aryan or Old Indo-Aryan's emergence.
grk-pro — Proto-Hellenic (reconstructed). Tree items: 1725. Now: none. Proto-Hellenic:
  every published estimate of when the stage was spoken or broke up (handbook
  statements, family ages, article node ages), each with its method. A crown of
  attested Greek dialects is bounded by Mycenaean (a sampled ancient tip); prefer
  handbook dates for Proto-Greek before Mycenaean (Horrocks, the IE Handbook's Greek
  chapters).
iir-nur-pro — Proto-Nuristani (reconstructed). Tree items: 315. Now: none.
  Proto-Nuristani: every published estimate of when the stage was spoken or broke up
  (handbook statements, family ages, article node ages), each with its method.
xsc-pro — Proto-Scythian (reconstructed). Tree items: 188. Now: none. Proto-Scythian:
  every published estimate of when the stage was spoken or broke up (handbook
  statements, family ages, article node ages), each with its method.
xsc-sak-pro — Proto-Saka (reconstructed). Tree items: 155. Now: none. Proto-Saka:
  every published estimate of when the stage was spoken or broke up (handbook
  statements, family ages, article node ages), each with its method.
xsc-sar-pro — Proto-Sarmatian (reconstructed). Tree items: 150. Now: none.
  Proto-Sarmatian: every published estimate of when the stage was spoken or broke up
  (handbook statements, family ages, article node ages), each with its method.
xsc-skw-pro — Proto-Saka-Wakhi (reconstructed). Tree items: 137. Now: none.
  Proto-Saka-Wakhi: every published estimate of when the stage was spoken or broke up
  (handbook statements, family ages, article node ages), each with its method.
pra-pro — Proto-New Indo-Aryan (etymology-only, a variety filed under pra Prakrit).
  Tree items: 107. Now: none. Proto-New Indo-Aryan: every published estimate of when
  the stage was spoken or broke up (handbook statements, family ages, article node
  ages), each with its method.
hyx-pro — Proto-Armenian (reconstructed). Tree items: 55. Now: none. Proto-Armenian:
  every published estimate of when the stage was spoken or broke up (handbook
  statements, family ages, article node ages), each with its method. Proto-Armenian:
  dates the Armenian handbook chapters give, not the modern dialect split.
ira-shy-pro — Proto-Shughni-Yazghulami (reconstructed). Tree items: 45. Now: none.
  Proto-Shughni-Yazghulami: every published estimate of when the stage was spoken or
  broke up (handbook statements, family ages, article node ages), each with its
  method.
ira-shr-pro — Proto-Shughni-Roshani (reconstructed). Tree items: 42. Now: none.
  Proto-Shughni-Roshani: every published estimate of when the stage was spoken or
  broke up (handbook statements, family ages, article node ages), each with its
  method.
ira-sym-pro — Proto-Shughni-Yazghulami-Munji (reconstructed). Tree items: 38. Now:
  none. Proto-Shughni-Yazghulami-Munji: every published estimate of when the stage was
  spoken or broke up (handbook statements, family ages, article node ages), each with
  its method.
inc-ash-pro — Proto-Middle Indo-Aryan (etymology-only, a variety filed under inc-ash
  Ashokan Prakrit). Tree items: 33. Now: none. Proto-Middle Indo-Aryan: every
  published estimate of when the stage was spoken or broke up (handbook statements,
  family ages, article node ages), each with its method.
ira-sgc-pro — Proto-Sogdic (reconstructed). Tree items: 31. Now: none. Proto-Sogdic:
  every published estimate of when the stage was spoken or broke up (handbook
  statements, family ages, article node ages), each with its method.
ira-pat-pro — Proto-Pathan (reconstructed). Tree items: 27. Now: none. Proto-Pathan:
  every published estimate of when the stage was spoken or broke up (handbook
  statements, family ages, article node ages), each with its method.
ira-mpr-pro — Proto-Medo-Parthian (reconstructed). Tree items: 22. Now: none.
  Proto-Medo-Parthian: every published estimate of when the stage was spoken or broke
  up (handbook statements, family ages, article node ages), each with its method.
ira-mny-pro — Proto-Munji-Yidgha (reconstructed). Tree items: 15. Now: none.
  Proto-Munji-Yidgha: every published estimate of when the stage was spoken or broke
  up (handbook statements, family ages, article node ages), each with its method.
```
