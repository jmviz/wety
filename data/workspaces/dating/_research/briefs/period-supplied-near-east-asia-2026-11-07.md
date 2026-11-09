# Batch brief: Supplied sources: Near East, Africa and Asia

Generated from `packages/dating/prompts/period-verification.md` for run `period-supplied-near-east-asia-2026-11-07`;
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
The works requested by the last batches, now supplied (use library with the language name or the work's title: it searches each holding's table of contents): Rilly & de Voogt, The Meroitic Language and Writing System (2012); Vincent W. J. van Gerven Oei, "Old Nubian" (UCLA Encyclopedia of Egyptology, 2024); Fassberg, "Jewish Palestinian Aramaic: Chronology, Geography, and Typology" (2021; an abstract-only holding: cite it as the article's abstract); Weninger et al. (eds.), The Semitic Languages: An International Handbook (2011; chapters on Samaritan Aramaic, Mandaic and the other Aramaic varieties); Interconnected Traditions: Semitic Languages, Literatures, Cultures (Open Book Publishers, two volumes; Gzella's chapter "Christian Palestinian Aramaic between Greek and Arabic"); Sokoloff, A Dictionary of Jewish Babylonian Aramaic of the Talmudic and Geonic Periods (introduction); Ben-Hayyim & Tal, A Grammar of Samaritan Hebrew (2000); Erdal, A Grammar of Old Turkic (2004); Janhunen (ed.), The Mongolic Languages (2003; chapter on Written Mongol); Sonntag (ed.), The Politics of Language Contact in the Himalaya (2019; the chapter "What Happened to the Ahom Language?"); Vajda (ed.), The Languages and Linguistics of Northern Asia (2024; the Yeniseian chapter); von Hinüber, A Handbook of Pāli Literature (1996). Open a holding with open, then use find (language names with "century", "BC", "period", "attested", "texts", "inscriptions", "dated", "speakers", "extinct") rather than paging through it with read. find gives the PDF page and the printed page label; cite the printed page. Check the title page of each holding you cite (identity and edition); for a chapter, give its author and title within the volume. Where find or page flags OCR or a scanned text layer, view the page image with page before relying on the quote, and say so in self-check.json.
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

pi — Pali (regular). Tree items: 11862. Now: none. von Hinüber: the chronology of the
  Pāli canon and the oldest surviving Pāli manuscripts and inscriptions. Record
  composition, redaction and first attestation as separate events; the Buddha's
  lifetime is not a date of Pāli.
oui — Old Uyghur (regular). Tree items: 1051. Now: none. Erdal, introduction: Old
  Uyghur's documentary window (earliest documents, latest manuscripts up to the 17th
  c.?), beyond the 9th–13th c. Turfan corpus an earlier run found.
cmg — Classical Mongolian (regular). Tree items: 730. Now: none. Janhunen (ed.),
  Written Mongol chapter: whether Classical Mongolian is a bounded 17th–19th c. stage
  or the continuing Written Mongol register; keep Preclassical/Middle Mongol apart.
otk — Old Turkic (regular). Varieties with their own codes: otk-kir Old Kirghiz,
  otk-ork Orkhon Turkic. Tree items: 706. Now: none. Erdal: the period of Old Turkic
  as a whole (Orkhon inscriptions to the latest Uyghur texts) and whether it includes
  Karakhanid.
zko — Kott (regular). Tree items: 489. Now: none. Vajda (ed.), Yeniseian chapter:
  Kott's first wordlists and the end of native transmission (Castrén's 19th-c.
  fieldwork is not the extinction).
aho — Ahom (regular). Tree items: 426. Now: until c. 1800 CE (last_speaker, Morey
  2015). Sonntag (ed.), "What Happened to the Ahom Language?": when Ahom ceased to be
  spoken natively, apart from court, manuscript and ritual use and later revival (wety
  now shows a last-speaker bound c. 1800 from Morey 2015).
yug — Yug (regular). Tree items: 277. Now: from 18th century CE (attested, Fries
  2024). Vajda (ed.), Yeniseian chapter: Yug's first documentation and last native
  speakers (census claims vs actual speakers).
myz — Classical Mandaic (regular). Tree items: 233. Now: none. The Semitic Languages
  (2011), Mandaic chapter (the requested Routledge chapter is replaced by this one):
  the Classical Mandaic stage's period or attestation (an earlier run found only "from
  the 7th–8th centuries onwards").
xpm — Pumpokol (regular). Tree items: 214. Now: none. Vajda (ed.), Yeniseian chapter:
  Pumpokol's documentation (18th c. wordlists) and extinction.
xrn — Arin (regular). Tree items: 202. Now: until 18th century CE (last_speaker, Georg
  2007). Vajda (ed.), Yeniseian chapter: Arin's documentation and extinction (wety now
  shows an 18th-c. last-speaker bound from Georg 2007).
onw — Old Nubian (regular). Tree items: 183. Now: none. van Gerven Oei 2024: Old
  Nubian's attestation window (late 8th – 15th c. in discovery metadata; earlier runs
  found conflicting latest dates 1035/1053 and a questioned 1483/84 endpoint).
tmr — Jewish Babylonian Aramaic (etymology-only, a variety filed under arc Aramaic).
  Tree items: 178. Now: c. 200–900 CE (conventional, Damsma 2018). Sokoloff,
  introduction: the dates of the Talmudic and Geonic corpus of Jewish Babylonian
  Aramaic, without treating composition, manuscripts and native use as one.
jpa — Jewish Palestinian Aramaic (etymology-only, a variety filed under arc Aramaic).
  Tree items: 82. Now: c. 200–700 CE (conventional, Damsma 2018). Fassberg 2021
  abstract: its chronology of Jewish Palestinian Aramaic, to test Damsma's schematic
  c. 200–700 CE now shown.
sam — Samaritan Aramaic (regular). Tree items: 59. Now: none. Tal, "Samaritan Aramaic"
  in The Semitic Languages (2011): its period, and Arabic's displacement of the spoken
  language vs continued written and liturgical use.
xmr — Meroitic (regular). Tree items: 54. Now: c. 300 BCE – 400 CE (attested, Rilly
  2012). Rilly & de Voogt: first and last Meroitic inscriptions and whether the
  often-cited c. 300 BC – AD 400 is a script window; keep cessation of writing apart
  from native transmission.
arc-jla — Jewish Literary Aramaic (etymology-only, a variety filed under arc Aramaic).
  Tree items: 53. Now: c. 200 BCE – 200 CE (conventional, Damsma 2018). Fassberg 2021
  abstract, if it dates Jewish Literary Aramaic separately; otherwise leave it.
smp — Samaritan Hebrew (regular). Tree items: 40. Now: none. Ben-Hayyim & Tal:
  Samaritan Hebrew as an inherited stage vs the recitation tradition; any dated
  textual window or end of vernacular use.
arc-cpa — Christian Palestinian Aramaic (etymology-only, a variety filed under arc
  Aramaic). Tree items: 37. Now: none. Gzella, Interconnected Traditions: Christian
  Palestinian Aramaic's documentary period; keep native vernacular cessation apart
  from later manuscript production.
```
