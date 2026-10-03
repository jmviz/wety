# Plan: a dated index of the world's languages (for wety)

Goal: for every language wety can name — modern, historical, and reconstructed — a
machine-readable estimate of *when it was spoken*, with the evidence, the
alternatives, and the reasoning, produced by an agent harness in a way that can be
re-run, audited, and diffed.

This document is the hand-off. Part A fixes the concepts and the schema (the parts
an agent cannot be allowed to improvise). Part B is the pipeline. Part C is what to
run first. The language-code cross-reference in `core/data/langcodes/` is the
foundation and already exists.

> **Implementation:** `dating/` (see its README). Every stage is implemented
> and has been run: the bootstrap (B1) for all seven sources, and the model
> stages (screen, extract, synthesis review) on the Indo-European pilot (C2)
> through OpenRouter (config: `openrouter.config.json`) and the Claude Agent
> SDK. Model choices per stage, and the evidence for them, are in that config.

---

## A. What is actually being dated

### A1. The earlier plan's mistakes, briefly

The plan sketched in the earlier conversation gets the shape right (Glottolog as
the index, literature per family, a rubric, JSON per family, logging) and several
things wrong:

1. **"A start date per language" is not a well-formed question** for most of the
   ~7,900 living languages. There is no paper that says when Ghotuo "started".
   What the literature dates is *splits* (nodes in a family tree) and
   *attestations* (documents). A living language's "origin" is the age of the
   node where it separated from its nearest relatives, and that is a property of
   the node, shared by all its daughters. Date nodes and documents; derive
   language spans from them.
