# Period verification brief (dating pilot)

Brief for `harness.frontier_research prepare --brief`. It replaces the family template
for one bounded run that checks display periods for the pilot's Wiktionary codes.
Tools, provenance, limits and `check` are those of the
[single frontier researcher](single-frontier.md). Output is a proposal: Claude (a
separate session) reviews it, re-checks every adopted quote against the retained text,
and only then imports claims and records decisions.

This version (2026-10-04) is the retry after the maintainer supplied handbooks as
holdings (`dating holdings list`). The code notes summarize the
[period pilot](../../../data/workspaces/dating/_research/period-pilot-2026-10-03/README.md)
and [scope check](../../../data/workspaces/dating/_research/scope-check-2026-10-03/README.md);
Wiktionary scope is from the legacy export `languages.json`. The first version is in git
history (used by the 2026-10-03 pilot run).

```text
Verify when each of the Wiktionary language codes below was spoken, for display on wety's
etymology trees ({{FAMILY}}). Work autonomously as one researcher, without subagents or
other model/API calls. Do not redesign tools, run comparisons or start other runs.

What wety will show. For each code, one interval: start and end, each as a range of
years; a basis (conventional period, attestation window, or estimate); and citations.
You are checking the evidence for that interval, not writing it. spoken stays null in
your records; the integrator derives display intervals only from reviewed claims.

The main task: nine codes have no displayable period yet (goh non got cu akk sa ojp
ine-pro sla-pro). Eleven have one, mostly from ISO 639-3 reference names or Glottolog
documentation windows, which the maintainer accepts only when nothing better exists
(ang enm gmh sga grc hit och ltc la gem-pro cel-pro). The maintainer has now supplied
scholarly handbooks. For every code, find what those handbooks (and other readable
scholarly sources) state about the period of the whole code.

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
1. Supplied sources first. Call library (no query lists everything). The maintainer has
   supplied, among others: Barnes, A New Introduction to Old Norse (non); Fortson,
   Indo-European Language and Culture (ine-pro and every IE branch); Frellesvig, A
   History of the Japanese Language (ojp); Huehnergard, A Grammar of Akkadian (akk);
   Kapović (ed.), The Indo-European Languages (ine-pro and branch chapters: Germanic,
   Celtic, Italic, Greek, Indo-Iranian, Anatolian, Slavic); Lunt, Old Church Slavonic
   Grammar (cu); Miller, The Oxford Gothic Grammar (got). Open a holding with open,
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
   later; record the gap in the report. Book handbooks the earlier pilot could not read:
   Baugh & Cable 2013 (ang, enm), Schmidt 2013 (goh, gmh), Clackson & Horrocks 2007
   (la), Horrocks 2010 (grc); their recorded quotes were written from memory and are
   not evidence.
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

goh — Old High German. Filed under goh: Lombardic. Glottolog oldh1241, ISO goh.
  Now: none. ISO "ca. 750-1050" and Glottolog 750–1050 are narrower than goh, which
  also files Lombardic; ISO lists Lombardic separately (lng). Needed: a period stated for Old High German
  that covers Lombardic, or a source saying Lombardic belongs to Old High German.
non — Old Norse (aliases Old Icelandic, Old Norwegian). Filed under non: Old East
  Norse, Old West Norse, Old Gutnish. Glottolog oldn1244, ISO non.
  Now: none. Glottolog 800–1150 is a documentation window; Glottolog's Old Norse also
  includes Old Norn and does not place Old Gutnish under it. Needed: a handbook period
  for Old Norse in the broad sense (East and West Norse, ideally Gutnish).
got — Gothic. Nothing filed under got. Glottolog goth1244, ISO got.
  Now: none. Glottolog 350–380 covers only Wulfila's Bible; Wikipedia "attested 3rd–10th
  century" is not scholarly. Needed: the attestation window of Gothic (Bible, Skeireins,
  Bologna fragment, Crimean graffiti, runic inscriptions) and any statement of when
  Gothic was spoken; Crimean Gothic is a related variety, say how the source treats it.
cu — Old Church Slavonic. Filed under cu: Middle Bulgarian. Glottolog chur1257 "Church
  Slavic", ISO chu "Church Slavic".
  Now: none. An earlier source gave the OCS stage c. 850–1100 (the canon), which
  excludes Middle Bulgarian. Needed: a period covering OCS and Middle Bulgarian, or a
  recorded gap; and the scope quotes of step 5.
akk — Akkadian. Filed under akk: Old Akkadian, Old/Middle/Neo-Assyrian, Old/Middle/
  Neo-/Late Babylonian, Standard Babylonian. Glottolog akka1240, ISO akk.
  Now: none. Glottolog's -2350 is a database field, likely an error. Needed: the
  attestation span of Akkadian as a whole (first Old Akkadian texts to the last
  Late Babylonian texts) and the conventional dialect periods.
sa — Sanskrit. Filed under sa: Vedic, Brahmanic, Classical, Buddhist Hybrid, New
  Sanskrit. Glottolog sans1269, ISO san.
  Now: none. Wikipedia's 1500–600 BCE is Vedic only; Glottolog -1500 to -1100 is a
  documentation window. Needed: a statement covering Sanskrit from Vedic onwards, or a
  recorded gap; and the scope quotes of step 5.
ojp — Old Japanese. Filed under ojp: Middle Japanese, Early and Late Middle Japanese.
  Glottolog oldj1239, ISO ojp.
  Now: none. Wikipedia "8th century" and Glottolog 712 cover Old Japanese only.
  Needed: Frellesvig's periodization; whether any source states one span for Old and
  Middle Japanese together; otherwise each stage as its own subset claim.
ine-pro — Proto-Indo-European (reconstructed). Now: none. An earlier source gave two
  date proposals for the last stage of Proto-Indo-Anatolian (c. 4400–4200 BCE; c.
  43rd–40th century BCE), with mapping to Wiktionary's PIE unresolved (PIE with or
  without Anatolian). Needed: dates Fortson or Kapović give for PIE itself, and which
  PIE (with or without Anatolian) they date.
sla-pro — Proto-Slavic (reconstructed; alias Common Slavic). Now: none; no eligible
  date found so far. Needed: dates a source gives for Proto-Slavic or Common Slavic
  itself (Kapović's Slavic chapter, Fortson's Balto-Slavic chapter), not migration or
  loanword dates.

### Codes with a period (look for a better or confirming source)

ang — Old English. Filed: Anglian, Mercian, Northumbrian Old English. Shown: ISO 639-3
  "Old English (ca. 450-1100)". Baugh & Cable's 450–1150 is unverified.
enm — Middle English. Filed: Early Scots, Northern Middle English. Shown: ISO "Middle
  English (1100-1500)".
gmh — Middle High German. Shown: ISO "ca. 1050-1500". Schmidt's 1050–1350 unverified.
sga — Old Irish. Shown: ISO "Old Irish (to 900)", end only.
grc — Ancient Greek. Filed: all ancient dialects, Koine and Byzantine Greek. Shown: ISO
  "Ancient Greek (to 1453)", end only. A period covering Koine and Byzantine is needed
  for a start.
hit — Hittite. Shown: Glottolog documentation window 1650–1180 BCE (as attestation).
och — Old Chinese. Filed: Early and Late Old Chinese. Shown: Glottolog 1200 BCE – 400
  CE (as attestation).
ltc — Middle Chinese. Filed: Early and Late Middle Chinese. Shown: Glottolog 601–1150
  (as attestation).
la — Latin. Filed: Old, Classical, Late, Vulgar, Ecclesiastical, Medieval, Renaissance,
  New Latin. Shown: first written record c. 550 BC (Koch 2020), no end.
gem-pro — Proto-Germanic. Shown: Koch 2020 estimate (start c. 500/400 BC, end 300 BC –
  AD 1).
cel-pro — Proto-Celtic. Shown: Koch 2020 estimate c. 1200–900 BC.
```
