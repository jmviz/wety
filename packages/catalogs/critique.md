> Historical audit: the original locations are retained in the reorganization relocation map. Current code is in `packages/catalogs`; current products are in `data/products/catalogs`.

# Language-code cross-reference: linguistic and cataloging review

Reviewed 19 September 2026 against the local Wiktionary export, ISO tables, Glottolog 5.3 CLDF, Wikidata snapshots, cross-reference builder, IE-CoR matcher and dating consumers. This is a code-and-source audit, not a claim to have conducted specialist field review of every language. Evidence below refers to the **archived pre-v2 snapshot** unless explicitly described as a repair. The machine-readable [resolution ledger](review-ledger.json) tracks every finding and its checks.

The old output was a useful discovery aid presented with stronger semantics than its evidence supported. It collapsed administrative registrations, dictionary editorial units, catalog languoids, reconstructed stages and sampled corpora into preferred identifiers. Its confidence labels could not measure the accuracy of that collapse. The most serious failures were not obscure exceptions: they followed directly from the matching rules. Connecting this table to chronology made those errors consequential.

The archived baseline has 8,625 Wiktionary language/stage codes and 887 family codes, against 27,177 Glottolog nodes. It assigns 7,105 language-code records “high” confidence, supplies 8,070 Glottocodes including 201 inherited mappings, and retains preferred identifiers on 24 records with explicit conflicts. Those are output counts, not precision estimates. The original IE-CoR matcher produces 158 selections from 160 samples: 111 by names, 32 by Glottocode, 13 by ISO and two by an ancestor fallback. High apparent coverage conceals the absence of a defensible identity criterion.

## C01 — An identifier is not a definition of a language

A Wiktionary code can denote a dictionary section, reconstructed stage, regional variety, register or editorial umbrella. An ISO registration and a Glottolog languoid have different inclusion and scope policies; a Wikidata item can describe still another concept. Neither equal names nor a shared third identifier makes their extensions identical. The original flat record offers no place to say which of these things it identifies, at which release, and for which use.

**Repair:** namespaced language/family concepts, fingerprinted snapshots, directional relation assertions and scoped target metadata. Wiktionary's asserted aliases have a separate validated lookup. `catalog_correspondence` supports lookup without asserting exact equivalence. No transitive identity inference is permitted. This follows the useful distinction between concepts, notations and mappings in [SKOS](https://www.w3.org/TR/skos-reference/#mapping), without importing SKOS or OWL identity semantics wholesale.

## C02 — The confidence scale was uncalibrated

Agreement between ISO-derived and Wikidata-derived routes is not necessarily independent evidence: Wikidata may copy ISO or Glottolog, and catalog editors may share sources. A “high” label has no empirical meaning without an evaluation target, reference judgments and error rates. A lower confidence label on a forced wrong stage would not fix the stage error.

**Repair:** remove public high/medium/low confidence and route-count ranking. Preserve routes and dependency limitations, distinguish proposed/contextual/reviewed links, and report coverage separately from unmeasured accuracy. A reviewed assertion identifies who reviewed what purpose, not an invented probability.

## C03 — Conflicts survived as authoritative-looking answers

The old table could say “conflict” and still export one Glottocode. Downstream consumers read the code, not the warning. Taking the first Wikidata value was also ordering-dependent evidence selection. Aramaic (`arc`) demonstrates the problem: the ISO route points to Imperial Aramaic (`impe1235`), while Wikidata includes the broader Aramaic family (`aram1259`). Neither a single preferred scalar nor a blended interval resolves that scope disagreement.

**Repair:** retain both proposals; withhold the scalar on ambiguity. A scoped review can select a lookup while preserving the contradictory observations. All alternatives remain in the typed reverse index. Aramaic is a regression case.

## C04 — Relatedness is not equal granularity

The original builder treated ancestor, descendant or sibling differences as less serious than unrelated differences. Linguistically, a related mismatch can be precisely the error that matters. A pan-dialectal dictionary, one regional lect and a historical stage may share a lineage while covering different evidence. Arabic (`ar`) links to a family node, as does Chinese (`zh`); that does not turn either dictionary unit into a sampled family crown. Conversely, `sh` maps to `sout1528`, a **language-level** Serbian-Croatian-Bosnian catalog entry. The old documentation incorrectly grouped it with family-level examples.

**Repair:** retain catalog level/category; ordinary language-to-family associations are context rather than automatic scalar matches. Do not suppress conflict merely because targets are related. More specific inclusion/overlap relations require a stated scope review.

## C05 — Parent inheritance fabricated stage metadata

