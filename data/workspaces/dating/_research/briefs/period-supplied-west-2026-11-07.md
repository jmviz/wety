# Batch brief: Supplied sources: Europe, the Americas, Australia and proto-languages

Generated from `packages/dating/prompts/period-verification.md` for run `period-supplied-west-2026-11-07`;
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
The works requested by the last batches, now supplied (use library with the language name or the work's title: it searches each holding's table of contents): Moul (ed.), A Guide to Neo-Latin Literature (2017; Moul's Introduction); Eska, "A salvage grammar of Galatian" (ZCPh 2013); Hill, The N-Factor and Russian Prepositions (1977; chapter 14, "Middle Russian (1400–1699)"); Dixon & Blake (eds.), Handbook of Australian Languages, volumes 1–3 (Crowley's "The Mpakwithi dialect of Anguthimri" is in volume 2); Drechsel, Mobilian Jargon (1997); Golla, California Indian Languages (2011); Piispanen, "Additional Turkic and Tungusic borrowings into Yukaghir" (2018); Blust, "The Resurrection of Proto-Philippines" (Oceanic Linguistics 2019); Klimov, Etymological Dictionary of the Kartvelian Languages (1998); Bartens, Permiläisten kielten rakenne ja kehitys (2000, in Finnish); Abondolo & Valijärvi (eds.), The Uralic Languages (2nd ed., 2023); Nichols, Ingush Grammar (2011); Janhunen (ed.), The Mongolic Languages (2003; chapter 1, Proto-Mongolic). Open a holding with open, then use find (language names with "century", "BC", "period", "attested", "texts", "inscriptions", "dated", "speakers", "extinct") rather than paging through it with read. find gives the PDF page and the printed page label; cite the printed page. Check the title page of each holding you cite (identity and edition); for a chapter, give its author and title within the volume. Where find or page flags OCR or a scanned text layer, view the page image with page before relying on the quote, and say so in self-check.json.
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
(role hypothesis) for when a language stopped being spoken natively (a revived
language too: the end is that of native transmission); revitalization (role
observation) for a dated statement that a language is being revived (a programme's
start, new speakers, or a year it is reported ongoing); literary_use for a written or
learned register. Attestation is not the spoken period: say so in
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

qfa-yuk-pro — Proto-Yukaghir (reconstructed). Tree items: 2745. Now: until c. 30 BCE
  (estimated, Eric W. Holman et al. 2011). Piispanen 2018: any date the article gives
  for Proto-Yukaghir itself (his 2019 sequel points to this part for dating), not a
  loan contact or a modern split.
phi-pro — Proto-Philippine (reconstructed). Tree items: 1632. Now: until c. 170 CE
  (estimated, Eric W. Holman et al. 2011). Blust 2019: whether he dates
  Proto-Philippines (or the levelling event) absolutely; his 2013 handbook's nearby
  1500 BC concerns the Chamorro migration, not Proto-Philippines.
la-new — New Latin (etymology-only, a variety filed under la Latin). Tree items: 1488.
  Now: none. Moul, Introduction: a definition and chronological scope of Neo-Latin;
  say whether a stated start (and any end) dates the stage or only the book's
  selection.
xgn-pro — Proto-Mongolic (reconstructed). Tree items: 1485. Now: until c. 1150 CE
  (estimated, Jaakko Häkkinen 2009). Janhunen (ed.), chapter 1 pp. 1–3: an explicit
  dating of Proto-Mongolic, and whether it is an end bound or a period.
urj-prm-pro — Proto-Permic (reconstructed). Tree items: 844. Now: c. 800–200 BCE
  (estimated, Parpola 2017). Bartens 2000 pp. 9–22 (Hausenberg's review reports a
  Proto-Permic period lasting to the middle of the first millennium and later); The
  Uralic Languages, Permic chapter. Quote the Finnish exactly and give a translation
  in qualifications.
syd-pro — Proto-Samoyedic (reconstructed). Tree items: 712. Now: until c. 1st century
  BCE (estimated, Jaakko Häkkinen 2009). The Uralic Languages, Samoyedic chapter and
  introduction: any date of Proto-Samoyedic as a stage.