2. **Proto-languages are not dated by "years BP ± range"; they are intervals
   bounded by two node ages.** A proto-language was spoken *after* its ancestor
   split from the sister clade (the node's *stem age*) and *before* its own first
   split (the *crown age*). Phylogenetic papers report crown ages; the interval
   [stem, crown] is what "Proto-Germanic was spoken c. 500 BCE – 200 CE" means.
   The schema must carry both.
3. **A single `confidence_score` collapses things that must stay distinct.** A
   dated inscription, a Bayesian posterior median with a 95% HPD, a
   glottochronological point estimate, and a handbook's conventional period
   boundary are different kinds of claim answering different questions. Store
   the *kind* and the *statistic*; do not average across kinds.
4. **"Temperature 0" is not reproducibility.** LLM outputs are not stable across
   model versions regardless of temperature. Reproducibility comes from pinned
   inputs with hashes, logged queries and retrieved documents, structured claims
   that quote their sources verbatim (mechanically checked against the source
   text), and a final artifact that is a deterministic function of the claims
   table plus written decision rules. The LLM's judgment steps are then a
   logged, re-runnable layer on top, not the whole pipeline.
5. **One task per glottocode is the wrong granularity.** Literature is organised
   by family and subgroup; a paper on Bantu dates a hundred nodes at once. Task
   per family (split into subgroups only for the big ones), not per language.
6. **"Years BP"** is ambiguous (before 1950? before the paper? before 2000?).
   Store calendar years (astronomical numbering, so 1 BCE = 0, 500 BCE = −499,
   or store as `-500` with an explicit convention — pick one and validate it).
   Convert BP at ingestion, recording the paper's own "present".
7. The closing question about PDF cost is real but has a plain answer: metadata
   and abstracts through OpenAlex / Semantic Scholar / Crossref for discovery;
   full text only for a shortlist, via open-access links; cache by hash. Volume
   is a few thousand papers, not hundreds of thousands (§B7).

### A2. Entities

Everything is keyed by **Glottolog languoid** (the crossref maps Wiktionary and
ISO codes onto it). Three kinds of entity, with different date semantics:

| entity | Glottolog | Wiktionary | what "when spoken" means |
|---|---|---|---|
| **attested language / stage** (Old English, Latin, Hittite, Cherokee) | language- or dialect-level languoid | regular or etymology-only code | attestation window (first/last document), extinction, and — for stages of a continuous tradition — conventional period boundaries |
| **living language** (English, Ghotuo) | language-level, AES not extinct | regular code | start = age of its parent node (derived), or a conventional emergence date where the field has one (Afrikaans, Tok Pisin); no end |
| **proto-language / node** (Proto-Germanic, Germanic) | family-level languoid | reconstructed `*-pro` code ↔ family code | [stem age, crown age] interval; crown age from phylogenetics or archaeology-linguistics; both usually with wide intervals |

Two things the harness must not blur:

- **Periodization ≠ divergence.** "Middle English 1100–1500" is a convention of
  English philology (and Glottolog's `First/Last_Year_Of_Documentation` for
  `midd1317` says exactly that). It is a legitimate, citable claim of kind
  `conventional_period`, and it is what wety's etymologies need. It is not a
  measurement.
- **Wiktionary's languages are not always Glottolog's.** `sh`, `zh`, `ar` are
  Glottolog *families*; Wiktionary splits Prakrits and Greek dialects finer than
  Glottolog. The crossref records the level and the relation; the dating layer
  inherits dates from the mapped node and says so.

### A3. Claim kinds and how they are evaluated

Each claim from a source has exactly one `kind`. Kinds are ordered by the weight
they carry *for the question they answer*; they are not ranked against each
other for different questions.

| kind | answers | typical form | weight / caveats |
|---|---|---|---|
| `attestation_first` / `attestation_last` | when documented | a year or range, from dated inscriptions, manuscripts, recordings | highest; but "first attestation" is a lower bound on age, not the age |
| `extinction` | when it stopped being spoken natively | year, often "last speaker died in …" | high when documented; distinguish L1 death from ceremonial/liturgical continuation |
| `conventional_period` | where the field draws stage boundaries | "Old English c. 450–1100" | authoritative for its purpose; cite a handbook; record competing conventions |
| `phylo_node_age` | when a node split (crown) or diverged from sister (stem) | median/mean + 95% HPD, from Bayesian phylogenetics | high for the node, wide intervals; quality depends on calibrations, taxon sampling, cognate coding, clock model; different studies of the same family can differ by thousands of years for structural reasons (IE: Bouckaert 2012 vs Chang 2015 vs Heggarty 2023), so keep all and record the model assumptions |
| `archaeo_linguistic` | when a proto-language was spoken | interval argued from reconstructed vocabulary vs archaeology (the wheel argument for PIE) | medium; explicit reasoning chains, contested |
| `lexicostatistic` | node age | point estimate from glottochronology or calibrated ASJP (Holman et al. 2011) | low but *universal*: the only method with dates for hundreds of families; use as a floor, never over a `phylo_node_age` |
| `expert_estimate` | any | a range asserted in a survey/handbook without a stated method | low–medium; useful when nothing else exists, and often what the phylogenetic calibrations themselves cite |
| `derived` | a language's start | computed from a parent node's age by the harness | not a source claim; recorded with the rule that produced it |

Selection rules the synthesis step applies (write these down in the repo and
apply them mechanically before any LLM judgment):

1. Attestation and extinction claims are facts; conflicting facts are a
   conflict to surface, not to average.
2. For a node age: prefer the most recent peer-reviewed `phylo_node_age` with
   the most attestation-based calibrations and published data/code; if several
   qualify, report all and adopt the one with the widest support, stating why;
   never adopt a `lexicostatistic` date when a `phylo_node_age` exists.
3. For a proto-language interval, stem age from the parent node's study where
   available, else "unknown"; crown age as in rule 2.
4. For a living language's start, `derived` from the parent node's crown age
   unless a `conventional_period` or documented emergence exists.
5. Every adopted value carries the *set of claims considered* and a one-paragraph
   rationale that references them by id.

### A4. Time representation

- Calendar years as integers in astronomical numbering (`-499` = 500 BCE). One
  convention, documented in the schema, validated.
- Precision: `year`, `decade`, `century`, `millennium`; and `value` is the
  midpoint when precision is coarse.
- Intervals: `{"earliest": int, "latest": int}`; distributions for phylogenetic
  claims: `{"median": int, "hpd95": [int, int], "mean": int|null}` plus the
  paper's `present` convention and the conversion applied.
- Open ends: `"latest": null` for living languages.

### A5. Artifact structure

Git-friendly, one directory per top-level Glottolog family (plus `isolates/` and
`unclassified/`), JSON validated against a JSON Schema in `schema/`:

```
data/dating/
  schema/
    languoid.schema.json     claim.schema.json     source.schema.json
  sources/
    sources.json             one record per source: DOI/URL, title, year, type,
                             retrieval date, sha256 of retrieved text, license
  indo1319/                  (Indo-European)
    claims.jsonl             every extracted claim, one per line
    languoids.json           adopted values per glottocode, with rationale
    search_log.jsonl         every query, API, timestamp, hit count, screened ids
  germ1287/ ...              (subgroups only when the family is split into tasks)
  _derived/
    wiktionary_dates.json    the wety-facing view: Wiktionary code -> dates
```

A claim:

```json
{
  "id": "clm_indo1319_00042",
  "languoid": "germ1287",
  "kind": "phylo_node_age",
  "node_role": "crown",
  "time": {"median": -450, "hpd95": [-1200, 100], "present": 2000, "unit": "calendar"},
  "source": "src_10.1126/science.abg0818",
  "locator": "Fig. 2; Table S12 row 'Germanic'",
  "quote": "…the Germanic crown age is estimated at 2.45 kya (95% HPD 2.1–3.2)…",
  "quote_verified": true,
  "method_notes": {"calibrations": 30, "clock": "relaxed", "data": "IE-CoR", "code_available": true},
  "extracted_by": {"model": "…", "run": "run_2026-09-17T01"}
}
```

An adopted record (`languoids.json` entry):

```json
{
  "glottocode": "germ1287",
  "name": "Germanic",
  "level": "family",
  "wiktionary": {"family": "gem", "proto_language": "gem-pro"},
  "spoken": {"earliest": -1500, "latest": -200, "basis": "stem–crown interval"},
  "node_age": {"crown": {"adopted": "clm_indo1319_00042", "alternatives": ["clm_indo1319_00017", "clm_indo1319_00031"]},
               "stem":  {"adopted": "clm_indo1319_00040"}},
  "attestation": null,
  "conventional_period": null,
  "status": "reconstructed",
  "rationale": "Three Bayesian studies date the Germanic crown between 2.0 and 3.0 kya; Heggarty et al. 2023 is adopted as the most recent with the densest attestation calibrations; Chang et al. 2015 (ancestry-constrained) is 300 years younger and within its HPD. Stem age from the same study's Northwest-Germanic/… split. No archaeological claim was found that contradicts this.",
  "confidence": "medium",
  "open_questions": ["Whether Proto-Germanic should be placed at the crown (first split) or somewhat earlier; the interval is reported."],
  "run": "run_2026-09-17T01"
}
```

`quote_verified` is set by a program, not the model: the quote must be found
(after whitespace normalisation) in the retrieved text of the source. Claims
that fail verification are kept but flagged and never adopted.

---

## B. Pipeline

### B0. Foundations (done)

`core/data/langcodes/` gives, for all 8,625 Wiktionary codes and 887 families,
the ISO codes and the Glottolog node, with routes and confidence, plus a
27k-row Glottolog index (`glottolog_langcodes.csv`) carrying each languoid's
parent, classification path, documentation years, endangerment status, and
Wikidata items. Rebuild: `fetch_sources.py` then `build.py`; see its README.

### B1. Bootstrap from existing data (no agent judgment needed)

Ingest as claims, with `source` set to the dataset and version:

| dataset | gives | coverage | kind |
|---|---|---|---|
| Glottolog `First/Last_Year_Of_Documentation` | attestation window | 217 first / 1,076 last, mostly extinct languages; already in `glottolog_langcodes.csv` | `attestation_*` / `conventional_period` (Glottolog's Old English 450–1100 is periodization) |
| Glottolog AES / ISO 639-3 `Language_Type` (L/E/H/A/C) | living vs extinct vs historical vs ancient | universal | status, not a date |
| Wikidata P571 inception, P580/P582 start/end, P576 dissolved, P1249 earliest written record | dates | ~150 items in the current snapshot (`langcodes/sources/wikidata_items.json` already holds them) | `expert_estimate` (uncited Wikidata) unless the statement has a reference |
| English Wikipedia language infobox `era` field | "c. 450 – 1150" style ranges for historical languages | several hundred | `conventional_period`; parse from the infobox wikitext, keep the infobox's own citation if any |
| IE-CoR calibration table (`core/data/iecor_languages.csv`, Heggarty et al. 2023 SI) | tip-date priors (normal / log-normal) for 52 historical IE languages, with justification text | IE | `attestation_*` with the prior's parameters; `match_iecor_wiktionary.py` already joins it to Wiktionary through the crossref |
| Bouckaert et al. 2012 and Chang et al. 2015 SI calibration tables | the same for IE, independently sourced | IE | as above |
| Holman et al. 2011 (ASJP) dates for ~400 families/subgroups | node ages | near-universal | `lexicostatistic` |
| D-PLACE `phylogenies/` (github.com/D-PLACE/dplace-data) | published dated trees (Austronesian, Bantu, IE, Pama-Nyungan, Uto-Aztecan, Arawak, Tupi-Guarani, Semitic, Turkic, Dravidian, Japonic, …) with `taxa.csv` → glottocode | ~15 families | `phylo_node_age` for every internal node, when the tree is time-calibrated (check per tree; some are topology-only) |
| Lexibank / CLDF datasets with dated trees in their releases | same | growing | `phylo_node_age` |

This step alone covers the well-studied families' nodes and most historical
languages. Do it first, because it also gives the harness a ground-truth set to
check its own extraction against (§B6).

### B2. Task generation

One task per family for families with ≤ ~50 languages; split larger families
by Glottolog subgroup (Atlantic-Congo, Austronesian, Sino-Tibetan, Indo-European
each become 5–20 tasks). Order tasks by wety relevance: number of Wiktionary
codes mapped into the subtree × number of etymology edges wety has for those
languages (wety's processed graph can produce this count). Indo-European,
Sino-Tibetan, Afro-Asiatic, Uralic, Turkic, Austronesian, Dravidian, Japonic,
Koreanic, and the Semitic/Egyptian ancient languages will dominate; that is
correct for an etymology site.

Each task receives: the subtree (glottocodes, names, Wiktionary codes, ISO
types), the bootstrap claims already present, and the family's existing Glottolog
reference list (`sources.bib` in the CLDF release, filtered by languoid).

### B3. Discovery

Per task, a **search agent** runs a fixed query template set against OpenAlex
(primary: free, abstracts, OA links, citation graph) and Semantic Scholar
(secondary), logging every query verbatim with timestamp and result count:

- `"<family name>" AND ("phylogen*" OR "Bayesian" OR "divergence" OR "time depth" OR "dating" OR "chronology" OR "homeland")`
- `"Proto-<family>" AND ("date" OR "age" OR "when")`
- for attested languages: `"<language>" AND ("earliest attestation" OR "inscription" OR "first attested" OR "extinct" OR "last speaker")`
- citation walks: everything that cites or is cited by the D-PLACE/IE-CoR source papers for the family; and the references in Glottolog's family entry marked as classification sources.

Semantic Scholar is used for more than search (its key allows exactly one
request per second across every endpoint and every process, and the harness
enforces that with a file-locked clock shared by all runs):

- **bulk search** with boolean syntax and Linguistics/year/open-access filters;
- **batch enrichment** of every candidate (500 per request): S2 ids, PMCID /
  arXiv ids, open-access PDF links, TL;DRs, citation counts;
- **citation walks** from seed papers, keeping the *citation contexts* — the
  sentences in which a citing paper restates the seed's result ("X dated
  Proto-Y to …"), which are themselves dated claims and go to the screener;
- **recommendations** with the screen's includes as positive and excludes as
  negative examples, for a second discovery round;
- **snippet search** (§B4): ~500-word body-text excerpts, restricted to a given
  paper, which turns a paywalled paper into extractable text.

Screening is on title + abstract (+ TL;DR and citation contexts), by a cheap model with a fixed prompt,
producing `{doi, decision: include|exclude, reason}`; every decision is logged,
including excludes. Include threshold: the paper plausibly contains a dated
claim about a node or language in the subtree.

### B4. Retrieval

Full text only for included papers: OA PDF via OpenAlex/Unpaywall/S2 links or
Europe PMC, else HTML, else Semantic Scholar full-text snippets for the paper
(queried with the family's dating vocabulary; excerpts, recorded as such), else
abstract-only (recorded as such). Text extraction with GROBID or
pymupdf. Everything cached under its sha256; the cache is part of the
reproducibility record (store hashes and URLs in git; store the texts outside git
or in LFS depending on licence).

### B4b. Linked data — always look for it

The dated content of a modern paper is mostly *not* in its prose. Calibration
tables, dated trees (MCC summary trees, posterior samples), per-language
metadata with glottocodes, and the README that explains units live in the
deposited data — on Zenodo, GitHub, OSF, Dryad, figshare, or as journal
supplementary files — and those are open even when the paper is paywalled.
For every included paper, whether or not its full text was obtained:

1. **Find the deposits — from the paper first.** The primary route is what the
   paper itself links: data-availability statements and deposit URLs in the
   text and abstract (`zenodo.org/records`, `doi.org/10.5281`, `github.com/…`,
   `osf.io`, `10.5061/dryad`), then the publisher landing page for the DOI
   (visible when the article is paywalled), then Crossref `relation` metadata
   (`is-supplemented-by`, `has-part`). Only as a supplementary route, a Zenodo
   search by the paper's DOI (`related.identifier`), keeping records that carry
   non-PDF files or a supplement-like title and dropping records that merely
   *cite* the paper. GitHub repositories are fetched only when they look like
   language data (description/README about languages, data files present) —
   papers also cite software. Record every link found, fetched or not, with
   where it was found and why it was skipped.
2. **Fetch them** with caps: allowed types only (tables, text, README, trees,
   PDFs), per-file and per-archive size limits, the smallest archive variant
   when several are offered, expanded in memory. Log what was skipped.
3. **Process mechanically what can be processed mechanically**: a tree file
   with branch lengths in years (or thousands of years — infer the unit from the
   root height and record the inference) becomes `phylo_node_age` crown claims
   for every node that maps onto Glottolog, using the same MRCA-and-monophyly
   rule as the D-PLACE bootstrap; taxa map through a companion table with a
   Glottocode column, else by name within the family. No model in the loop.
4. **Give the rest to extraction**: README files, calibration and language
   tables, and supplementary PDFs are attached to the paper's extraction prompt
   as labelled `DATASET FILE` sections; a calibration-table row is a claim and
   is quoted like any other, with the file name and row as locator.
5. **Ask the model to report deposits it saw but could not get** (in the
   paper summary) so they can be fetched by hand.

The Indo-European check: Heggarty et al. 2023's Zenodo supplement yields the
MCC tree whose root (8116 BP) reproduces the paper's headline result without
any text extraction, plus 37 further node ages.

### B5. Extraction

An **extraction agent** reads one document — the paper text followed by its
attached dataset files (B4b) — and emits claims in the schema, each with a
verbatim quote and locator. It is forbidden from emitting a claim without
a quote. A program then verifies quotes against the text and unit-converts
(BP → calendar, kya → years, "8th century" → interval), recording the
conversion. Extraction runs with a structured-output schema, and the same
document is extracted twice by independent runs on the first ~100 documents to
measure agreement (§B6).

### B6. Synthesis and QA

A **synthesis agent** per task sees all claims for a languoid (bootstrap +
extracted), applies the rules in §A3 to produce the adopted record and
rationale, and lists open questions. Then:

- **Mechanical checks:** every adopted claim id exists and is `quote_verified`;
  intervals are ordered; a child node's crown age is not older than its parent's;
  an attested language's first attestation is not older than its parent node's
  crown age (violations are flagged, not auto-fixed — they are often the
  interesting cases).
- **Held-out agreement:** for families with D-PLACE trees, compare the harness's
  extracted `phylo_node_age` claims with the tree's node ages; for IE, compare
  extracted attestation claims with the IE-CoR / Chang calibration tables.
  Report precision/recall of extraction before trusting it on families with no
  ground truth.
- **Human review queue:** everything `confidence: low` or with a flagged check,
  ordered by wety relevance, in a single markdown file per run. Decisions go
  into `overrides/` with reasons, exactly as the crossref does.

### B7. Cost and scale

Roughly: 300–400 tasks; 20–60 candidate papers per task from search (≈15k
metadata records, free); 5–15 included per task (≈3–4k full texts, most OA);
extraction of 3–4k documents at ~10–30k tokens each is on the order of 10⁸
input tokens — modest. Search and screening are the bulk of API calls; both
are cheap. Budget the expensive model for extraction and synthesis only.

### B8. Reproducibility record (what makes a run a run)

Each run directory has: the git commit of the harness and rules; the crossref
build hashes; the bootstrap dataset versions; `search_log.jsonl`; the include /
exclude decisions; the retrieved-text hashes; model identifiers for every agent
call; and the resulting claims and adopted records. A second run on the same
inputs must reproduce the bootstrap claims and the mechanical checks exactly,
and the search/screen/extract layers up to logged LLM variance, which the diff
makes visible. Re-running only the synthesis layer (claims → adopted values) is
supported and cheap.

---

## C. What to run first

1. **B1 bootstrap**, end to end, with the schema and validators. Deliverable:
   `data/dating/` populated from Glottolog, Wikidata, Wikipedia infoboxes,
   IE-CoR, and D-PLACE, with a coverage report per family (how many Wiktionary
   codes have any date, by claim kind). Expect: nearly all historical IE
   languages and ~15 families' nodes covered before any literature search.
2. **Indo-European as the pilot** for B2–B6, because ground truth is richest
   (three independent Bayesian studies, dense attestations) and wety relevance
   is highest. Measure extraction agreement and mechanical-check violations;
   fix prompts and rules; only then scale.
3. **Next families by wety relevance**: Sino-Tibetan, Afro-Asiatic (Semitic and
   Egyptian first), Uralic, Turkic, Austronesian, Dravidian, Japonic, Koreanic,
   Mongolic, Tai-Kadai, Austroasiatic, Kartvelian; then the Americas and
   Australia, where the lexicostatistic floor will often be all there is.
4. **Wety integration** last: `_derived/wiktionary_dates.json` keyed by
   Wiktionary code, consumed by `core/src/languages.rs` next to
   `languages.json`; the crossref README already describes the pattern.

## D. Decisions to make before starting

- Adopt astronomical year numbering (recommended) or ISO 8601 with `-0500`
  strings; the validator enforces whichever.
- Whether periodization boundaries for the big literary traditions (Old/Middle/
  Modern stages of English, French, German, Chinese, Japanese, Greek, Arabic,
  Persian, Indic) are curated by hand from standard handbooks before the agent
  runs — recommended: they are few, high-value, and an agent will only find the
  same handbooks.
- Licensing of the output: CC-BY-4.0 is compatible with Glottolog and IE-CoR;
  Wikidata is CC0; the SIL ISO tables are freely redistributable with attribution.
- Whether unaccepted families' proto-languages in Wiktionary (Proto-Niger-Congo,
  Proto-Nilo-Saharan, Proto-Altaic, Proto-North-Caucasian …) get a record at all.
  Recommended: yes, `status: "contested"`, with the claims but no adopted value,
  so wety can show "hypothetical" rather than nothing.
