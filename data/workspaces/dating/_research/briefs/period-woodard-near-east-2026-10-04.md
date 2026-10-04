# Batch brief: Woodard volumes: Mesopotamia, Egypt, Aksum, Syria-Palestine and Arabia

Generated from `packages/dating/prompts/period-verification.md` for run `period-woodard-near-east-2026-10-04`;
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
   Woodard (ed.) 2008, The Ancient Languages of Mesopotamia, Egypt and Aksum (supplied copy: 2010 reprint; its text layer is poor, so verify quotes against
   page images) and Woodard (ed.) 2008, The Ancient Languages of Syria-Palestine and Arabia (both Cambridge, reprinting the 2004 Cambridge Encyclopedia of
   the World's Ancient Languages). Also supplied: Huehnergard 2011, A Grammar of Akkadian.
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

egy — Egyptian (regular). Varieties with their own codes: egy-lat Late Egyptian,
  egy-mid Middle Egyptian, egy-nmi Neo-Middle Egyptian, egy-old Old Egyptian. Tree
  items: 5842. Now: none. Woodard (ed.) 2008, The Ancient Languages of Mesopotamia,
  Egypt and Aksum, ch. 5 (Loprieno, Egyptian and Coptic): the stages of Egyptian and
  their dates; record each stage under its variety code.
arc — Aramaic (regular). Varieties with their own codes: arc-bib Biblical Aramaic,
  arc-cpa Christian Palestinian Aramaic, arc-hat Hatran Aramaic, arc-imp Imperial
  Aramaic, arc-jla Jewish Literary Aramaic, arc-nab Nabataean Aramaic, arc-old Old
  Aramaic, arc-pal Palmyrene Aramaic, jpa Jewish Palestinian Aramaic, tmr Jewish
  Babylonian Aramaic. Tree items: 4074. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Syria-Palestine and Arabia, ch. 6 (Creason, Aramaic): periods of
  Aramaic; record Old, Imperial, Biblical, Nabataean, Palmyrene etc. under their
  variety codes.
cop — Coptic (regular). Varieties with their own codes: cop-akh Akhmimic Coptic,
  cop-boh Bohairic Coptic, cop-fay Fayyumic Coptic, cop-ggg Coptic Dialect G, cop-her
  Hermopolitan Coptic, cop-jjj Coptic Dialect J, cop-kkk Coptic Dialect K, cop-lyc
  Lycopolitan Coptic, cop-old Old Coptic, cop-oxy Oxyrhynchite Coptic, cop-ply
  Proto-Lycopolitan Coptic, cop-ppp Coptic Dialect P, cop-sah Sahidic Coptic. Tree
  items: 3322. Now: none. Woodard (ed.) 2008, The Ancient Languages of Mesopotamia,
  Egypt and Aksum, ch. 5: Coptic's attestation (its dialects have variety codes).
sux — Sumerian (regular). Tree items: 2612. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Mesopotamia, Egypt and Aksum, ch. 2 (Michalowski, Sumerian):
  attestation and the end of spoken Sumerian.
uga — Ugaritic (regular). Tree items: 1097. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Syria-Palestine and Arabia, ch. 2 (Pardee, Ugaritic).
gez — Ge'ez (regular). Tree items: 897. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Mesopotamia, Egypt and Aksum, ch. 6 (Gragg, Ge'ez): earliest
  inscriptions and the end of Ge'ez as a spoken language.
xsa — Sabaean (regular). Tree items: 351. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Syria-Palestine and Arabia, ch. 7 (Nebes & Stein, Ancient South
  Arabian): Sabaean periods.
phn — Phoenician (regular). Tree items: 284. Now: none. Woodard (ed.) 2008, The
  Ancient Languages of Syria-Palestine and Arabia, ch. 4 (Hackett, Phoenician and
  Punic).
hbo — Biblical Hebrew (etymology-only, a variety filed under he Hebrew). Tree items:
  221. Now: none. Woodard (ed.) 2008, The Ancient Languages of Syria-Palestine and
  Arabia, ch. 3 (McCarter, Hebrew): Biblical/Ancient Hebrew (a variety of he).
xpu — Punic (regular). Tree items: 166. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Syria-Palestine and Arabia, ch. 4: Punic.
elx — Elamite (regular). Varieties with their own codes: elx-ach Achaemenid Elamite,
  elx-mid Middle Elamite, elx-neo Neo-Elamite, elx-old Old Elamite. Tree items: 97.
  Now: none. Woodard (ed.) 2008, The Ancient Languages of Mesopotamia, Egypt and
  Aksum, ch. 3 (Stolper, Elamite): Old, Middle, Neo- and Achaemenid Elamite.
xeb — Eblaite (regular). Tree items: 52. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Mesopotamia, Egypt and Aksum, ch. 4 (Huehnergard & Woods, Akkadian and
  Eblaite): Eblaite texts.
obm — Moabite (regular). Tree items: 39. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Syria-Palestine and Arabia, ch. 5 (Pardee, Canaanite dialects):
  Moabite.
inm — Minaean (regular). Tree items: 38. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Syria-Palestine and Arabia, ch. 7: Minaean.
xqt — Qatabanian (regular). Tree items: 34. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Syria-Palestine and Arabia, ch. 7: Qatabanian.
xdm — Edomite (regular). Tree items: 11. Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Syria-Palestine and Arabia, ch. 5: Edomite.
xna — Ancient North Arabian (regular). Tree items: 4. Now: none. Woodard (ed.) 2008,
  The Ancient Languages of Syria-Palestine and Arabia, ch. 8 (Macdonald, Ancient North
  Arabian).
akk-old — Old Akkadian (etymology-only, a variety filed under akk Akkadian). Now:
  none. Woodard (ed.) 2008, The Ancient Languages of Mesopotamia, Egypt and Aksum, ch.
  4 (Huehnergard & Woods) and Huehnergard 2011 p. xxv (mid-3rd to end of 3rd
  millennium, recorded but not yet displayed).
akk-stb — Standard Babylonian (etymology-only, a variety filed under akk Akkadian).
  Now: none. Huehnergard 2011 and Woodard (ed.) 2008, The Ancient Languages of
  Mesopotamia, Egypt and Aksum, ch. 4: Standard Babylonian as a literary language.
sem-srb — Old South Arabian (regular). Now: none. Woodard (ed.) 2008, The Ancient
  Languages of Syria-Palestine and Arabia, ch. 7: Ancient South Arabian as a whole, if
  dated.
sem-amm — Ammonite (regular). Now: none. Woodard (ed.) 2008, The Ancient Languages of
  Syria-Palestine and Arabia, ch. 5: Ammonite.
sem-saf — Safaitic (regular). Now: none. Woodard (ed.) 2008, The Ancient Languages of
  Syria-Palestine and Arabia, ch. 8: Safaitic.
```