urj-mdv-pro — Proto-Mordvinic (reconstructed). Tree items: 710. Now: until c. 1000 CE
  (estimated, Jaakko Häkkinen 2012). The Uralic Languages, Mordvin chapter: any date
  of Proto-Mordvin(ic).
ccs-gzn-pro — Proto-Georgian-Zan (reconstructed). Tree items: 590. Now: until c. 670
  BCE (estimated, Alexander Gavashelishvili et al. 2023). Klimov 1998: any date of
  Proto-Georgian-Zan; keep Georgian-Zan apart from Proto-Georgian.
sel-pro — Proto-Selkup (reconstructed). Tree items: 567. Now: none. The Uralic
  Languages, Samoyedic/Selkup chapter: any date of Proto-Selkup.
ccs-pro — Proto-Kartvelian (reconstructed). Tree items: 513. Now: until c. 5690 BCE
  (estimated, Alexander Gavashelishvili et al. 2023). Klimov 1998, introduction: any
  date of Proto-Kartvelian as a spoken stage, not only Svan's separation.
cau-nkh-pro — Proto-Nakh (reconstructed). Tree items: 512. Now: until c. 1st century
  CE (estimated, Global Lexicostatistical Database). Nichols, Ingush Grammar,
  introduction: any date of Proto-Nakh itself (a Nakh-Daghestanian family age is not
  one).
awg — Anguthimri (regular). Tree items: 483. Now: none. Handbook of Australian
  Languages vol. 2, Crowley: Anguthimri's documentation history and native speakers; a
  Mpakwithi-only record is a subset of awg.
chm-pro — Proto-Mari (reconstructed). Tree items: 379. Now: until after c. 1300 CE
  (estimated, Holopainen & Niklas Metsäranta 2020). The Uralic Languages, Mari
  chapter: any date of Proto-Mari.
zle-mru — Middle Russian (etymology-only, a variety filed under ru Russian). Tree
  items: 322. Now: none. Hill, ch. 14: whether "Middle Russian (1400–1699)" is a
  conventional period of the stage or only the study's corpus division; check how the
  book defines its periods.
mns-pro — Proto-Mansi (reconstructed). Tree items: 285. Now: none. The Uralic
  Languages, Mansi chapter: any date of Proto-Mansi.
fiu-pro — Proto-Finno-Ugric (etymology-only, a variety filed under urj-pro
  Proto-Uralic). Tree items: 262. Now: until c. 3000 BCE (estimated, Petri Kallio
  2006). The Uralic Languages, introduction: any date of Proto-Finno-Ugric as a stage
  (a divergence date alone is not one).
urj-fpr-pro — Proto-Finno-Permic (etymology-only, a variety filed under urj-pro
  Proto-Uralic). Tree items: 235. Now: until c. 2000 BCE (estimated, Petri Kallio
  2006). The Uralic Languages, introduction: any date of Proto-Finno-Permic.
mod — Mobilian (regular). Tree items: 204. Now: none. Drechsel: Mobilian Jargon's
  earliest records and last users ("at least 1700 until the mid-twentieth century" in
  metadata, unverified); a pidgin has no native transmission: say so.
wiy — Wiyot (regular). Tree items: 198. Now: none. Golla: Wiyot's documentation and
  its last fluent speaker.
urj-ugr-pro — Proto-Ugric (reconstructed). Tree items: 194. Now: until c. 1480 BCE
  (estimated, Václav Blažek 2012). The Uralic Languages, introduction or Ugric
  chapters: any date of Proto-Ugric.
wlk — Wailaki (regular). Tree items: 156. Now: none. Golla: Wailaki (Eel River
  Athabaskan) documentation and last speakers; broad vs narrow Wailaki scope.
wao — Wappo (regular). Tree items: 100. Now: none. Golla: Wappo's documentation and
  last fluent speaker (a 1990 claim is unverified).
css — Southern Ohlone (regular). Tree items: 83. Now: none. Golla: Southern Ohlone
  (Mutsun and Rumsen) documentation and last speakers; say which variety each date
  concerns.
kla — Klamath-Modoc (regular). Tree items: 80. Now: none. Golla: Klamath-Modoc
  documentation and last speakers.
xga — Galatian (regular). Tree items: 16. Now: none. Eska 2013: first/last Galatian
  witnesses and any survival endpoint (Jerome's 4th-c. remark is a report, not a
  witness; say so if relied on).
```
