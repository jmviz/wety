# Brief: published proto-language estimates, round 5 ({{FAMILY}})

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

Already imported, do not collect again: Holman et al. 2011 and Rama & Wichmann 2020 (ASJP); King et al. 2024; Gavashelishvili et al. 2023; Dunn et al. 2013; Kallio 2006 Table 1; Janhunen 1998 via Häkkinen 2009; Häkkinen 2009 on Mongolic; the GLD Nakh tree; Oskolskaya et al. 2021 and Robbeets & Oskolskaya 2022's table; Nagaraja et al. 2013; Wu et al. 2022; Peiros 1998 (Hmong-Mien, Austroasiatic); G. Starostin 2010 and 2014 (Yeniseian, Ket-Yugh, generic bounds); Vajda 2018; Ratliff 2010; Militarev 2022; Blust 2013's general 5,000–6,000 years; Goddard 1978 via Denny 1991; EHL 2014; Lakarra 2009; Kulikov 2017; Tao et al. 2023; Zhang et al. 2019; the deposited trees (Heggarty, Kolipakam, Savelyev & Robbeets, Bouckaert, Kitchen, Grollemund, Koile, Lee & Hasegawa, Greenhill 2023, Gerardi 2023). Also imported in round 4: Rau & Sidwell 2019; Grant 2005; Bellwood 2019; Sidwell's Pearic page; Sidwell 2022; Pawley 2005; Militarev 2000, 2002, 2005; Parpola 2017; Rahkonen 2014; Holopainen & Metsäranta 2020; Kaufman & Justeson 2004; Rankin 1996; Trask 2008; Antczak et al. 2017. Also imported in round 5: G. Starostin 2003 and 2020; Häkkinen 2012; Bartens 1999; Blažek 2012 and 2024; Barreteau & Jungraithmayr 1993; Walworth 2012; Vydrin 2009; Krauss 1973 and Kari 2010; Kaufman & Golla 2000; Lounsbury 1961; Julian 2010; Davletshin 2012; Hopkins; Broadwell 1992; Rodrigues 1964/2020; Migliazza 1982; Durbin 1977; Tarble 1985; Jolkesky 2016; Proulx 1980/1984; Gerardi et al. 2025; Martínez-Areta 2013; Blažek 2019 (South Cushitic); Forker 2020 (Dargwa); Nichols 2003 (Northeast Caucasian). Look further: specialist grammars' and handbooks' introductions (Campbell's American Indian Languages, Bellwood's First Farmers, Blench, Ehret, Heine & Nurse, Sidwell & Jenny, Kaufman), JSEALS and Diachronica papers, regional archaeology syntheses.

Codes (Wiktionary proto code, name, number of wety tree items; "ASJP only" marks codes
whose only date is an ASJP point):
qfa-yuk-pro Proto-Yukaghir (2745); ASJP only
phi-pro Proto-Philippine (1632); ASJP only
poz-mly-pro Proto-Malayic (1003)
ypk-pro Proto-Yupik (600)
esx-inu-pro Proto-Inuit (464); ASJP only
sit-nas-pro Proto-Naish (378)
kar-pro Proto-Karen (344); ASJP only
poz-cet-pro Proto-Central-Eastern Malayo-Polynesian (313); ASJP only
mns-pro Proto-Mansi (285)
omq-mxt-pro Proto-Mixtec (260)
bnt-sab-pro Proto-Sabaki (238)
poz-pnp-pro Proto-Nuclear Polynesian (229)
kca-pro Proto-Khanty (224)
myn-pro Proto-Mayan (209); ASJP only
mkh-kat-pro Proto-Katuic (183)
xsc-sak-pro Proto-Saka (155)
xsc-sar-pro Proto-Sarmatian (150)
hmn-pro Proto-Hmongic (129); ASJP only
bnt-ngu-pro Proto-Nguni (125)
mkh-khm-pro Proto-Khmuic (113)
trk-ogz-pro Proto-Oghuz (111)
cau-lzg-pro Proto-Lezghian (102)
map-ata-pro Proto-Atayalic (97); ASJP only
alv-edk-pro Proto-Edekiri (92)
sai-tar-pro Proto-Taranoan (86)
sit-tan-pro Proto-Tani (81)
sai-nje-pro Proto-Northern Jê (73)
cau-nwc-pro Proto-Northwest Caucasian (67); ASJP only
poz-msa-pro Proto-Malayo-Sumbawan (60); ASJP only
omq-cha-pro Proto-Chatino (55); ASJP only
cau-ava-pro Proto-Avaro-Andian (52)
sit-tng-pro Proto-Tangkhulic (49)
aql-pro Proto-Algic (47); ASJP only

Deliverables (write action): assertions.jsonl (id, code, source_id, quote, locator,
event, role, method, native_time in the registry's form: {{"form": "point" | "event_range" |
"duration" | "distribution" | "bound", "unit": ..., "value" or "lower"/"upper",
"median"/"mean" for posteriors, "interval_type", "probability", "present" if stated}},
subject {{code, source_label, mapping, mapping_basis, taxa}}, proposed_status, citation),
report.md (table: code, study, estimate, method, mapping; then codes with nothing found),
sources.json, self-check.json. Call check after saving, fix problems once, and finish.
An unstated epoch is not a problem to fix.
```
