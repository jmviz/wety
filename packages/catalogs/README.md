# Language catalog links, version 2

This is a registry of **assertions between source-specific concepts**, not a universal table of equivalent languages. Wiktionary language and family codes, ISO registrations, Glottolog languoids, Wikidata items and IE-CoR documentary samples describe different objects. Agreement between identifiers does not establish identical geographic, temporal, social or lexical scope.

Read [the linguistic and cataloging critique](critique.md), [the finding-by-finding resolution ledger](review-ledger.json), and [the generated coverage report](../../data/products/catalogs/report.md). The previous outputs, code and documentation are preserved under `data/archive/catalogs/pre_v2/` with hashes. They are not current results.

## Contract

Each Wiktionary main code retains a concept ID, source name, kind, alias codes, family, ancestors, parents and non-etymology-only association. Language and family namespaces are distinct: `wikt:lang:urj-pro` and `wikt:family:urj`. These identify catalog concepts in a fingerprinted snapshot, not timeless real-world entities. `index_by_wiktionary_code.json` resolves Wiktionary's own asserted aliases to main concepts; it does not resolve external identities.

A `links` entry has a content-derived ID, source and target, directional relation, assessment status, basis, source snapshot, scoped target metadata and `temporal_transfer_allowed: false`. Multiple targets, conflicting assertions and alternative names survive. Source attributes are not copied onto a different historical entity. Glottolog documentation dates remain in its own table; they do not become Wiktionary stage dates or language lifespans.

| Relation (source → target) | Meaning |
|---|---|
| `catalog_correspondence` | Useful catalog lookup; no assertion of equal historical extension |
| `equivalent` | Explicitly reviewed equivalence for the stated purpose and snapshot; no permission to transfer chronology |
| `source_narrower` / `source_broader` | Reviewed scope inclusion in the stated direction |
| `overlap` | Some shared scope, without containment or identity |
| `unresolved` | Insufficient or conflicting evidence about the relation |
| `family_context` | Family/proto association; not equality with a reconstructed population, stage or dated crown |
| `parent_context` | Wiktionary non-etymology-only association; not automatically genetic ancestry |
| `retired_successor` | Reserved for explicit administrative successor assertions; current retirement histories are retained in target metadata |

States are `catalog_link`, `proposed`, `context`, `reviewed`, and `rejected`. `catalog_link` is an unambiguous code-based lookup, **not adjudicated equivalence**. Names generate proposals. Neither a common ancestor nor a sibling relationship resolves conflicting scope. No relation is automatically composed transitively. The vocabulary is informed by concept mapping practice but deliberately does not claim OWL identity or implement SKOS entailments.

ISO parts have separate namespaces (`iso639-1`, `iso639-2b`, `iso639-2t`, `iso639-3`, `iso639-5`). Target metadata retains macro/member scope, active/retired status, registration types and retirement remedies, including split successors. These are administrative/categorical descriptions, not dates of language birth or extinction. Bibliographic and terminology spellings remain distinguishable.

Wikidata item links retain the asserted and resolved QIDs. The legacy truthy snapshots lack ranks, qualifiers, references and time precision: this is an explicit limitation, not silently reconstructed provenance. Route agreement is never assigned a confidence score or counted as independent corroboration. Optional full-statement retrieval preserves whole responses, revision IDs, redirects, ranks, qualifiers, references and snaks; it supplements observations and does not adjudicate them automatically.

Bookkeeping and non-genealogical grouping nodes remain identifiable as catalog objects. They cannot supply an automatic genetic ancestor. This does not imply sign languages, mixed languages or contact varieties lack histories; those histories require appropriate, explicitly reviewed relations.

## Build and validate

Use the repository root Python environment (`uv sync --frozen`; Python 3.12+). Commands below run from the repository root; existing pinned sources make them offline:

```sh
uv run --frozen python -m catalogs.build
uv run --frozen python -m catalogs.registry
uv run --frozen python -m unittest discover -s packages/catalogs/tests -v
uv run --frozen python -m catalogs.iecor
```

Glottolog is explicitly selected in `data/registry/inputs.json`, or pass `--glottolog-cldf PATH`. Directory order never selects a release. The release metadata and CLDF files are fingerprinted. A malformed classification or ambiguous ISO assignment fails validation rather than silently taking the last row. Wiktionary alias payloads must agree with their main entry.

For deliberate source refreshes:

```sh
uv run --frozen python -m source_tools.catalog_sources
uv run --frozen python -m source_tools.catalog_sources --skip-iso --skip-wikidata --statement-details
```

