# Brief: published proto-language estimates, round 4 ({{FAMILY}})

All kinds of published estimate for proto codes still undated or dated only by ASJP (maintainer decisions 2026-10-06: published estimates at scale; 'years ago' readable).

```text
Collect every published estimate of when each proto-language below was spoken or broke
up ({{FAMILY}}). Work as one researcher, without subagents or other model calls. Do not
redesign tools.

Kinds of estimate, each its own claim with its method: a Bayesian or other dated
phylogeny's node age for the named family or subgroup (method bayesian_phylogenetic; list
the node's sampled descendants in subject.taxa); lexicostatistical or glottochronological
dates (lexicostatistic); a specialist's dating from reconstructed vocabulary, loans or
sound changes (comparative_reconstruction); archaeological correlations
(archaeological_correlation); other expert statements (expert_estimate). Event
crown_divergence for a breakup or family age, conventional_period for when the stage was
spoken. Record ages stated as "years ago" or "BP" as printed (unit years_ago or BP),
with present only if the source states it. Map exact only if the dated group is the
family the code reconstructs (its first split sampled); otherwise subset, superset or
overlap with the reason. Keep calibration priors apart from posteriors. Never quote from
memory; quote exactly from retained text with a locator.

Already imported, do not collect again: Holman et al. 2011 and Rama & Wichmann 2020 (ASJP); King et al. 2024; Gavashelishvili et al. 2023; Dunn et al. 2013; Kallio 2006 Table 1; Janhunen 1998 via Häkkinen 2009; Häkkinen 2009 on Mongolic; the GLD Nakh tree; Oskolskaya et al. 2021 and Robbeets & Oskolskaya 2022's table; Nagaraja et al. 2013; Wu et al. 2022; Peiros 1998 (Hmong-Mien, Austroasiatic); G. Starostin 2010 and 2014 (Yeniseian, Ket-Yugh, generic bounds); Vajda 2018; Ratliff 2010; Militarev 2022; Blust 2013's general 5,000–6,000 years; Goddard 1978 via Denny 1991; EHL 2014; Lakarra 2009; Kulikov 2017; Tao et al. 2023; Zhang et al. 2019; the deposited trees (Heggarty, Kolipakam, Savelyev & Robbeets, Bouckaert, Kitchen, Grollemund, Koile, Lee & Hasegawa, Greenhill 2023, Gerardi 2023). Also imported in round 4: Rau & Sidwell 2019; Grant 2005; Bellwood 2019; Sidwell's Pearic page; Sidwell 2022; Pawley 2005; Militarev 2000, 2002, 2005; Parpola 2017; Rahkonen 2014; Holopainen & Metsäranta 2020; Kaufman & Justeson 2004; Rankin 1996; Trask 2008; Antczak et al. 2017.

Codes (Wiktionary proto code, name, number of wety tree items; "ASJP only" marks codes
whose only date is an ASJP point):
qfa-yuk-pro Proto-Yukaghir (2745); ASJP only
grk-pro Proto-Hellenic (1725)
urj-mdv-pro Proto-Mordvinic (710); ASJP only
sel-pro Proto-Selkup (567)
euq-pro Proto-Basque (368)
iir-nur-pro Proto-Nuristani (315)
alv-yrd-pro Proto-Yoruboid (288)
mns-pro Proto-Mansi (285)
bnt-sab-pro Proto-Sabaki (238)
kca-pro Proto-Khanty (224)
dra-sou-pro Proto-South Dravidian (211)
urj-ugr-pro Proto-Ugric (194)
xsc-pro Proto-Scythian (188)
alv-gbe-pro Proto-Gbe (180)
xsc-sak-pro Proto-Saka (155)
xsc-sar-pro Proto-Sarmatian (150)
xsc-skw-pro Proto-Saka-Wakhi (137)
cau-abz-pro Proto-Abkhaz-Abaza (128)
bnt-ngu-pro Proto-Nguni (125)
cau-cir-pro Proto-Circassian (121)
alv-yor-pro Proto-Yoruba (111)
trk-ogz-pro Proto-Oghuz (111)
pra-pro Proto-New Indo-Aryan (107)
cau-lzg-pro Proto-Lezghian (102)
cdc-pro Proto-Chadic (98); ASJP only
alv-edk-pro Proto-Edekiri (92)
qfa-cka-pro Proto-Chukotko-Kamchatkan (69); ASJP only
cau-nwc-pro Proto-Northwest Caucasian (67); ASJP only
cus-sou-pro Proto-South Cushitic (61); ASJP only
hyx-pro Proto-Armenian (55)
alv-pro Proto-Atlantic-Congo (53)
cau-ava-pro Proto-Avaro-Andian (52)
alv-igb-pro Proto-Igboid (48)
ira-shy-pro Proto-Shughni-Yazghulami (45)
cus-som-pro Proto-Somaloid (42)
ira-shr-pro Proto-Shughni-Roshani (42)
cau-nec-pro Proto-Northeast Caucasian (38); ASJP only
ira-sym-pro Proto-Shughni-Yazghulami-Munji (38)
alv-edo-pro Proto-Edoid (37)
cau-tsz-pro Proto-Tsezian (35)
cdc-cbm-pro Proto-Central Chadic (35); ASJP only
inc-ash-pro Proto-Middle Indo-Aryan (33)
khi-kho-pro Proto-Khoe (32)
ira-sgc-pro Proto-Sogdic (31)
ira-pat-pro Proto-Pathan (27)
nic-pro Proto-Niger-Congo (24)
trk-bul-pro Proto-Bulgar (23)
alv-von-pro Proto-Volta-Niger (22)
ira-mpr-pro Proto-Medo-Parthian (22)
csu-sar-pro Proto-Sara (18)
qfa-yko-pro Proto-Kottic (18)
ira-mny-pro Proto-Munji-Yidgha (15)
csu-pro Proto-Central Sudanic (14)
nic-bco-pro Proto-Benue-Congo (14)
csu-bba-pro Proto-Bongo-Bagirmi (13)
cus-hec-pro Proto-Highland East Cushitic (12)
cau-and-pro Proto-Andian (10)

Deliverables (write action): assertions.jsonl (id, code, source_id, quote, locator,
event, role, method, native_time in the registry's form: {{"form": "point" | "event_range" |
"duration" | "distribution" | "bound", "unit": ..., "value" or "lower"/"upper",
"median"/"mean" for posteriors, "interval_type", "probability", "present" if stated}},
subject {{code, source_label, mapping, mapping_basis, taxa}}, proposed_status, citation),
report.md (table: code, study, estimate, method, mapping; then codes with nothing found),
sources.json, self-check.json. Call check after saving, fix problems once, and finish.
An unstated epoch is not a problem to fix.
```