`nonEtymologyOnly` is an editorial association. It need not mean immediate genetic parent, temporal predecessor, or an entity with interchangeable identifiers. Copying its ISO codes, types and Glottolog documentation range into an etymology-only child creates apparently direct information where there was none. Multiple historical stages may then appear identical because the system manufactured that identity.

**Repair:** remove attribute inheritance. Keep an explicit `parent_context` assertion and the original ancestor/parent arrays as Wiktionary claims. No child's dates or identifiers are filled through that relation. Missing direct scope evidence remains missing.

## C06 — Proto-language, family and crown were conflated

The old README explicitly proposed treating the family node's root age as the proto-language date. This is the same conceptual mistake identified in the dating critique. A reconstructed stage, a catalog family and a model's MRCA of sampled descendants are three entities. Their relation requires linguistic and sampling arguments. P279 (“subclass of”) is not “reconstructed ancestor of.” A family naming a proto-language is an association, not proof of its temporal extension.

The Uralic examples make this concrete. `urj-ugr-pro` had a Uralic family association; `urj-fpr-pro` could inherit the broader proto-parent's metadata. This review does not claim all root-record aliases were exported as dated stages.

**Repair:** preserve explicit proto/family associations as context only, including multiple families naming the same proto code. Remove generic family-membership fallback and P279-derived family identity. Dating no longer treats family-level catalog nodes as reconstructed-language entities. No crown becomes a proto period.

## C07 — ISO is an administrative system, not a historical ontology

ISO 639-1, 639-2 bibliographic/terminology, 639-3 and 639-5 are distinct namespaces, even where strings coincide. Macrolanguage membership is not necessarily phylogenetic descent. Retirement can merge, split, rename or remove an administrative registration; its effective date is not extinction. A replacement code cannot automatically inherit the old concept's scope. Keeping only `Change_To` misses split remedies expressed in retirement notes.

**Repair:** explicit namespace links; retain scope, type, macro/member relations and complete retirement metadata, including listed split successors. Do not infer ISO-1 membership from two-letter shape. Remove Glottocode lookup through retirement successors. The German bibliographic/terminology pair and a split-retirement control are tested separately.

## C08 — Catalog organization was treated as genealogy