Refreshes archive the previous source set, stage downloads, serialize writers, and commit the source manifest last. Builds verify source hashes and targeted-Wikidata input hashes under the source lock. Changed inputs require rebuilding and reviewing stale decisions. Do not update a decision digest merely to bypass this gate: inspect the changed sources and record the renewed review.

`python -m source_tools.language_data DUMP` now defaults to repository-relative output paths and writes a receipt containing dump, Lua/exporter and output hashes plus dependency versions. The current legacy dump has no such receipt; its missing provenance remains disclosed. A future receipt must match both source outputs before the registry accepts it.

## Publication and consumers

The authoritative unit is `data/artifacts/catalog-generations/<digest>/bundle.json`, committed through `data/products/catalogs/CURRENT.json`. Use `registry.current_bundle()` for a consistent generation and `registry.validate_current()` to detect modified sources, decisions or mirrors. The bundle contains records, indexes, Glottolog source attributes, a prioritized review queue, decisions and input fingerprints. The workflow run receipt records the code that built a generation; code edits do not make it stale. Generations published before 2026-10-06 also carry an `implementation` record, which is no longer checked; later ones are identified by content alone, so a rebuild that changes no output keeps the generation. Mirrors are individually atomic, not a multi-file transaction; consumers must not assemble their own generation from independently opened mirrors.

JSON and CSV mirrors retain complete link assertions. CSV nested values are JSON, not semicolon-joined lists. `index_by_target.json` includes proposed, contextual and reviewed links from both language and family concepts, with relation/status/link IDs. Compatibility `index_by_glottocode.json` and `index_by_iso639_3.json` contain only projected language-code catalog lookups. Their multiplicity means shared catalog context, not synonymy. They are not safe equality joins.

Scalar `glottocode`, `iso639_3` and `iso639_5` fields are a narrow compatibility projection: one eligible catalog lookup, or an explicit scoped reviewed lookup. Conflicts and name-only proposals do not select a winner. Proto associations and inherited parent identifiers do not fill these fields. Read `links` for provenance and unresolved alternatives.

An editorial decision in `data/workspaces/catalogs/overrides.json` must name a qualified source and target, relation, reviewer, rationale, references, source snapshot and purpose. A subject holds one decision or a list with one per target. Arbitrary field patches, unknown targets, duplicate targets, stale decisions and undocumented equivalence fail. Observations remain visible after a decision. A reviewed `equivalent` or `catalog_correspondence` replaces the automatic lookup for its namespace, and so does a reviewed `unresolved`. A reviewed scope relation (`source_narrower`, `source_broader`, `overlap`) documents extension but leaves the catalog lookup in place. The Low German decision supports catalog lookup only; it is not a historical identity judgment. The 2026-10-03 dating pilot added scope decisions for 13 codes (`ang enm gmh goh sga grc got akk hit och ltc ojp la`), for the purpose of applying a source's period to a whole Wiktionary code. On 2026-10-04 `cu` was recorded as narrower than ISO `chu` and Glottolog `chur1257` (both cover the whole Church Slavonic tradition) and `sa` as broader than ISO `san` and Glottolog `sans1269` (Vedic and Classical Sanskrit; Wiktionary also files Buddhist Hybrid Sanskrit). `non` and ISO `lat` stay unresolved.

IE-CoR matching now produces **proposals**, preserving all 160 source sample IDs, names, codes, descriptions and prior parameters. No proposal selects a preferred Wiktionary stage. Its receipt binds the table, code, Wiktionary input, catalog generation and output. A unique Glottocode or similar name cannot turn Late Cornish into Old Cornish. Future sample-to-stage adjudication must record temporal/geographic/lexical scope; it must not overwrite the original sample or automatically transfer its prior.

Dating reads and validates the committed catalog bundle. Family nodes are not classified as reconstructed languages. Catalog ancestry remains context; unknown tips prevent a complete common-ancestor result, and non-genealogical buckets are excluded from genetic paths. Source-specific sample identities survive ingestion. Exact code-specific chronological evidence requires a separate dating assessment; every `spoken` field remains null.

## What remains unestablished

The software tests establish safety properties and reproduction, not mapping accuracy. No held-out specialist mapping evaluation has been performed. `review_queue.json` separates conflicts, stage scope and ordinary catalog-scope review; a queued record is not necessarily wrong. Most ordinary lookup links can remain useful without any claim of exact equivalence. Lack of a link is not evidence that a language or stage never existed.

The next work is bounded concept-scope review of the dating pilots and historically misleading matches, followed by an independently annotated evaluation set. See the critique's agenda. Do not improve apparent coverage by restoring inheritance, token matches or unconditional exact joins.
