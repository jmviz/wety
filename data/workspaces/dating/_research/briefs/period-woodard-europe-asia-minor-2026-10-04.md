# Batch brief: Woodard volumes: Europe and Asia Minor

Generated from `packages/dating/prompts/period-verification.md` for run `period-woodard-europe-asia-minor-2026-10-04`;
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

Proto-languages and reconstructed codes: use only dates a source gives for that proto
stage itself (e.g. "Proto-Slavic was spoken c. ..."). Never use a family's crown or
stem divergence date, a phylogenetic split, a daughter's first attestation, or
archaeological culture dates unless the source itself explicitly dates the proto stage
with them. Such periods are always estimates (role hypothesis), even when stated
confidently. Record competing proposals separately. (Phylogenetic crown ages are
handled by separate family research, not this brief.)

Source access, in this order.
1. Supplied sources first. Call library (no query lists everything). For this batch:
   Woodard (ed.) 2008, The Ancient Languages of Europe and Woodard (ed.) 2008, The Ancient Languages of Asia Minor (both Cambridge 2008, reprinting the 2004 Cambridge
   Encyclopedia of the World's Ancient Languages; chapters by the authors named in the
   notes). Also supplied: Clackson & Horrocks 2007, The Blackwell History of the Latin
   Language; Miller 2019, The Oxford Gothic Grammar; Fortson 2010; Kapović (ed.) 2017.
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
surviving texts; documentary_sampling (role observation) for a database documentation
window; conventional_period with role hypothesis and method expert_estimate for a
proto-stage dating. Attestation is not the spoken period: say so in qualifications.

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
shows now, and where to look.

got — Gothic (regular). Tree items: 23601. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Europe, ch. 9 (Jasanoff, Gothic); Miller 2019, The Oxford Gothic
  Grammar (dates witnesses only, per the retry). Wanted: a stated attestation window
  or period; Crimean Gothic is a later relative.
xcl — Old Armenian (regular). Tree items: 11356. Now: none. Woodard (ed.) 2008, The
  Ancient Languages of Asia Minor, ch. 11 (Clackson, Classical Armenian): earliest
  texts, the classical period, and its end.
la-vul — Vulgar Latin (etymology-only, a variety filed under la Latin). Tree items:
  3756. Now: none. Clackson & Horrocks 2007 and Clackson in Woodard (ed.) 2008, The
  Ancient Languages of Europe, ch. 4: whether a period is stated for Vulgar Latin.
oge — Old Georgian (regular). Tree items: 1753. Now: none. Woodard (ed.) 2008, The
  Ancient Languages of Asia Minor, ch. 12 (Tuite, Early Georgian): earliest
  inscriptions and the Old Georgian period.
xum — Umbrian (regular). Tree items: 881. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Europe, ch. 5 (Wallace, Sabellian languages): the Iguvine Tables and
  other Umbrian inscriptions.
hit — Hittite (regular). Tree items: 778. Now: c. 19th century – 1200 BCE (attested,
  Kapović 2017). Shown from Kapović and Fortson, checked against Watkins (Woodard
  (ed.) 2008, The Ancient Languages of Asia Minor, ch. 2) in the last round; nothing
  more needed unless a conflict appears.
ett — Etruscan (regular). Tree items: 601. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Europe, ch. 7 (Rix, Etruscan): attestation window of the inscriptions.
gmy — Mycenaean Greek (regular). Tree items: 572. Now: none. Woodard (ed.) 2008, The
  Ancient Languages of Europe, ch. 3 (Woodard, Greek dialects) and ch. 1: Linear B
  attestation (Mycenaean has its own code, not grc).
osc — Oscan (regular). Varieties with their own codes: osc-luc Lucanian, osc-sam
  Samnite. Tree items: 494. Now: none. Woodard (ed.) 2008, The Ancient Languages of
  Europe, ch. 5 (Wallace, Sabellian languages): Oscan inscriptions.
