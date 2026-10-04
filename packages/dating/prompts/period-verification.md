# Period verification brief (dating pilot)

Brief for `harness.frontier_research prepare --brief`. It replaces the family template
for one bounded run that checks display periods for the pilot's Wiktionary codes.
Tools, provenance, limits and `check` are those of the
[single frontier researcher](single-frontier.md). Output is a proposal: Claude (a
separate session) reviews it, re-checks every adopted quote against the retained text,
and only then imports claims and records decisions.

This version (2026-10-04, third round) follows the maintainer's supply of the six works
the retry requested and five volumes of Woodard's *Ancient Languages* series
(`dating holdings list`). The code notes summarize the
[period pilot](../../../data/workspaces/dating/_research/period-pilot-2026-10-03/README.md),
[scope check](../../../data/workspaces/dating/_research/scope-check-2026-10-03/README.md)
and [retry](../../../data/workspaces/dating/_research/period-retry-2026-10-04/README.md);
Wiktionary scope is from the legacy export `languages.json`. Earlier versions are in git
history (the 2026-10-03 pilot and the 2026-10-04 retry).

```text
Verify when each of the Wiktionary language codes below was spoken, for display on wety's
etymology trees ({{FAMILY}}). Work autonomously as one researcher, without subagents or
other model/API calls. Do not redesign tools, run comparisons or start other runs.

What wety will show. For each code, one interval: start and end, each as a range of
years; a basis (conventional period, attestation window, or estimate); and citations.
You are checking the evidence for that interval, not writing it. spoken stays null in
your records; the integrator derives display intervals only from reviewed claims.

The main task: six codes have no displayable period yet (non got cu sa ojp ine-pro).
Fourteen have one, but several rest on ISO 639-3 reference names or Glottolog
documentation windows, which the maintainer accepts only when nothing better exists
(enm gmh sga grc och ltc), and several give only a start or only an end (goh la sga grc
sla-pro). A previous run (the retry) read Barnes, Fortson, Frellesvig, Huehnergard,
Kapović, Lunt and Miller and requested six works; the maintainer has now supplied all
six, plus five volumes of Woodard's Ancient Languages series. This run's task is to read
the newly supplied works for every code they bear on, and to report what they state
about the period of the whole code. Do not repeat the retry's searches of the earlier
holdings except to resolve a conflict with a new source.

For each code:
1. Look for a scholarly statement of the period or attestation window of the entity the
   code covers: a conventional period from a handbook or reference grammar, or the
   first/last surviving texts. Quote it exactly from retained text with a locator
   (printed page, and PDF page). Never reproduce a quotation from memory.
2. Mapping. Wiktionary's scope for each code is given below, including the
   etymology-only varieties it files under the code. A period is usable for display only
   if the source's dated entity covers the whole code: mapping exact. Say subset,
   superset or overlap otherwise, and why. A period for Classical Latin is a subset of
   "la"; an Old Japanese period is a subset of "ojp", which also files Middle Japanese.
   Never join several stage periods into one interval yourself (e.g. Old + Middle
   Japanese): record each stage's period as its own subset claim, and say in the report
   whether the source states one span for the whole. If no source covers the whole
   code, that is a recorded gap, not a failure.
3. Record conflicts between sources (e.g. Old English ends 1100 vs 1150) as separate
   claims. Do not average or choose by recency.
4. Propose at most one default per code and event, with a rationale, and keep the
   others as alternatives. Prefer a scholarly handbook statement over ISO 639-3 names
   and Glottolog documentation windows when both have exact scope.
5. For cu and sa, also quote what a handbook says each stage covers (for cu: whether
   Old Church Slavonic includes Middle Bulgarian or later Church Slavonic recensions;
   for sa: whether Sanskrit includes Vedic, Buddhist Hybrid and later learned Sanskrit).
   These quotes settle catalog scope (ISO 639-3 chu/san, Glottolog chur1257/sans1269),
   so record them as scope assertions (see Deliverables).

Proto-languages (gem-pro, ine-pro, sla-pro, cel-pro): use only dates a source gives for
that proto stage itself (e.g. "Proto-Slavic was spoken c. ... "). Never use a family's
crown or stem divergence date, a phylogenetic split, a daughter's first attestation, or
archaeological culture dates unless the source itself explicitly dates the proto stage
with them. Such periods are always estimates (role hypothesis), even when stated
confidently. Record competing proposals separately.

Source access, in this order.
1. Supplied sources first. Call library (no query lists everything). New in this round:
   Baugh & Cable, A History of the English Language (ang, enm; the supplied file is a
   later edition than the 6th requested: cite the edition its title and copyright pages
   state); Schmidt, Geschichte der deutschen Sprache, 11th ed. 2013 (goh, gmh; German);
   Clackson & Horrocks, The Blackwell History of the Latin Language (la); Horrocks,
   Greek: A History of the Language and its Speakers, 2nd ed. (grc); Sun, Chinese: A
   Linguistic Introduction (och, ltc); and Woodard (ed.), Cambridge 2008, five volumes
   reprinting The Cambridge Encyclopedia of the World's Ancient Languages: The Ancient
   Languages of Asia and the Americas (Jamison, Sanskrit: sa; Peyraube, Ancient Chinese:
   och; Ringe, appendix on reconstructed languages: ine-pro and other proto-languages),
   of Europe (Woodard, Attic Greek and Greek dialects: grc; Clackson, Latin: la; Eska,
   Continental Celtic: cel-pro; Jasanoff, Gothic: got; Faarlund, Ancient Nordic: non;
   appendix on Indo-European: ine-pro), of Asia Minor (Watkins, Hittite: hit), of
   Mesopotamia, Egypt and Aksum (Huehnergard & Woods, Akkadian and Eblaite: akk; its
   text layer is poor, so verify quotes against page images), and of Syria-Palestine
   and Arabia (no pilot code). Supplied earlier and already read by the retry: Barnes
   (non), Fortson (IE), Frellesvig (ojp), Huehnergard (akk), Kapović (ed.) (IE), Lunt
   (cu) and Miller (got). Open a holding with open,
   then use find (stage names with "century", "BC", "period", "attested", "spoken",
   "dated") rather than paging through it with read. find gives the PDF page and the
   printed page label; cite the printed page. Check the title page of each holding
   you cite (identity and edition). Where find or page flags OCR or a scanned text layer,
   view the page image with page before relying on the quote, and say so in
   self-check.json.
2. Openly reachable sources: the ISO 639-3 code tables and Glottolog CLDF (already in
   the registry: use them only for a new question), open-access chapters and articles.
   search indexes and web search (if enabled) give leads, never evidence: acquire a
   source before quoting it.
3. A work you need but cannot read: try distinct routes once each (publisher page,
   open-access locations from openalex or semanticscholar, institutional repositories,
   the author's homepage via s2author or web search, Open Library public scans). Do not
   retry a blocked route. Then call request: say exactly what the work would settle, for
   which codes, which part is needed, and the routes tried. Request only works that
   would change a code's result, not every search hit. The maintainer obtains them
   later; record the gap in the report. The pilot's recorded quotes from Baugh & Cable,
   Schmidt, Clackson & Horrocks and Horrocks were written from memory and are not
   evidence; the supplied copies now let you check them.
Check status early and periodically; save partial files early; deliver partial work
rather than exceeding limits.

Time semantics. Record native_time exactly as the source states it:
{"form": "duration" | "event_range" | "point" | "bound",
 "unit": "CE" | "BCE" | "century_CE" | "century_BCE" | "millennium_BCE",
 "lower": n, "upper": n, "value": n, "relation": "before|after|by|since",
 "precision": "year|decade|century|millennium", "approximate": true|false,
 "expression": "the source's own words for the dates"}
A period "c. 450–1150" is form duration, unit CE, lower 450, upper 1150, approximate
true. "5th–12th century" is unit century_CE, lower 5, upper 12. BCE years are positive
numbers with unit BCE (lower = the earlier endpoint as written, e.g. 2600 for "2600–500
BC"). If the endpoints are in different eras (e.g. "1250 BC – AD 220"), give
{"form": "duration", "start": {"value": 1250, "unit": "BCE"}, "end": {"value": 220,
"unit": "CE"}, ...}. Never convert to other years yourself; conversion is programmatic.
"To 900" is an end with no stated start: say so; do not invent a start.

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
   bibliographic citation of the source as published, with edition).
   Scope assertions (step 5) use the same fields with event "unspecified", role
   "observation", method "scholarly_description", native_time {"form": "unknown"},
   plus target (e.g. "iso639-3:chu", "glottolog:chur1257") and what_it_shows (one
   sentence).
2. report.md: a table with one row per code (best whole-code evidence, its mapping,
   proposed default, alternatives, conflicts, remaining gap), then the cu/sa scope
   findings, then sources requested from the maintainer and why.
3. sources.json: source_id, citation, URL or holding, what was read (pages), access
   problems, and sources attempted but not read.
4. self-check.json: per assertion, checks of quote, numbers and era, event/role,
   mapping, title-page identity of the source, and page-image checks for OCR text.
Call check after saving, fix problems once, and finish. Passing check does not
establish correctness. Finish with a short summary of what is verified per code.

Codes. Wiktionary scope (legacy export), the period wety shows now, and what is open.

### Codes without a period

non — Old Norse (aliases Old Icelandic, Old Norwegian). Filed under non: Old East
  Norse, Old West Norse, Old Gutnish. Glottolog oldn1244, ISO non.
  Now: none. Barnes p. 1 dates a broad Old Norse (c. 750–1050 and c. 1050–1350) that
  may include colonial varieties beyond non; Glottolog 800–1150 is a documentation
  window. New: Faarlund (Ancient Nordic) dates the runic predecessor; say whether it
  bears on non's start, and look for any period of Old Norse as a whole.
got — Gothic. Nothing filed under got. Glottolog goth1244, ISO got.
  Now: none. Miller gives witness dates (Codex Argenteus c. 520, Bologna fragment,
  Crimean graffiti 9th or 10th century) but no whole attestation or spoken span.
  New: Jasanoff (Gothic). Needed: a stated attestation window or period of Gothic;
  how the source treats Crimean Gothic.
cu — Old Church Slavonic. Filed under cu: Middle Bulgarian. Glottolog chur1257 "Church
  Slavic", ISO chu "Church Slavic".
  Now: none. Lunt p. 4: an arbitrary pre-1100 canon that excludes Middle Bulgarian. No
  new holding targets cu; record only what a new source states incidentally.
sa — Sanskrit. Filed under sa: Vedic, Brahmanic, Classical, Buddhist Hybrid, New
  Sanskrit. Glottolog sans1269, ISO san.
  Now: none. Kulikov (in Kapović) allows Sanskrit as a cover term for Vedic and
  post-Vedic; Buddhist Hybrid Sanskrit coverage is open. New: Jamison (Sanskrit; the
  retry's request). Needed: a statement covering Sanskrit from Vedic onwards, or a
  recorded gap; and the scope quotes of step 5.
ojp — Old Japanese. Filed under ojp: Middle Japanese, Early and Late Middle Japanese.
  Glottolog oldj1239, ISO ojp.
  Now: none. Frellesvig p. 1 dates OJ 700–800, EMJ 800–1200, LMJ 1200–1600 separately,
  with no one span. No new holding targets ojp.
ine-pro — Proto-Indo-European (reconstructed). Now: none. Fortson §2.58 gives only a
  lower bound (latest pre-breakup PIE not earlier than c. 3500–3400 BCE), with the
  with-/without-Anatolian mapping open. New: Ringe's appendix on reconstructed
  languages (Asia and the Americas) and the Indo-European appendix (Europe). Needed:
  dates they give for PIE itself and which PIE they date.

### Codes with a period (look for a better or confirming source)

goh — Old High German. Filed under goh: Lombardic. Glottolog oldh1241, ISO goh.
  Shown: attested from c. 600 CE (Fortson §15.70; §15.71 counts Langobardic as OHG),
  no end. ISO and Glottolog 750–1050 do not cover Lombardic. New: Schmidt (requested).
  Needed: Schmidt's OHG period and its treatment of Langobardisch.
ang — Old English. Filed: Anglian, Mercian, Northumbrian Old English. Shown: 5th–11th
  century (Slocum & Lehmann, EIEOL). New: Baugh & Cable (requested). Needed: their
  Old English period, checked against the pilot's unverified 450–1150.
enm — Middle English. Filed: Early Scots, Northern Middle English. Shown: ISO "Middle
  English (1100-1500)". New: Baugh & Cable. Needed: their Middle English period and
  whether it covers early Scots / northern English.
gmh — Middle High German. Shown: ISO "ca. 1050-1500" (the maintainer kept ISO over
  Wright 1917's 1100–1500). New: Schmidt. Needed: Schmidt's MHG period (the pilot's
  1050–1350 is unverified).
sga — Old Irish. Shown: ISO "Old Irish (to 900)", end only (Sims-Williams: earliest
  manuscripts 8th century, not adopted). Eska's Continental Celtic chapter is not
  about Old Irish; record only what a new source states.
grc — Ancient Greek. Filed: all ancient dialects, Koine and Byzantine Greek. Shown: ISO
  "Ancient Greek (to 1453)", end only. New: Horrocks (requested), Woodard's Attic
  Greek and Greek dialects. Needed: a period covering Koine and Byzantine for a start,
  or each stage as its own subset claim. Mycenaean has its own Wiktionary code (gmy).
hit — Hittite. Shown: attested c. 19th century – 1200 BCE (Kapović p. 3 for names,
  Fortson §9.17 for the end of text production). New: Watkins (Hittite). Confirm or
  contradict.
och — Old Chinese. Filed: Early and Late Old Chinese. Shown: Glottolog 1200 BCE – 400
  CE (as attestation). New: Sun (requested), Peyraube (Ancient Chinese). Needed: their
  periodization of Old Chinese as a whole.
ltc — Middle Chinese. Filed: Early and Late Middle Chinese. Shown: Glottolog 601–1150
  (as attestation). New: Sun, Peyraube. Needed: their Middle Chinese period.
la — Latin. Filed: Old, Classical, Late, Vulgar, Ecclesiastical, Medieval, Renaissance,
  New Latin. Shown: attested from the mid-7th century BCE (Wallace in Kapović p. 322),
  no end. New: Clackson & Horrocks (requested; chapter VIII on late antiquity and
  beyond), Clackson's Latin chapter. Needed: whether either states a span or end for
  Latin including its medieval and later learned use.
akk — Akkadian. Filed: Old Akkadian, Old/Middle/Neo-Assyrian, Old/Middle/Neo-/Late
  Babylonian, Standard Babylonian. Shown: attested 26th century BCE – 1st century CE
  (Huehnergard 2011 pp. xxiii–xxiv). New: Huehnergard & Woods (Akkadian and Eblaite).
  Confirm or contradict.
sla-pro — Proto-Slavic (alias Common Slavic). Shown: estimated, until c. 800/1000 CE
  (Fortson §18.29), no start. No new holding targets it.
gem-pro — Proto-Germanic. Shown: Koch 2020 estimate (start c. 500/400 BC, end 300 BC –
  AD 1). No new holding targets it.
cel-pro — Proto-Celtic. Shown: Koch 2020 estimate c. 1200–900 BC. New: Eska
  (Continental Celtic): any date for Proto-Celtic itself.
```
