# Scope verification brief (dating pilot follow-up)

Brief for `harness.frontier_research prepare --brief`. One bounded run settles how
four Wiktionary codes relate in scope to their ISO 639-3 and Glottolog entries, so a
period a source states for one can (or cannot) be shown for the other. It also checks
that current Wiktionary still files four varieties the way the legacy export does. The
reviewer records accepted relations in `data/workspaces/catalogs/overrides.json`.
Context: [period pilot](../../../data/workspaces/dating/_research/period-pilot-2026-10-03/README.md).

```text
Settle concept scope for wety's dating display ({{FAMILY}}). Work autonomously as one
researcher, without subagents or other model/API calls. Do not redesign tools.

Question. wety will show a period for a Wiktionary language code only when the source's
dated entity has the same scope as the whole code, including the etymology-only
varieties Wiktionary files under it. For each code below and each catalog target
(ISO 639-3 code, Glottolog languoid), decide the relation, from Wiktionary's side:
- equivalent: same language/stage and same extension;
- source_broader: the Wiktionary code covers more (e.g. later stages filed under it);
- source_narrower: the Wiktionary code covers less (the target is an umbrella);
- overlap: each covers something the other does not;
- unresolved: the evidence read does not settle it (say what would).
This is identity and extension, not dates. Do not research periods.

Evidence to use, all openly reachable:
- Glottolog: the languoid page (https://glottolog.org/resource/languoid/id/GLOTTOCODE),
  which shows its dialect children, and the CLDF table
  https://raw.githubusercontent.com/glottolog/glottolog-cldf/v5.3/cldf/languages.csv.
  Use the find action on that table (search by glottocode) rather than paging with read.
  Children rows show what the languoid includes.
- ISO 639-3: the code pages (https://iso639-3.sil.org/code/CODE) for reference name,
  scope, type and any denotation or comments; the code table
  https://iso639-3.sil.org/sites/iso639-3/files/downloads/iso-639-3.tab.
- Current Wiktionary data: the language and etymology-language data modules on
  en.wiktionary.org (e.g. Module:etymology_languages/data and Module:languages/data/2,
  or the module pages that define these codes). The table below comes from a legacy
  export of unknown date; say whether current Wiktionary still files each variety
  under the same code.
- A scholarly description of the stage, only where needed to say what the catalog
  entity's name means (e.g. whether "Church Slavic" covers later recensions).

Also check, in current Wiktionary data only, whether these are still filed as below.
Earlier catalog decisions rest on them:
- Early Scots (sco-osc) under enm; Lombardic (goh-lng / lng) under goh;
- Middle Japanese (ja-mid, ja-mid-ear, ja-mid-lat) under ojp;
- Koine Greek (grc-koi) and Byzantine Greek (gkm) under grc.

Deliverables (write action; these four names only):
1. assertions.jsonl: one object per piece of quoted evidence, with id; code; target
   (e.g. "glottolog:chur1257", "iso639-3:chu" or "wikt:cu"); source_id; quote (exact
   substring of retained text); locator; subject (what the quote describes); event
   "unspecified"; role "observation"; method "catalog_scope" (or "scholarly_description");
   native_time {"form": "unknown"}; qualifications (list); what_it_shows (one sentence).
2. report.md: a table with one row per code and target: proposed relation, the
   assertion ids it rests on, and the reasoning in two or three sentences. Then the
   four filing checks (still filed / changed / not found), then gaps.
3. sources.json: source_id, citation, URL, what was read, access problems.
4. self-check.json: per proposed relation, whether the quoted evidence supports it.
Call check after saving, fix problems once, and finish with a short summary.

Codes (Wiktionary scope from the legacy export, and current catalog targets):

### cu: Old Church Slavonic
- Wiktionary (legacy export): family zls; parents -; aliases Old Church Slavic
- Etymology-only varieties filed under cu: Middle Bulgarian (cu-bgm)
- Catalog target glottolog:chur1257: Church Slavic (catalog_link)
- Catalog target iso639-3:chu: Church Slavic (catalog_link)

### sa: Sanskrit
- Wiktionary (legacy export): family inc-old; parents -; aliases -
- Etymology-only varieties filed under sa: Brahmanic Sanskrit (sa-bra), Buddhist Hybrid Sanskrit (sa-bhs), Classical Sanskrit (sa-cls), New Sanskrit (sa-neo), Vedic Sanskrit (sa-ved)
- Catalog target glottolog:sans1269: Sanskrit (catalog_link)
- Catalog target iso639-3:san: Sanskrit (catalog_link)

### la: Latin
- Wiktionary (legacy export): family itc; parents ['itc-ola']; aliases -
- Etymology-only varieties filed under la: Classical Latin (la-cla), Ecclesiastical Latin (la-ecc), Late Latin (la-lat), Medieval Latin (la-med), New Latin (la-new), Old Latin (itc-ola), Renaissance Latin (la-ren), Vulgar Latin (la-vul)
- Catalog target glottolog:lati1261: Latin (catalog_link)
- Catalog target iso639-3:lat: Latin (catalog_link)

### non: Old Norse
- Wiktionary (legacy export): family gmq; parents -; aliases Old Icelandic, Old Norwegian
- Etymology-only varieties filed under non: Old East Norse (non-oen), Old Gutnish (non-ogt), Old West Norse (non-own)
- Catalog target iso639-3:non: Old Norse (catalog_link)
- Catalog target glottolog:oldn1244: Old Norse (catalog_link)
```
