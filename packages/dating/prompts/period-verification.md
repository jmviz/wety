# Period verification brief (dating pilot)

Brief for `harness.frontier_research prepare --brief`. It replaces the family template
for one bounded run that checks display periods for a fixed list of Wiktionary codes.
Tools, provenance, limits and `check` are those of the
[single frontier researcher](single-frontier.md). Output is a proposal: Claude (a
separate session) reviews it, re-checks every adopted quote against the retained text,
and only then imports claims and records decisions.

The code table was generated from the dating generation current on 2026-10-03
(`catalog_context` of `wiktionary_dates.json`, Wiktionary export `languages.json`).

```text
Verify when each of the Wiktionary language codes below was spoken, for display on wety's
etymology trees ({{FAMILY}}). Work autonomously as one researcher, without subagents or
other model/API calls. Do not redesign tools, run comparisons or start other runs.

What wety will show. For each code, one interval: start and end, each as a range of
years; a basis (conventional period, attestation window, or estimate); and citations.
You are checking the evidence for that interval, not writing it. spoken stays null in
your records; the integrator derives display intervals only from reviewed claims.

For each code:
1. Verify the existing candidate claims listed for it against the actual source text.
   Acquire the source, find the passage, and quote it exactly from retained text with a
   locator (read offsets, page, table row). Verdict per existing claim: confirmed,
   corrected (state what differs: numbers, event, era, scope), contradicted, or
   unverifiable in this run (say why: not openly available, blocked, not found). Never
   reproduce a quotation from memory. A recorded quote in the table below is NOT
   evidence: several were written by an earlier model from memory.
2. Confirm the subject mapping to the Wiktionary code. Wiktionary's scope for each code
   is given below, including the etymology-only varieties it files under the code
   (e.g. Wiktionary "la" also covers Late, Medieval, New Latin). Mapping is exact only if
   the source's dated entity has the same scope as the Wiktionary code. Otherwise say
   subset, superset or overlap and why. A period for Classical Latin is a subset of
   "la", not its period. Catalog names and codes (ISO 639-3, Glottolog) are evidence
   about identity, not automatically about dates.
3. Where no usable claim exists, look for one: a conventional period from a scholarly
   handbook, reference grammar, encyclopedia of languages or standards registry; or an
   attestation window (first/last surviving texts) from a scholarly source.
4. Flag conflicts between sources (e.g. Old English ends 1100 vs 1150). Do not average or
   choose by recency; keep each source's period as a separate claim.
5. Propose at most one default per code and event, with a rationale, and keep the
   others as alternatives.

Proto-languages (gem-pro, ine-pro, sla-pro, cel-pro): use only dates a source gives for
that proto stage itself (e.g. "Proto-Germanic was spoken c. ... "). Never use a family's
crown or stem divergence date, a phylogenetic split, a daughter's first attestation, or
archaeological culture dates unless the source itself explicitly dates the proto stage
with them. Such periods are always estimates (role hypothesis), even when stated
confidently. Record competing proposals separately.

Source access. Only openly reachable HTTPS sources can be read. Useful starting points:
- the ISO 639-3 code tables from SIL (names carry date ranges, e.g. "Old English (ca.
  450-1100)") and their code-to-name identity;
- Glottolog languoid pages or its CLDF data (documentation windows; check whether a
  value is first or last documentation, and whether its sign or era was inverted);
- open-access handbook chapters, encyclopedia entries and articles found via search;
- Wikipedia only to check the existing src_enwiki_infobox claims, or to locate the
  scholarly works it cites. Wikipedia periods may be kept as alternatives, never as a
  proposed default if a scholarly or standards source exists.
Book handbooks in the table (Baugh & Cable 2013, Schmidt 2013, Clackson & Horrocks 2007,
Horrocks 2010) are probably not open. Try one route per book (an OpenAlex open-access
location or publisher preview); if it fails, mark the claim unverifiable in this run.
One acquisition can serve many codes (a registry table, a CLDF file): prefer those.
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
   source_id (from acquire); quote (exact substring of the retained text, long enough to
   include the dates and the entity name, no invented line breaks); locator (read
   offsets start/end, page or table row); subject {"code", "source_label", "mapping":
   "exact|subset|superset|overlap|unresolved", "mapping_basis"}; event; role; method;
   native_time; qualifications (list); verifies (list of {"claim": existing claim id
   prefix from the table, "verdict": "confirmed|corrected|contradicted", "note"}) or
   [] for new evidence; proposed_status ("supported" only if quote, numbers, event and
   exact mapping are all checked, else "candidate"); proposed_decision ({"default":
   true, "rationale"} on at most one assertion per code and event, else null); citation
   (full bibliographic citation of the source as published).
2. report.md: a table with one row per code (existing-claim verdicts, proposed default,
   alternatives, conflicts, mapping problems), then the unverifiable claims and the
   remaining gaps.
3. sources.json: source_id, citation, URL, what was read (body/table/page), access
   problems, and sources attempted but not read.
4. self-check.json: per assertion, checks of quote, numbers and era, event/role,
   mapping and the verdicts on existing claims, with any corrections made.
Call check after saving, fix problems once, and finish. Passing check does not
establish correctness. Finish with a short summary of what is verified per code.

Codes and existing candidate claims (claim id prefix, method/event, years as recorded,
registered source, recorded quote, which may be wrong):

### ang — Old English (Wiktionary kind: regular; family gmw; parents -)
- Wiktionary aliases: Anglo-Saxon
- Etymology-only varieties Wiktionary files under this code: Anglian Old English, Mercian Old English, Northumbrian Old English
- Wikipedia article named by Wiktionary: Old English language; catalog link: olde1238
- `ev_521568031e02a1bf` conventional_period/conventional_period 401 CE – 1200 CE; source src_enwiki_infobox (English Wikipedia, Infobox language `era` / `extinct` fields); recorded quote: "[[5th century]]–[[12th century]]; mostly developed into [[Middle English]]."
- `ev_580d6417f41347f1` conventional_period/documentary_sampling 450 CE – 1100 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Old English (ca. 450-1100): First_Year_Of_Documentation=450, Last_Year_Of_Documentation=1100"
- `ev_c643e784d5a65d79` conventional_period/conventional_period 450 CE – 1150 CE; source src_baugh_cable_2013 (Baugh, A. C. & Cable, T. 2013. A History of the English Language, 6th ed. Routledge. §§ 4, 8, 12 (periods: Old English 450–1150, Middle English 1150–1500, Modern English 1500–).); recorded quote: "Old English 450–1150; Middle English 1150–1500; Modern English 1500–"

### enm — Middle English (Wiktionary kind: regular; family gmw; parents ['ang'])
- Wiktionary aliases: Medieval English, Mediaeval English
- Etymology-only varieties Wiktionary files under this code: Early Scots, Northern Middle English
- Wikipedia article named by Wiktionary: Middle English language; catalog link: midd1317
- `ev_3bb84c8667a61df7` conventional_period/documentary_sampling 1100 CE – 1500 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Middle English: First_Year_Of_Documentation=1100, Last_Year_Of_Documentation=1500"
- `ev_adeca71b18fb14fb` conventional_period/conventional_period 1150 CE – 1500 CE; source src_baugh_cable_2013 (Baugh, A. C. & Cable, T. 2013. A History of the English Language, 6th ed. Routledge. §§ 4, 8, 12 (periods: Old English 450–1150, Middle English 1150–1500, Modern English 1500–).); recorded quote: "Old English 450–1150; Middle English 1150–1500; Modern English 1500–"

### goh — Old High German (Wiktionary kind: regular; family gmw; parents -)
- Etymology-only varieties Wiktionary files under this code: Lombardic
- Wikipedia article named by Wiktionary: Old High German language; catalog link: oldh1241
- `ev_912c1d9edfab44a7` conventional_period/conventional_period 750 CE – 1050 CE; source src_schmidt_2013 (Schmidt, W. 2013. Geschichte der deutschen Sprache, 11th ed. Hirzel. (Althochdeutsch 750–1050, Mittelhochdeutsch 1050–1350, Frühneuhochdeutsch 1350–1650, Neuhochdeutsch 1650–.)); recorded quote: "Althochdeutsch 750–1050, Mittelhochdeutsch 1050–1350, Frühneuhochdeutsch 1350–1650, Neuhochdeutsch 1650–"
- `ev_ae476493becd4e77` conventional_period/documentary_sampling 750 CE – 1050 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Old High German (ca. 750-1050): First_Year_Of_Documentation=750, Last_Year_Of_Documentation=1050"

### gmh — Middle High German (Wiktionary kind: regular; family gmw; parents ['goh'])
- Etymology-only varieties Wiktionary files under this code: none
- Wikipedia article named by Wiktionary: Middle High German language; catalog link: midd1343
- `ev_5348068daf8753fb` conventional_period/documentary_sampling 1050 CE – 1500 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Middle High German: First_Year_Of_Documentation=1050, Last_Year_Of_Documentation=1500"
- `ev_b706f429c07f65e1` conventional_period/conventional_period 1050 CE – 1350 CE; source src_schmidt_2013 (Schmidt, W. 2013. Geschichte der deutschen Sprache, 11th ed. Hirzel. (Althochdeutsch 750–1050, Mittelhochdeutsch 1050–1350, Frühneuhochdeutsch 1350–1650, Neuhochdeutsch 1650–.)); recorded quote: "Althochdeutsch 750–1050, Mittelhochdeutsch 1050–1350, Frühneuhochdeutsch 1350–1650, Neuhochdeutsch 1650–"

### non — Old Norse (Wiktionary kind: regular; family gmq; parents -)
- Wiktionary aliases: Old Icelandic, Old Norwegian
- Etymology-only varieties Wiktionary files under this code: Old East Norse, Old Gutnish, Old West Norse
- Wikipedia article named by Wiktionary: Old Norse language; catalog link: oldn1244
- `ev_de5c44c425a4a55d` conventional_period/documentary_sampling 800 CE – 1150 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Old Norse: First_Year_Of_Documentation=800, Last_Year_Of_Documentation=1150"

### sga — Old Irish (Wiktionary kind: regular; family cel-gae; parents ['pgl'])
- Etymology-only varieties Wiktionary files under this code: none
- Wikipedia article named by Wiktionary: Old Irish language; catalog link: oldi1245
- `ev_517bac7ac6b1e7b7` conventional_period/documentary_sampling 500 CE – 900 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Early Irish: First_Year_Of_Documentation=500, Last_Year_Of_Documentation=900"
- `ev_c1e044a89b175049` conventional_period/conventional_period 501 CE – 1000 CE; source src_enwiki_infobox (English Wikipedia, Infobox language `era` / `extinct` fields); recorded quote: "6th–10th century; evolved into [[Middle Irish]] by around the 10th century"

### got — Gothic (Wiktionary kind: regular; family gme; parents -)
- Etymology-only varieties Wiktionary files under this code: none
- Wikipedia article named by Wiktionary: Gothic language; catalog link: goth1244
- `ev_bfd6f252bd2d4583` conventional_period/documentary_sampling 350 CE – 380 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Gothic: First_Year_Of_Documentation=350, Last_Year_Of_Documentation=380"
- `ev_d4334c6411f08ecd` conventional_period/conventional_period 201 CE – 1000 CE; source src_enwiki_infobox (English Wikipedia, Infobox language `era` / `extinct` fields); recorded quote: "Attested 3rd–10th century; [[Crimean Gothic|related dialects]] survived until 18th century in Crimea"

### cu — Old Church Slavonic (Wiktionary kind: regular; family zls; parents -)
- Wiktionary aliases: Old Church Slavic
- Etymology-only varieties Wiktionary files under this code: Middle Bulgarian
- Wikipedia article named by Wiktionary: Old Church Slavonic language; catalog link: chur1257
- `ev_370d3c2790315987` conventional_period/documentary_sampling 900 CE – 1100 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Church Slavic: First_Year_Of_Documentation=900, Last_Year_Of_Documentation=1100"
- `ev_62e4a83ac8b1276c` conventional_period/conventional_period 801 CE – 1100 CE; source src_enwiki_infobox (English Wikipedia, Infobox language `era` / `extinct` fields); recorded quote: "9th–11th centuries; then evolved into several variants of [[Church Slavonic]] including [[Middle Bulgarian]]"

### akk — Akkadian (Wiktionary kind: regular; family sem-eas; parents -)
- Etymology-only varieties Wiktionary files under this code: Late Babylonian, Middle Assyrian, Middle Babylonian, Neo-Assyrian, Neo-Babylonian, Old Akkadian, Old Assyrian, Old Babylonian, Standard Babylonian
- Wikipedia article named by Wiktionary: Akkadian language; catalog link: akka1240
- `ev_0ec32a9cdfb3f1ef` attestation_last/last_attestation 2350 BCE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Akkadian: First_Year_Of_Documentation=, Last_Year_Of_Documentation=-2350"
- `ev_4081c68992951f14` conventional_period/conventional_period 2600 BCE – 500 BCE; source src_enwiki_infobox (English Wikipedia, Infobox language `era` / `extinct` fields); recorded quote: "{{circa|2600}}–500 BC; academic or liturgical use until AD 100<ref>{{cite web|url=http://multitree.org/codes/akk |archive-url=https://web.archive.org/web/201108"

### hit — Hittite (Wiktionary kind: regular; family ine-ana; parents -)
- Wiktionary aliases: Hettite, Kanesite, Kaneshite, Kanesian, Kaneshian, Nesian, Neshian, Nesite, Nessite, Neshite
- Etymology-only varieties Wiktionary files under this code: none
- Wikipedia article named by Wiktionary: Hittite language; catalog link: hitt1242
- `ev_0a0e996a762a4472` conventional_period/documentary_sampling 1650 BCE – 1180 BCE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Hittite: First_Year_Of_Documentation=-1650, Last_Year_Of_Documentation=-1180"
- `ev_aa009824b06685dc` conventional_period/conventional_period 1600 BCE – 1200 BCE; source src_enwiki_infobox (English Wikipedia, Infobox language `era` / `extinct` fields); recorded quote: "attested {{circa|1600|1200 BC}}"

### la — Latin (Wiktionary kind: regular; family itc; parents ['itc-ola'])
- Etymology-only varieties Wiktionary files under this code: Classical Latin, Ecclesiastical Latin, Late Latin, Medieval Latin, New Latin, Old Latin, Renaissance Latin, Vulgar Latin
- Wikipedia article named by Wiktionary: Latin language; catalog link: lati1261
- `ev_cca428e7df82845a` conventional_period/documentary_sampling 100 BCE – 100 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Latin: First_Year_Of_Documentation=-100, Last_Year_Of_Documentation=100"
- `ev_deed12835fe90cd7` conventional_period/conventional_period 100 BCE – 200 CE; source src_clackson_horrocks_2007 (Clackson, J. & Horrocks, G. 2007. The Blackwell History of the Latin Language. Blackwell. (Archaic/Old Latin to c. 100 BCE; Classical Latin c. 100 BCE–c. 200 CE; Late Latin c. 200–c. 600.)); recorded quote: "Archaic/Old Latin to c. 100 BCE; Classical Latin c. 100 BCE–c. 200 CE; Late Latin c. 200–c. 600"

### grc — Ancient Greek (Wiktionary kind: regular; family grk; parents -)
- Wiktionary aliases: Classical Greek
- Etymology-only varieties Wiktionary files under this code: Aeolic Greek, Arcadian Greek, Arcadocypriot Greek, Attic Greek, Boeotian Greek, Byzantine Greek, Cretan Ancient Greek, Doric Greek, Elean Greek, Epic Greek, Ionic Greek, Koine Greek, Opuntian Locrian, Ozolian Locrian, Pamphylian Greek, Thessalian Greek
- Wikipedia article named by Wiktionary: Ancient Greek language; catalog link: anci1242
- `ev_b9e352b67bb90fe0` conventional_period/documentary_sampling 500 BCE – 300 BCE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Ionic-Attic Ancient Greek: First_Year_Of_Documentation=-500, Last_Year_Of_Documentation=-300"
- `ev_dcf2f6e2cd5234da` conventional_period/conventional_period 800 BCE – 300 BCE; source src_horrocks_2010 (Horrocks, G. 2010. Greek: A History of the Language and its Speakers, 2nd ed. Wiley-Blackwell. (Ancient Greek to c. 300 BCE; Koine c. 300 BCE–c. 600 CE; Medieval/Byzantine Greek c. 600–1453; Modern Greek 1453–.)); recorded quote: "Ancient Greek to c. 300 BCE; Koine c. 300 BCE–c. 600 CE; Medieval/Byzantine Greek c. 600–1453; Modern Greek 1453–"

### sa — Sanskrit (Wiktionary kind: regular; family inc-old; parents -)
- Etymology-only varieties Wiktionary files under this code: Brahmanic Sanskrit, Buddhist Hybrid Sanskrit, Classical Sanskrit, New Sanskrit, Vedic Sanskrit
- Wikipedia article named by Wiktionary: Sanskrit language; catalog link: sans1269
- `ev_29c9e0dd7a9e484e` conventional_period/conventional_period 1500 BCE – 600 BCE; source src_enwiki_infobox (English Wikipedia, Infobox language `era` / `extinct` fields); recorded quote: "{{circa}} 1500–600 BCE (Vedic Sanskrit);<ref>{{cite book|author=Uta Reinöhl |title=Grammaticalization and the Rise of Configurationality in Indo-Aryan |url=http"
- `ev_f001d8212584ed7c` conventional_period/documentary_sampling 1500 BCE – 1100 BCE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Sanskrit: First_Year_Of_Documentation=-1500, Last_Year_Of_Documentation=-1100"

### ltc — Middle Chinese (Wiktionary kind: regular; family zhx; parents ['och'])
- Etymology-only varieties Wiktionary files under this code: Early Middle Chinese, Late Middle Chinese
- Wikipedia article named by Wiktionary: Middle Chinese language; catalog link: midd1344
- `ev_ef5248bf278e57a8` conventional_period/documentary_sampling 601 CE – 1150 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Middle Chinese: First_Year_Of_Documentation=601, Last_Year_Of_Documentation=1150"

### och — Old Chinese (Wiktionary kind: regular; family zhx; parents -)
- Etymology-only varieties Wiktionary files under this code: Early Old Chinese, Late Old Chinese
- Wikipedia article named by Wiktionary: Old Chinese language; catalog link: oldc1244
- `ev_a9c01e1cff158ae1` conventional_period/documentary_sampling 1200 BCE – 400 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Old Chinese: First_Year_Of_Documentation=-1200, Last_Year_Of_Documentation=400"

### ojp — Old Japanese (Wiktionary kind: regular; family jpx; parents -)
- Etymology-only varieties Wiktionary files under this code: Early Middle Japanese, Late Middle Japanese, Middle Japanese
- Wikipedia article named by Wiktionary: Old Japanese language; catalog link: oldj1239
- `ev_7180bd3fcb62e939` conventional_period/conventional_period 701 CE – 800 CE; source src_enwiki_infobox (English Wikipedia, Infobox language `era` / `extinct` fields); recorded quote: "8th century"
- `ev_bcebe5432b93cd2a` attestation_last/last_attestation 712 CE; source src_glottolog_5_3 (Hammarström, Harald & Forkel, Robert & Haspelmath, Martin & Bank, Sebastian. 2026. Glottolog 5.3. Leipzig: Max Planck Institute for Evolutionary Anthropology.); recorded quote: "Old Japanese: First_Year_Of_Documentation=, Last_Year_Of_Documentation=712"

### gem-pro — Proto-Germanic (Wiktionary kind: reconstructed; family gem; parents -)
- Wiktionary aliases: Common Germanic
- Etymology-only varieties Wiktionary files under this code: none
- Wikipedia article named by Wiktionary: Proto-Germanic language; catalog link: none
- Existing candidate claims: none (new sources needed)

### ine-pro — Proto-Indo-European (Wiktionary kind: reconstructed; family ine; parents -)
- Etymology-only varieties Wiktionary files under this code: none
- Wikipedia article named by Wiktionary: Proto-Indo-European language; catalog link: none
- Existing candidate claims: none (new sources needed)

### sla-pro — Proto-Slavic (Wiktionary kind: reconstructed; family sla; parents -)
- Wiktionary aliases: Common Slavic
- Etymology-only varieties Wiktionary files under this code: none
- Wikipedia article named by Wiktionary: Proto-Slavic language; catalog link: none
- Existing candidate claims: none (new sources needed)

### cel-pro — Proto-Celtic (Wiktionary kind: reconstructed; family cel; parents -)
- Etymology-only varieties Wiktionary files under this code: none
- Wikipedia article named by Wiktionary: Proto-Celtic language; catalog link: none
- Existing candidate claims: none (new sources needed)

```