Glottolog explicitly distinguishes genealogical classifications from non-genealogical trees and bookkeeping languoids. Its own classifications are revisable scholarly judgments. A lowest common catalog node inside a bookkeeping or administrative grouping is not a historical ancestor. Silently dropping unknown tips can also return an ancestor for a different sample from the one requested. These points follow directly from [Glottolog's description of its catalog](https://glottolog.org/glottolog/glottologinformation).

**Repair:** preserve categories and exclude these groupings from automatic genetic paths/MRCA. Require every requested tip. Keep family status unknown rather than reconstructed. Exclusion of the broad sign-language grouping is not a claim that sign-language families lack genealogy; such cases need their own reviewed representation. Catalog placement of an old stage beneath a modern language likewise cannot establish temporal descent direction.

## C09 — Names were being asked to do identity's work

Aliases can be cover terms, exonyms, outdated classifications and regional labels. Stripping accents, erasing non-Latin characters, truncating tokens to five letters or preferring the sole “regular” code cannot establish linguistic identity. A family guard merely makes a false match look plausible. The IE-CoR error “Late Cornish” → “Old Cornish” is not an obscure linguistic debate; it is a failure to preserve an explicit stage distinction.

**Repair:** names generate proposals only; remove suffix-parent inference, token selection, regular-code preference and ancestor fallback. IE-CoR name normalization preserves Unicode distinctions. Original names and evidence remain inspectable. No nearest candidate wins simply because the correct stage is absent.

## C10 — Classification guards and mutation order hid disagreement

Using one catalog's family tree to filter another catalog's candidate set assumes the very classification agreement being evaluated. Order-dependent deletion of family mappings further made downstream matches depend on earlier mutations. Proto backfilling and late overrides compounded this effect.

**Repair:** source classifications remain separate; name candidates are not deleted by family guards. Incompatible superfamily assertions become review issues. Proto associations do not manufacture family matches. Apply bounded decisions before building projections and indexes; sort outputs deterministically.

## C11 — Overrides bypassed the epistemic model

Arbitrary field patches could overwrite identifiers or confidence without a bounded relation, reviewer, source revision or reproducible justification. An override applied after dependent inference left downstream data based on the superseded match. The Low German note also incorrectly called `nds-de` and `nds-nl` etymology-only codes; both are regular entries in this snapshot.

**Repair:** exact decision fields, qualified subjects/targets, purpose, reviewer, rationale, references and source fingerprint; reject unknown fields, targets and stale decisions. The renewed Low German decision permits a broader catalog lookup at `lowg1239`, explicitly not exact historical identity. Observed alternatives remain visible. This is a narrow documented review, not specialist adjudication of Low German periodization.

## C12 — Reproducibility stopped short of the actual inputs

A downloaded filename is not a source version. Reading whichever release sorts last, failing to check declared hashes, or using a Wikidata extract queried against a different Wiktionary export can silently change the mapping universe. Original dump revisions cannot be recovered by assigning a new download date to old bytes.

**Repair:** verify downloaded-source hashes, record the source manifest and all relevant Glottolog/Wiktionary hashes, require explicit selection among multiple Glottolog releases, and bind targeted Wikidata to its input exports. Preserve release metadata and old source sets. Report missing legacy upstream revisions honestly. Existing decisions fail closed when their source snapshot changes.

## C13 — Truthy Wikidata values lost the statements that justified them

A truthy SPARQL value does not retain the complete statement's rank, qualifiers, references, precision, no-value/unknown-value state or revision history. A preferred rank is not independent verification; deprecated statements may matter to a history of a mapping. Redirects can be chained or missing, and should not overwrite the originally asserted QID. [Wikidata's rank documentation](https://www.wikidata.org/wiki/Help:Ranking) explains the rank distinction; the audit's conclusion is that the original extract lacks enough information to judge statement provenance.

**Repair:** disclose that loss at every legacy item link; preserve asserted/resolved QIDs and detect redirect cycles. Add optional full entity-response retrieval with statement evidence intact and fixture-based verification. The current old snapshot remains incomplete; it is **not retroactively promoted** by the new fetch capability. Fetching fresh statements would create a new source version requiring comparison and review, not recover the old revision automatically.

## C14 — Flat exports made careful internal notes disappear

A reverse index listing bare Wiktionary codes does not say whether they are aliases, broader concepts, hypotheses or conflicts. Semicolon-joined CSV values lose structure, and family codes can collide with language codes if namespaces are omitted. Publishing warnings only in prose invites downstream equality joins.

**Repair:** full link assertions survive CSV as JSON; the typed target index includes language and family sources, link IDs, relations and states. Compatibility indexes have a documented limited lookup meaning. Dates remain attributes of their source catalog entries, not flattened language facts. A CSV round-trip test verifies unresolved Aramaic assertions survive export.

## C15 — Independent file writes were not a reproducible release

Readers could observe new identifiers with old reverse indexes. A timestamp-dependent report also contradicts a claim of byte-for-byte deterministic output. Replacing downloads before a complete fetch succeeds can destroy the last coherent input set.

**Repair:** content-addressed bundles, writer locks, staged source retrieval, immutable generation directories and an atomic commit pointer. Consumers read one bundle. Mirrors are individually atomic and verified, but not advertised as a multi-file transaction. Actual successive builds are compared by generation digest. Failure retains the previous committed catalog generation; changed source sets invalidate validation until rebuilt.

## C16 — A sampled variety is not whichever dictionary stage matches best

IE-CoR's identifiers often supply coarse catalog context for narrower sampled material. Old Swedish uses the Swedish source code; Old Novgorod can point at Old East Slavic; Early Modern Slovene is not simply modern Slovene. The old matcher also chose Old Cornish for Late Cornish. A unique code, shared lexical token or catalog ancestor cannot reconstruct what material the authors sampled. Selecting a different catalog code is not a correction to the authors' sample identity.

**Repair:** all 160 samples retain `iecor:sample:<ID>` plus original labels, codes, descriptions and prior parameters. Suggestions stay unresolved proposals; no selected Wiktionary identity is exported. Seven samples currently have no candidate under these restricted routes, rather than being forced to an ancestor. The other 153 have proposals, not 153 verified matches. Dating uses the literal source catalog code for context and the sample ID/name for the subject. The matcher receipt prevents stale combinations. Reviewing sample-to-stage relations remains substantive work.

## C17 — Downstream dating undid the methodological distinction

The dating export's null lifespan policy was necessary but insufficient. Its loader inferred statuses from codes on linked concepts, equated family nodes with reconstructed languages, and its IE-CoR importer preferred the matcher's substituted node. Wikidata and Wikipedia bootstrap paths also used first-winner joins and proto/family shortcuts. A safe top-level display does not excuse a corrupted subject underneath it.

**Repair:** validate and load one catalog generation; never transfer ISO/Wikidata type labels to another entity's status. Keep source-specific IE-CoR, Wikidata and article subjects; preserve ambiguous context instead of selecting by iteration order. Remove proto/family shortcuts. Include catalog and IE-CoR generation/input fingerprints in dating's evidence context. Catalog relations, their unresolved scope and prohibition on date transfer are visible in dating exports. No language-level mapping dates an etymological edge.

## C18 — The Wiktionary extractor lacked an audit receipt

The exporter required a dump but did not record which one, its hash, extraction code or library versions. Its default output paths were also inconsistent with the documented repository-root invocation. A reproducible cross-reference cannot repair unrecorded upstream history merely by hashing the final JSON.

**Repair:** repository-relative defaults, complete both extractions before replacing output, atomic JSON writes, and a final receipt with dump, Lua, exporter, dependency and output fingerprints. The registry checks a receipt when supplied. The old dump receipt remains unavailable and is explicitly disclosed. Full revision-level reconstruction and executing a fresh dump extraction require the actual dump; neither is claimed completed here.

## C19 — Coverage was standing in for validation

The old report measured how many links existed, not whether the intended linguistic relation was correct. A benchmark consisting only of straightforward modern ISO matches would miss historical stages, dialect continua, macros, family/proto associations, contact varieties and retired-code splits. Mechanical regression tests alone cannot establish worldwide linguistic accuracy either.

**Repair:** offline adversarial mapping/provenance tests and dating integration tests, plus real snapshot regressions for Aramaic, Low German, Uralic proto associations, umbrella codes and misleading IE-CoR stages. Reports explicitly leave held-out mapping accuracy unmeasured. Add an independent reference set before making accuracy or confidence claims; stratify it by entity type and relation, not just match route.

## C20 — No accountable route from findings to decisions

A critique without a ledger can disappear into revised prose; a mapping system without a review queue can quietly reproduce the same mistakes. Equally, changing every label to “unknown” and declaring the linguistic research finished would be misleading.

**Repair:** this numbered audit, a machine-readable resolution ledger with test references, a prioritized per-concept mapping review queue, and a bounded decision format. Implementation closure means the unsafe behavior is removed, tested, or explicitly withheld. It does **not** mean thousands of concepts have received expert equivalence judgments. The updated registry currently makes no reviewed equivalence claims.

## Agenda after the repair

1. **Adjudicate a small, consequential mapping packet.** Start with the 160 IE-CoR samples, especially Late/Middle/Old Cornish, Old Swedish, Old Novgorod and historical Slovene; then pilot proto-family associations and the 24 archived conflicted records. For each, record source definitions, sampled material, geography, period, inclusion direction, reviewer and purpose. An absent stage may require a new source-specific entity, not a forced catalog code. Completion means every selected case has a reviewed relation or a documented abstention, with alternatives retained.
2. **Recover provenance deliberately.** Obtain a reproducible Wiktionary dump receipt and statement-level Wikidata snapshots; preserve the legacy version and compare revisions. Verify the IE-CoR table against its publication/deposit. Completion means the actual source versions can be retrieved or access gaps are explicit. Do not equate a fresh Wikidata query with historical provenance recovery.
3. **Create an independently reviewed evaluation set.** Include modern languages, dialect/register scope, ancient stages, reconstructed stages, macro/member codes, contested classifications, aliases, Unicode names, retired splits and negative matches. Separate development cases from held-out cases. Report relation-specific precision/recall, false identity rate and abstention, including disagreement between annotators.
4. **Review the three dating pilot entity dossiers before adopting dates.** Define each paper's sampled clade and documentary tips independently of catalog codes; establish whether a candidate stage is narrower, broader, overlapping or unresolved. Keep source priors, posterior estimates and reconstructed periods separate. This is an explicit prerequisite in the [dating agenda](../dating/docs/next-steps.md).
5. **Exercise downstream use with relative etymological constraints.** Review a few borrowings and inherited forms against donor innovations, sound changes, locality and attestations. Record event/lexeme identity explicitly. A catalog lookup must not turn into an assertion that a word existed at a family divergence.
6. **Expand only with versioned comparisons and measured error.** Review additions, retirements, scope changes, splits and merged concepts as release events. Keep old concept snapshots addressable. Extend coverage where the evaluation supports it; do not reward filled scalar fields as the primary objective.

Verification: [the recorded final checks](verification.json) report 36 catalog tests and 78 dating tests passing, both registries validating without errors, identical successive catalog generations, and all 14 archived files matching their recorded hashes. These results establish the listed software properties, not linguistic ground truth.
