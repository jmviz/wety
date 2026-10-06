# Batch brief: Historical and extinct languages, second batch

Generated from `packages/dating/prompts/period-verification.md` for run `period-historical-2-2026-10-06`;
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
Supplied handbooks where relevant (library searches tables of contents: The Iranian Languages and Woodard's Asia and the Americas volume for Parthian, Khotanese and Bactrian; The Turkic Languages for Old Turkic; Woodard's volumes for Minoan/Linear A; Loprieno for Coptic dialects; the IE Handbook volumes for Prakrit and Iranian; Kapović and Fortson for Frankish context). Otherwise open scholarly sources (Encyclopaedia Iranica, encyclopedia articles by specialists, open-access papers and grammars). For extinct languages record the end of native transmission (the last native speakers, with the source's date) as event native_transmission_end: wety shows it as the language's end. The maintainer cannot supply new full texts until tomorrow: request a work only when nothing readable settles a code.
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

cmg — Classical Mongolian (regular). Tree items: 730. Now: none. Classical Mongolian:
  the literary language's conventional period (from the 17th century to the 20th).
otk — Old Turkic (regular). Varieties with their own codes: otk-kir Old Kirghiz,
  otk-ork Orkhon Turkic. Tree items: 706. Now: none. Old Turkic: the attestation
  window of the runic and Old Uyghur-script texts (the Orkhon inscriptions are 8th
  century).
xpr — Parthian (regular). Tree items: 659. Now: none. Parthian: attestation window
  (Arsacid documents, Manichaean texts).
yur — Yurok (regular). Tree items: 645. Now: none. Yurok: earliest records; the last
  first-language speakers if a source dates them (the language is being revived, so
  not an end of the language).
txg — Tangut (regular). Tree items: 579. Now: none. Tangut: attestation window of
  Tangut texts (Western Xia, 11th–16th c.).
nrn — Norn (regular). Tree items: 577. Now: none. Norn: earliest records and the end
  of native transmission (last speakers, 18th–19th c.).
xas — Kamassian (regular). Tree items: 527. Now: none. Kamassian: the last native
  speaker's death (Klavdiya Plotnikova).
zko — Kott (regular). Tree items: 489. Now: none. Kott: earliest records (18th-c.
  wordlists) and the end of native transmission (19th c.).
awg — Anguthimri (regular). Tree items: 483. Now: none. Anguthimri: end of native
  transmission if a source dates it.
zls-chs — Church Slavonic (regular). Varieties with their own codes: zls-chs-ru
  Russian Church Slavonic, zls-chs-uk Ukrainian Church Slavonic. Tree items: 476. Now:
  none. Church Slavonic (the later recensions after Old Church Slavonic): conventional
  start; still in liturgical use.
xaa — Andalusian Arabic (regular). Tree items: 464. Now: none. Andalusian Arabic:
  conventional period or attestation window (zajal, glossaries) and its end.
pra-mah — Maharastri Prakrit (etymology-only, a variety filed under pra Prakrit). Tree
  items: 433. Now: none. Maharastri Prakrit (variety): dated attestation
  (Gāhāsattasaī, Setubandha).
aho — Ahom (regular). Tree items: 426. Now: none. Ahom: attestation window of Ahom
  manuscripts; end of native transmission.
cop-sah — Sahidic Coptic (etymology-only, a variety filed under cop Coptic). Tree
  items: 404. Now: none. Sahidic Coptic (variety of cop): period of its use as the
  main literary dialect.
kho — Khotanese (regular). Varieties with their own codes: kho-lat Late Khotanese,
  kho-old Old Khotanese. Tree items: 400. Now: none. Khotanese: attestation window
  (Old and Late Khotanese documents).
omn — Minoan (regular). Tree items: 379. Now: none. Minoan (Linear A): the attestation
  window of Linear A documents.
frk — Frankish (etymology-only, a variety filed under gmw-pro Proto-West Germanic).
  Tree items: 374. Now: none. Frankish (variety): only dated attestations (Malberg
  glosses, names) a source gives.
xbc — Bactrian (regular). Tree items: 348. Now: none. Bactrian: attestation window
  (Kushan inscriptions to the latest documents, 8th–9th c.).
sia — Akkala Sami (regular). Tree items: 341. Now: none. Akkala Sami: the last native
  speaker's death.
```