xlu — Luwian (regular). Tree items: 190. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Asia Minor, ch. 3 (Melchert, Luvian): cuneiform and hieroglyphic Luvian
  attestation.
xpg — Phrygian (regular). Tree items: 161. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Asia Minor, ch. 8 (Brixhe, Phrygian): Old and New Phrygian
  inscriptions.
xfa — Faliscan (regular). Varieties with their own codes: xfa-cap Capenate. Tree
  items: 159. Now: none. Woodard (ed.) 2008, The Ancient Languages of Europe, ch. 4
  (Clackson, Latin) or ch. 5: Faliscan inscriptions, if dated.
xce — Celtiberian (regular). Tree items: 109. Now: none. Woodard (ed.) 2008, The
  Ancient Languages of Europe, ch. 8 (Eska, Continental Celtic): Celtiberian
  inscriptions.
xlc — Lycian (regular). Tree items: 107. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Asia Minor, ch. 5 (Melchert, Lycian).
spx — South Picene (regular). Tree items: 96. Now: none. Woodard (ed.) 2008, The
  Ancient Languages of Europe, ch. 5 (Wallace): South Picene inscriptions.
xld — Lydian (regular). Tree items: 75. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Asia Minor, ch. 6 (Melchert, Lydian).
xcr — Carian (regular). Tree items: 63. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Asia Minor, ch. 7 (Melchert, Carian).
xve — Venetic (regular). Tree items: 45. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Europe, ch. 6 (Wallace, Venetic).
xhu — Hurrian (regular). Tree items: 39. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Asia Minor, ch. 9 (Wilhelm, Hurrian).
xlp — Lepontic (regular). Tree items: 36. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Europe, ch. 8 (Eska): Lepontic inscriptions.
grc-dor — Doric Greek (etymology-only, a variety filed under grc Ancient Greek). Tree
  items: 27. Now: none. Woodard (ed.) 2008, The Ancient Languages of Europe, ch. 3
  (Woodard, Greek dialects): whether dialect periods are stated.
plq — Palaic (regular). Tree items: 24. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Asia Minor, ch. 4 (Melchert, Palaic).
xur — Urartian (regular). Tree items: 23. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Asia Minor, ch. 10 (Wilhelm, Urartian).
xga — Galatian (regular). Tree items: 16. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Europe, ch. 8 (Eska): Galatian.
grc-aeo — Aeolic Greek (etymology-only, a variety filed under grc Ancient Greek). Tree
  items: 12. Now: none. Woodard (ed.) 2008, The Ancient Languages of Europe, ch. 3.
imy — Milyan (regular). Tree items: 5. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Asia Minor, ch. 5 (Melchert): Lycian B / Milyan.
grc-ion — Ionic Greek (etymology-only, a variety filed under grc Ancient Greek). Tree
  items: 4. Now: none. Woodard (ed.) 2008, The Ancient Languages of Europe, ch. 3.
grc-att — Attic Greek (etymology-only, a variety filed under grc Ancient Greek). Tree
  items: 3. Now: none. Woodard (ed.) 2008, The Ancient Languages of Europe, ch. 2
  (Woodard, Attic Greek): the period of Attic as a dialect (Koine, grc-koi, is already
  dated).
cel-gau — Gaulish (regular). Varieties with their own codes: xcg Cisalpine Gaulish,
  xtg Transalpine Gaulish. Now: none. Woodard (ed.) 2008, The Ancient Languages of
  Europe, ch. 8 (Eska, Continental Celtic): Gaulish inscriptions (Transalpine xtg and
  Cisalpine xcg are varieties).
gmq-pro — Proto-Norse (regular). Now: none. Woodard (ed.) 2008, The Ancient Languages
  of Europe, ch. 10 (Faarlund, Ancient Nordic): the runic language of the older
  futhark; the last round found its earliest inscriptions at the end of the 2nd
  century AD. Say whether Ancient Nordic is the entity Wiktionary calls Proto-Norse.
```
