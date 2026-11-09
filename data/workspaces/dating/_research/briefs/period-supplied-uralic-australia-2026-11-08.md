# Batch brief: Supplied sources: Uralic proto-stages and Australian languages

Generated from `packages/dating/prompts/period-verification.md` for run `period-supplied-uralic-australia-2026-11-08`;
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
Works the maintainer supplied after the last batches (use library with the language name or the work's title: it searches each holding's table of contents): Abondolo & Valijärvi (eds.), The Uralic Languages, 2nd ed. (Routledge 2023), the full volume (holding 008277b740fc0188), with chapter 3 (Zhivlov, Reconstruction of Proto-Uralic) and the language chapters on Saami, Finnic, Moksha Mordvin, Mari, Udmurt, Zyrian Komi, Mansi, Khanty, Nganasan, Enets, Nenets and Selkup; Bartens, Permiläisten kielten rakenne ja kehitys (2000, in Finnish); Dixon, Australian Languages: Their Nature and Development (2002); Bowern & Koch (eds.), Australian Languages: Classification and the Comparative Method (2004); Dixon & Blake (eds.), Handbook of Australian Languages, volumes 1–3. For a reconstructed stage, a date the source gives for its breakup may be recorded as crown_divergence (role hypothesis, with the source's method) when the source dates the breakup of that stage itself; a date of a daughter's first attestation or of a loan contact is not one. Open a holding with open, then use find (the stage's name with "BC", "BP", "AD", "millennium", "century", "years ago", "dated", "split", "disintegrat", "loanwords", "speakers", "extinct", "last") rather than paging through it with read. find gives the PDF page and the printed page label; cite the printed page. Check the title page of each holding you cite (identity and edition); for a chapter, give its author and title within the volume. Where find or page flags OCR or a scanned text layer, view the page image with page before relying on the quote, and say so in self-check.json.
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

urj-fin-pro — Proto-Finnic (reconstructed). Tree items: 3884. Now: until c. 740 CE
  (estimated, Cultural and climatic changes shape the evolutionary history of the
  Uralic languages 2013). Abondolo & Valijärvi (eds.), The Uralic Languages, 2nd ed.
  (2023), now supplied in full (holding 008277b740fc0188; the earlier 104-page holding
  703425ea22a8408c is partial: do not use it): the Finnic chapter: any dating of
  Proto-Finnic or its breakup.
smi-pro — Proto-Samic (reconstructed). Tree items: 2048. Now: until c. 1140 CE
  (estimated, Cultural and climatic changes shape the evolutionary history of the
  Uralic languages 2013). Abondolo & Valijärvi (eds.), The Uralic Languages, 2nd ed.
  (2023), now supplied in full (holding 008277b740fc0188; the earlier 104-page holding
  703425ea22a8408c is partial: do not use it): the Saami chapters and the
  introduction: any dating of Proto-Saami or its breakup.
wyi — Woiwurrung (regular). Tree items: 1678. Now: until c. 20th century CE
  (last_speaker, Hercus 1986). Dixon 2002 and Bowern & Koch 2004: anything on
  Woiwurrung (Melbourne language, Kulin): earliest documentation (e.g. word lists of
  the 1830s–1840s, Mathews's 1902 grammar) and the end of native transmission (wety
  now shows a last-speaker bound in the 20th century from Hercus 1986). Blake's
  Handbook vol. 4 chapter is not available.
urj-pro — Proto-Uralic (reconstructed). Varieties with their own codes: fiu-pro
  Proto-Finno-Ugric, urj-fpr-pro Proto-Finno-Permic. Tree items: 1192. Now: until c.
  3320 BCE (estimated, Cultural and climatic changes shape the evolutionary history of
  the Uralic languages 2013). Abondolo & Valijärvi (eds.), The Uralic Languages, 2nd
  ed. (2023), now supplied in full (holding 008277b740fc0188; the earlier 104-page
  holding 703425ea22a8408c is partial: do not use it): chapter 3 (Zhivlov) and the
  introduction: any dating of Proto-Uralic or of its breakup, with its method and time
  scale (years BP need their epoch).
urj-prm-pro — Proto-Permic (reconstructed). Tree items: 844. Now: c. 800–200 BCE
  (estimated, Parpola 2017). Abondolo & Valijärvi (eds.), The Uralic Languages, 2nd
  ed. (2023), now supplied in full (holding 008277b740fc0188; the earlier 104-page
  holding 703425ea22a8408c is partial: do not use it): the Udmurt and Zyrian Komi
  chapters; and Bartens 2000 pp. 9–13 (an earlier run recorded her Proto-Permic
  breakup 'in the 700s' CE and her 'Proto-Permic period continued into the middle of
  the first millennium AD and even after that'). Which period of Proto-Permic does
  current scholarship give, and on what evidence (loanword strata such as Volga Bulgar
  or Iranian loans, the breakup into Udmurt and Komi)? Record each dated statement
  separately.
syd-pro — Proto-Samoyedic (reconstructed). Tree items: 712. Now: until c. 1st century
  BCE (estimated, Jaakko Häkkinen 2009). Abondolo & Valijärvi (eds.), The Uralic
  Languages, 2nd ed. (2023), now supplied in full (holding 008277b740fc0188; the
  earlier 104-page holding 703425ea22a8408c is partial: do not use it): the Nganasan,
  Enets, Nenets and Selkup chapters and the introduction (an earlier run found
  'separation of Proto-Samoyedic ca 2500 BP' on p. 14, event unclear): any dating of
  Proto-Samoyedic or its breakup.
urj-mdv-pro — Proto-Mordvinic (reconstructed). Tree items: 710. Now: until c. 1000 CE
  (estimated, Jaakko Häkkinen 2012). Abondolo & Valijärvi (eds.), The Uralic
  Languages, 2nd ed. (2023), now supplied in full (holding 008277b740fc0188; the
  earlier 104-page holding 703425ea22a8408c is partial: do not use it): the Moksha
  Mordvin chapter: any dating of Proto-Mordvin(ic) or its breakup.
sel-pro — Proto-Selkup (reconstructed). Tree items: 567. Now: none. Abondolo &
  Valijärvi (eds.), The Uralic Languages, 2nd ed. (2023), now supplied in full
  (holding 008277b740fc0188; the earlier 104-page holding 703425ea22a8408c is partial:
  do not use it): the Selkup chapter: any dating of Proto-Selkup or its breakup.
awg — Anguthimri (regular). Tree items: 483. Now: none. Handbook of Australian
  Languages vol. 2 (Crowley, 'The Mpakwithi dialect of Anguthimri') was read already:
  one known Mpakwithi speaker 'today' (1981). Dixon 2002 and Bowern & Koch 2004:
  Anguthimri's documentation history and the end of native transmission for the
  language as a whole.
chm-pro — Proto-Mari (reconstructed). Tree items: 379. Now: until after c. 1300 CE
  (estimated, Holopainen & Niklas Metsäranta 2020). Abondolo & Valijärvi (eds.), The
  Uralic Languages, 2nd ed. (2023), now supplied in full (holding 008277b740fc0188;
  the earlier 104-page holding 703425ea22a8408c is partial: do not use it): the Mari
  chapter: any dating of Proto-Mari or its breakup (wety now shows an end after c.
  1300 CE).
mns-pro — Proto-Mansi (reconstructed). Tree items: 285. Now: none. Abondolo &
  Valijärvi (eds.), The Uralic Languages, 2nd ed. (2023), now supplied in full
  (holding 008277b740fc0188; the earlier 104-page holding 703425ea22a8408c is partial:
  do not use it): the Mansi chapter: any dating of Proto-Mansi or its breakup.
fiu-pro — Proto-Finno-Ugric (etymology-only, a variety filed under urj-pro
  Proto-Uralic). Tree items: 262. Now: until c. 3000 BCE (estimated, Petri Kallio
  2006). Abondolo & Valijärvi (eds.), The Uralic Languages, 2nd ed. (2023), now
  supplied in full (holding 008277b740fc0188; the earlier 104-page holding
  703425ea22a8408c is partial: do not use it): chapter 3 and the introduction: any
  dating of Proto-Finno-Ugric as a stage (the 2023 introduction questions some
  intermediate nodes: say so if it does).
urj-fpr-pro — Proto-Finno-Permic (etymology-only, a variety filed under urj-pro
  Proto-Uralic). Tree items: 235. Now: until c. 2000 BCE (estimated, Petri Kallio
  2006). Abondolo & Valijärvi (eds.), The Uralic Languages, 2nd ed. (2023), now
  supplied in full (holding 008277b740fc0188; the earlier 104-page holding
  703425ea22a8408c is partial: do not use it): chapter 3 and the introduction: any
  dating of Proto-Finno-Permic, and whether the volume accepts the node.
kca-pro — Proto-Khanty (reconstructed). Tree items: 224. Now: none. Abondolo &
  Valijärvi (eds.), The Uralic Languages, 2nd ed. (2023), now supplied in full
  (holding 008277b740fc0188; the earlier 104-page holding 703425ea22a8408c is partial:
  do not use it): the Khanty chapter: any dating of Proto-Khanty or its breakup.
urj-ugr-pro — Proto-Ugric (reconstructed). Tree items: 194. Now: until c. 1480 BCE
  (estimated, Václav Blažek 2012). Abondolo & Valijärvi (eds.), The Uralic Languages,
  2nd ed. (2023), now supplied in full (holding 008277b740fc0188; the earlier 104-page
  holding 703425ea22a8408c is partial: do not use it): the introduction, chapter 3,
  the Mansi and Khanty chapters: any dating of Proto-Ugric (the introduction quotes
  Saarikivi that it would be older than Proto-Samoyedic).
```
