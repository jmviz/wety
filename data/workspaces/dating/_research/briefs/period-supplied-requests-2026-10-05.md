# Batch brief: Codes whose requested sources were supplied

Generated from `packages/dating/prompts/period-verification.md` for run `period-supplied-requests-2026-10-05`;
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
   The works requested by the last batches, now supplied (use library with the
   language name: it searches each holding's table of contents): Rickard, A History of
   the French Language (2nd ed.); Penny, A History of the Spanish Language (2nd ed.);
   Johanson & Csató (eds.), The Turkic Languages (2nd ed., chapters on Ottoman,
   Proto-Turkic and old Turkic languages); Windfuhr (ed.), The Iranian Languages (Middle
   West Iranian; Persian and Tajik); Oberlies, Pāli; Houston, Robertson & Stuart, The
   Language of Classic Maya Inscriptions; the Wacana article on the earliest Old Javanese
   documents; Pulleyblank, Outline of Classical Chinese Grammar; Loprieno, Ancient
   Egyptian; Bakkum, The Latin Dialect of the Ager Faliscus; Grumet, Neighbors and
   Intruders (Delaware); and the abstract of the Oxford Bibliographies article Medieval
   Latin (an abstract-only holding: cite it as an abstract).
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

ota — Ottoman Turkish (regular). Tree items: 21300. Now: none. The Turkic Languages,
  ch. on Ottoman Turkish: its period (the last run found Özkan's period includes Old
  Anatolian Turkish).
fro — Old French (regular). Varieties with their own codes: fro-nor Old Northern
  French, fro-pic Picard Old French, xno Anglo-Norman, xno-law Law French, zrp
  Zarphatic. Tree items: 20470. Now: 9th–13th century CE (conventional, Regnault et
  al. 2019). Rickard: Old French period (replaces a treebank paper's passing 9th–13th
  c., adopted provisionally).
frm — Middle French (regular). Tree items: 12185. Now: 14th–15th century CE
  (conventional, Regnault et al. 2019). Rickard: Middle French period (replaces a
  passing 14th–15th c.).
pi — Pali (regular). Tree items: 11862. Now: none. Oberlies: the dating of Pali (canon
  composition vs. redaction vs. manuscripts).
trk-pro — Proto-Turkic (reconstructed). Varieties with their own codes: trk-bul-pro
  Proto-Bulgar, trk-cmn-pro Proto-Common Turkic, trk-ogr-pro Proto-Oghur, trk-ogz-pro
  Proto-Oghuz. Tree items: 6336. Now: none. The Turkic Languages: any date given for
  Proto-Turkic itself (an estimate; never a split date unless the source dates the
  proto stage with it).
osp — Old Spanish (regular). Tree items: 4330. Now: none. Penny: Old Spanish (medieval
  Castilian) period.
kaw — Old Javanese (regular). Tree items: 3277. Now: none. The earliest documents in
  Old Javanese: first attestation (an 804 inscription was found in a Japanese paper).
fa-cls — Classical Persian (etymology-only, a variety filed under fa Persian). Tree
  items: 3086. Now: none. The Iranian Languages, Persian and Tajik: Classical
  Persian's period (Paul says no single classical standard existed; say so if the
  source agrees).
unm — Unami (regular). Tree items: 2723. Now: none. Grumet: earliest Unami (Delaware)
  materials and their dates.
pal — Middle Persian (regular). Varieties with their own codes: pal-ear Early Middle
  Persian, pal-lat Late Middle Persian, xmn Manichaean Middle Persian. Tree items:
  2721. Now: none. The Iranian Languages, Middle West Iranian chapter (Skjærvø): the
  period of Middle Persian; early/late Middle Persian and Manichaean Middle Persian
  are varieties.
la-med — Medieval Latin (etymology-only, a variety filed under la Latin). Tree items:
  2602. Now: none. The Medieval Latin abstract: its dating of Medieval Latin.
xqa — Karakhanid (regular). Tree items: 1082. Now: none. The Turkic Languages:
  Karakhanid's attestation or period.
oui — Old Uyghur (regular). Tree items: 1051. Now: none. The Turkic Languages: Old
  Uyghur's attestation or period.
xfa — Faliscan (regular). Varieties with their own codes: xfa-cap Capenate. Tree
  items: 159. Now: from 7th century BCE (attested, Clackson & Horrocks 2007). Bakkum:
  the latest Faliscan texts (Clackson & Horrocks end in the 3rd c. BC, Fortson in the
  2nd).
emy — Epigraphic Mayan (regular). Tree items: 80. Now: none. Houston, Robertson &
  Stuart: the language of the Classic inscriptions and its dates (Woodard's Stela 29,
  AD 292, was left unresolved).
och-lat — Late Old Chinese (etymology-only, a variety filed under och Old Chinese).
  Tree items: 2. Now: none. Pulleyblank: Late Old Chinese / Classical Chinese period.
och-ear — Early Old Chinese (etymology-only, a variety filed under och Old Chinese).
  Now: none. Pulleyblank: Early Old Chinese / pre-Classical periodization.
egy-mid — Middle Egyptian (etymology-only, a variety filed under egy Egyptian). Now:
  none. Loprieno, Ancient Egyptian: Middle Egyptian's period (Woodard's quote was
  garbled by OCR).
```
