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
esx-esk-pro Proto-Eskimo (1084); ASJP only
ath-pro Proto-Athabaskan (726); ASJP only
ypk-pro Proto-Yupik (600)
esx-inu-pro Proto-Inuit (464); ASJP only
omq-mxt-pro Proto-Mixtec (260)
sai-car-pro Proto-Cariban (245); ASJP only
nai-mus-pro Proto-Muskogean (219); ASJP only
myn-pro Proto-Mayan (209); ASJP only
azc-nah-pro Proto-Nahuan (148)
iro-nor-pro Proto-North Iroquoian (119); ASJP only
sal-pro Proto-Salish (92); ASJP only
sai-tar-pro Proto-Taranoan (86)
sio-pro Proto-Siouan (85); ASJP only
tup-pro Proto-Tupian (79); ASJP only
sai-nje-pro Proto-Northern Jê (73)
iro-pro Proto-Iroquoian (65); ASJP only
omq-cha-pro Proto-Chatino (55); ASJP only
omq-zpc-pro Proto-Zapotec (51); ASJP only
aql-pro Proto-Algic (47); ASJP only
sai-cer-pro Proto-Cerrado (46)
auf-pro Proto-Arawa (45); ASJP only
omq-zap-pro Proto-Zapotecan (45); ASJP only
omq-pro Proto-Oto-Manguean (38); ASJP only
azc-num-pro Proto-Numic (35); ASJP only
sai-jee-pro Proto-Jê (33)
oto-otm-pro Proto-Otomi (32)
omq-otp-pro Proto-Oto-Pamean (26)
awd-nwk-pro Proto-Nawiki (21)
omq-tri-pro Proto-Triqui (19)
xnd-pro Proto-Na-Dene (18)
qwe-pro Proto-Quechuan (17)
oto-pro Proto-Otomian (14)
awd-taa-pro Proto-Ta-Arawak (11)
nai-mdu-pro Proto-Maidun (11)

Deliverables (write action): assertions.jsonl (id, code, source_id, quote, locator,
event, role, method, native_time in the registry's form: {{"form": "point" | "event_range" |
"duration" | "distribution" | "bound", "unit": ..., "value" or "lower"/"upper",
"median"/"mean" for posteriors, "interval_type", "probability", "present" if stated}},
subject {{code, source_label, mapping, mapping_basis, taxa}}, proposed_status, citation),
report.md (table: code, study, estimate, method, mapping; then codes with nothing found),
sources.json, self-check.json. Call check after saving, fix problems once, and finish.
An unstated epoch is not a problem to fix.
```
