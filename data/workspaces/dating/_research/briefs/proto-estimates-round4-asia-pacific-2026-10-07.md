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
phi-pro Proto-Philippine (1632); ASJP only
poz-mly-pro Proto-Malayic (1003)
mkh-pro Proto-Mon-Khmer (826); ASJP only
sit-aao-pro Proto-Central Naga (463)
sit-nas-pro Proto-Naish (378)
poz-mic-pro Proto-Micronesian (361)
kar-pro Proto-Karen (344); ASJP only
poz-cet-pro Proto-Central-Eastern Malayo-Polynesian (313); ASJP only
pqe-pro Proto-Eastern Malayo-Polynesian (277); ASJP only
mkh-pal-pro Proto-Palaungic (256)
poz-ssw-pro Proto-South Sulawesi (237); ASJP only
poz-pnp-pro Proto-Nuclear Polynesian (229)
mkh-ban-pro Proto-Bahnaric (226)
mkh-kat-pro Proto-Katuic (183)
tbq-lol-pro Proto-Loloish (151)
poz-pep-pro Proto-Eastern Polynesian (138); ASJP only
hmn-pro Proto-Hmongic (129); ASJP only
mkh-khm-pro Proto-Khmuic (113)
dru-pro Proto-Rukai (99)
map-ata-pro Proto-Atayalic (97); ASJP only
tbq-bdg-pro Proto-Bodo-Garo (95)
sit-tan-pro Proto-Tani (81)
qfa-kra-pro Proto-Kra (76)
poz-mcm-pro Proto-Malayo-Chamic (65); ASJP only
poz-msa-pro Proto-Malayo-Sumbawan (60); ASJP only
sit-tng-pro Proto-Tangkhulic (49)
btk-pro Proto-Batak (37)
paa-nha-pro Proto-North Halmahera (33); ASJP only
poz-swa-pro Proto-North Sarawak (33)
sit-kon-pro Proto-Northern Naga (33)
ngf-pro Proto-Trans-New Guinea (28)
aus-cww-pro Proto-Central New South Wales (26)
qfa-kms-pro Proto-Kam-Sui (26)
qfa-onb-pro Proto-Be (26)
hmx-mie-pro Proto-Mienic (25)
qfa-adm-pro Proto-Great Andamanese (22)
mkh-mnc-pro Proto-Monic (21)
sit-khp-pro Proto-Puroik (21)
poz-btk-pro Proto-Bungku-Tolaki (20)
aus-nyu-pro Proto-Nyulnyulan (18)
qfa-kor-pro Proto-Koreanic (18)
aav-pkl-pro Proto-Pnar-Khasi-Lyngngam (15)
aus-wdj-pro Proto-Iwaidjan (14)
dmn-mdw-pro Proto-Western Mande (12)
nub-pro Proto-Nubian (12)
omv-aro-pro Proto-Aroid (10)

Deliverables (write action): assertions.jsonl (id, code, source_id, quote, locator,
event, role, method, native_time in the registry's form: {{"form": "point" | "event_range" |
"duration" | "distribution" | "bound", "unit": ..., "value" or "lower"/"upper",
"median"/"mean" for posteriors, "interval_type", "probability", "present" if stated}},
subject {{code, source_label, mapping, mapping_basis, taxa}}, proposed_status, citation),
report.md (table: code, study, estimate, method, mapping; then codes with nothing found),
sources.json, self-check.json. Call check after saving, fix problems once, and finish.
An unstated epoch is not a problem to fix.
```
