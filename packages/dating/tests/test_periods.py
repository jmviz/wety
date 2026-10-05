import unittest

from harness.evidence import normalize_time
from harness.periods import export_periods, label

SOURCES = {
    "src_bc": {"id": "src_bc", "authors": ["Baugh, Albert C.", "Cable, Thomas"], "year": 2013},
    "src_iso": {
        "id": "src_iso",
        "title": "ISO 639-3 code tables",
        "url": "https://iso639-3.sil.org",
    },
    "src_many": {"id": "src_many", "authors": ["A, a", "B, b", "C, c"], "year": 2001},
}


def claim(cid, code, event, role, raw, source="src_bc", status="supported", mapping="exact"):
    return {
        "id": cid,
        "status": status,
        "subject": {"id": "wikt:" + code, "name": code, "mapping": mapping},
        "event": event,
        "role": role,
        "time": normalize_time(raw).model_dump(),
        "source_id": source,
        "spans": [{"quote": "q " + cid, "locator": "p. 1"}],
        "analysis_id": None,
    }


def bundle(claims, adopted, kinds=None):
    kinds = kinds or {}
    codes = {c["subject"]["id"][5:] for c in claims}
    view = {
        code: {
            "name": code.upper(),
            "kind": kinds.get(code, "regular"),
            "evidence_ids": [c["id"] for c in claims if c["subject"]["id"] == "wikt:" + code],
        }
        for code in codes
    }
    by_id = {c["id"]: c for c in claims}
    decisions = [
        {"subject_id": by_id[i]["subject"]["id"], "event": by_id[i]["event"], "claim_id": i}
        for i in adopted
    ]
    return {"claims": claims, "sources": SOURCES, "view": view, "decisions": decisions}


def years(lo, hi, unit="CE", **extra):
    return {
        "form": "duration",
        "unit": unit,
        "lower": lo,
        "upper": hi,
        "precision": "year",
        **extra,
    }


class LabelTests(unittest.TestCase):
    def test_millennia_and_single_units(self):
        third = (-2999, -2000)
        self.assertEqual(label(third, third, approximate=True), "c. 3rd millennium BCE")
        self.assertEqual(label((-2999, -2000), (-1999, -1000)), "3rd–2nd millennium BCE")
        self.assertEqual(label((-2999, -2000), (-1200, -1200)), "3rd millennium – 1201 BCE")
        self.assertEqual(label((401, 500), (1101, 1200)), "5th–12th century CE")


class PeriodExportTests(unittest.TestCase):
    def test_conventional_period_with_alternative_kept_separate(self):
        claims = [
            claim(
                "a", "ang", "conventional_period", "convention", years(450, 1150, approximate=True)
            ),
            claim("b", "ang", "conventional_period", "convention", years(450, 1100), "src_iso"),
        ]
        out = export_periods(bundle(claims, ["a"]), "gen")
        (ang,) = out["languages"]
        self.assertEqual(ang["kind"], "stage")
        self.assertEqual(ang["basis"], "conventional")
        self.assertEqual(
            ang["spoken"],
            {
                "start": {"earliest": 450, "latest": 450},
                "end": {"earliest": 1150, "latest": 1150},
                "ongoing": False,
            },
        )
        self.assertEqual(ang["label"], "c. 450–1150 CE")
        self.assertEqual(ang["evidence"][0]["source"], "Baugh & Cable 2013")
        self.assertEqual(ang["evidence"][0]["quote"], "q a")
        # The other source stays a separate alternative; nothing is averaged.
        (alt,) = ang["alternatives"]
        self.assertEqual((alt["claim_id"], alt["label"]), ("b", "450–1100 CE"))
        self.assertEqual(alt["source"], "ISO 639-3 code tables")
        self.assertEqual(out["generation"], "gen")

    def test_varieties_give_a_derived_span(self):
        cp = ("conventional_period", "convention")
        claims = [
            claim("oj", "ojp", *cp, years(700, 800)),
            claim("emj", "ja-mid-ear", *cp, years(800, 1200)),
            claim("lmj", "ja-mid-lat", *cp, years(1200, 1600)),
            claim("ved", "vsn", *cp, years(500, 1500, "BCE", approximate=True)),
            claim(
                "cls",
                "cls",
                *cp,
                {"form": "duration", "start": {"value": 400, "unit": "BCE"}, "ongoing": True},
            ),
        ]
        b = bundle(claims, ["oj", "emj", "lmj", "ved", "cls"])
        parent = {
            "ja-mid-ear": "ojp",
            "ja-mid-lat": "ojp",
            "ojp-eas": "ojp",
            "vsn": "sa",
            "cls": "sa",
        }
        for code in ("ojp-eas", "sa"):
            b["view"][code] = {"name": code.upper(), "kind": "regular", "evidence_ids": []}
        for code, p in parent.items():
            b["view"][code]["mapping"] = {
                "links": [{"relation": "parent_context", "target": "wikt:lang:" + p}]
            }
        out = {r["code"]: r for r in export_periods(b, "gen")["languages"]}
        ojp = out["ojp"]
        # The code's own period is unchanged; the span is separate and derived.
        self.assertEqual(ojp["label"], "700–800 CE")
        self.assertEqual(ojp["span"]["label"], "700–1600 CE")
        self.assertEqual(
            [c["code"] for c in ojp["span"]["components"]], ["ojp", "ja-mid-ear", "ja-mid-lat"]
        )
        self.assertEqual(ojp["span"]["undated"], ["ojp-eas"])
        self.assertTrue(ojp["span"]["partial"])
        # A span never stands in for a missing period: sa is not shown, its dated
        # varieties are.
        self.assertNotIn("sa", out)
        self.assertEqual(out["cls"]["label"], "400 BCE–present")

    def test_composition_window_when_nothing_else_is_adopted(self):
        comp = claim(
            "v",
            "vsn",
            "composition",
            "hypothesis",
            years(500, 1500, "BCE", approximate=True),
        )
        first = claim("f", "vsn", "first_attestation", "observation", years(1000, 1100))
        (vsn,) = export_periods(bundle([comp], ["v"]), "gen")["languages"]
        self.assertEqual((vsn["basis"], vsn["label"]), ("composition", "c. 1500–500 BCE"))
        rig = claim(
            "r",
            "sa-rig",
            "composition",
            "hypothesis",
            {"form": "point", "unit": "BCE", "value": 1500, "approximate": True},
        )
        (out,) = export_periods(bundle([rig], ["r"]), "gen")["languages"]
        self.assertEqual(out["label"], "c. 1500 BCE")
        # An adopted attestation takes precedence; the composition stays visible.
        (vsn,) = export_periods(bundle([comp, first], ["v", "f"]), "gen")["languages"]
        self.assertEqual(vsn["basis"], "attested")
        self.assertEqual(vsn["alternatives"][0]["basis"], "composition")

    def test_crown_age_dates_a_proto_languages_end(self):
        crown = {
            "id": "k",
            "status": "supported",
            "subject": {
                "id": "clade:ie",
                "name": "Indo-European (sampled)",
                "type": "sampled_clade",
                "taxa": ["hit", "sa", "la"],
                "mapping": "exact",
            },
            "event": "crown_divergence",
            "role": "posterior",
            "time": {
                "form": "distribution",
                "mean": None,
                "median": -6119,
                "lower": -7999,
                "upper": -4399,
                "interval_type": "hpd",
                "probability": 0.95,
                "ongoing": False,
                "original": {},
            },
            "source_id": "src_many",
            "spans": [{"quote": "q k", "locator": "Table 1"}],
            "analysis_id": "a1",
        }
        b = {
            "claims": [crown],
            "sources": SOURCES,
            "view": {
                "ine-pro": {
                    "name": "Proto-Indo-European",
                    "kind": "reconstructed",
                    "evidence_ids": [],
                }
            },
            "decisions": [
                {
                    "subject_id": "wikt:ine-pro",
                    "event": "crown_divergence",
                    "claim_id": "k",
                    "proto_mapping": "Taxa include Anatolian, Indo-Iranian and Italic.",
                }
            ],
        }
        (pie,) = export_periods(b, "gen")["languages"]
        self.assertEqual((pie["kind"], pie["basis"]), ("proto", "estimated"))
        self.assertIsNone(pie["spoken"]["start"])
        self.assertEqual(pie["spoken"]["end"], {"earliest": -7999, "latest": -4399})
        self.assertEqual(pie["label"], "until c. 8000/4400 BCE")
        self.assertEqual(pie["evidence"][0]["node"], "Indo-European (sampled)")
        # Without the stated correspondence, or for a code that is not a proto
        # language, the crown age is not displayed.
        del b["decisions"][0]["proto_mapping"]
        self.assertEqual(export_periods(b, "gen")["languages"], [])
        b["decisions"][0]["proto_mapping"] = "x"
        # Wiktionary's kind does not decide it: Proto-Norse is filed as a regular code.
        b["view"]["ine-pro"]["kind"] = "regular"
        self.assertEqual(len(export_periods(b, "gen")["languages"]), 1)
        b["view"]["ine-pro"]["name"] = "Latin"
        self.assertEqual(export_periods(b, "gen")["languages"], [])

    def test_only_adopted_supported_exact_evidence_is_displayed(self):
        claims = [
            claim("a", "enm", "conventional_period", "convention", years(1150, 1500)),
            claim(
                "c", "got", "conventional_period", "convention", years(350, 380), status="candidate"
            ),
            claim("s", "la", "conventional_period", "convention", years(1, 200), mapping="subset"),
        ]
        out = export_periods(bundle(claims, ["s"]), "gen")
        self.assertEqual(out["languages"], [])
        # Without a decision, a supported claim is not a period.
        self.assertEqual(export_periods(bundle(claims, []), "gen")["languages"], [])

    def test_bce_years_are_astronomical_and_labels_convert(self):
        raw = years(500, 2600, unit="BCE", approximate=True)
        claims = [claim("a", "akk", "conventional_period", "convention", raw)]
        (akk,) = export_periods(bundle(claims, ["a"]), "gen")["languages"]
        self.assertEqual(akk["spoken"]["start"], {"earliest": -2599, "latest": -2599})
        self.assertEqual(akk["spoken"]["end"], {"earliest": -499, "latest": -499})
        self.assertEqual(akk["label"], "c. 2600–500 BCE")

    def test_century_endpoints_keep_their_width(self):
        raw = {"form": "duration", "unit": "century_CE", "lower": 5, "upper": 12}
        claims = [claim("a", "ang", "conventional_period", "convention", raw)]
        (ang,) = export_periods(bundle(claims, ["a"]), "gen")["languages"]
        self.assertEqual(ang["spoken"]["start"], {"earliest": 401, "latest": 500})
        self.assertEqual(ang["spoken"]["end"], {"earliest": 1101, "latest": 1200})
        self.assertEqual(ang["label"], "5th–12th century CE")

    def test_attestation_window_from_two_events(self):
        first = {
            "form": "event_range",
            "unit": "CE",
            "lower": 350,
            "upper": 380,
            "precision": "decade",
        }
        last = {"form": "point", "unit": "CE", "value": 550, "precision": "year"}
        claims = [
            claim("f", "got", "first_attestation", "observation", first),
            claim("l", "got", "last_attestation", "observation", last),
        ]
        (got,) = export_periods(bundle(claims, ["f", "l"]), "gen")["languages"]
        self.assertEqual((got["kind"], got["basis"]), ("attested", "attested"))
        self.assertEqual(got["spoken"]["start"], {"earliest": 350, "latest": 380})
        self.assertEqual(got["spoken"]["end"], {"earliest": 550, "latest": 550})
        self.assertEqual(got["label"], "350/380–550 CE")
        self.assertEqual([e["claim_id"] for e in got["evidence"]], ["f", "l"])

    def test_proto_is_always_estimated(self):
        raw = years(2500, 4500, unit="BCE", approximate=True)
        claims = [claim("p", "ine-pro", "conventional_period", "convention", raw, "src_many")]
        (pie,) = export_periods(bundle(claims, ["p"], {"ine-pro": "reconstructed"}), "gen")[
            "languages"
        ]
        self.assertEqual((pie["kind"], pie["basis"]), ("proto", "estimated"))
        self.assertEqual(pie["evidence"][0]["source"], "A et al. 2001")

    def test_separately_stated_endpoints_keep_eras_and_uncertainty(self):
        raw = {
            "form": "duration",
            "start": {
                "earliest": {"value": 500, "unit": "BCE"},
                "latest": {"value": 400, "unit": "BCE"},
            },
            "end": {
                "earliest": {"value": 300, "unit": "BCE"},
                "latest": {"value": 1, "unit": "CE"},
            },
            "approximate": True,
        }
        time = normalize_time(raw)
        self.assertEqual(
            (time.lower, time.upper, time.normalization), (-499, 1, "program_verified")
        )
        claims = [claim("g", "gem-pro", "conventional_period", "hypothesis", raw)]
        (gem,) = export_periods(bundle(claims, ["g"], {"gem-pro": "reconstructed"}), "gen")[
            "languages"
        ]
        self.assertEqual(gem["spoken"]["start"], {"earliest": -499, "latest": -399})
        self.assertEqual(gem["spoken"]["end"], {"earliest": -299, "latest": 1})
        self.assertEqual(gem["label"], "c. 500/400 BCE – 300 BCE/1 CE")
        mixed = {
            "form": "duration",
            "start": {"value": 1200, "unit": "BCE"},
            "end": {"value": 400, "unit": "CE"},
        }
        claims = [claim("o", "och", "documentary_sampling", "observation", mixed)]
        (och,) = export_periods(bundle(claims, ["o"]), "gen")["languages"]
        self.assertEqual(och["label"], "1200 BCE – 400 CE")
        self.assertEqual(och["basis"], "attested")
        for bad in (
            {"form": "duration", "unit": "CE", "start": {"value": 1, "unit": "CE"}},
            {"form": "point", "start": {"value": 1, "unit": "CE"}},
            {"form": "duration", "start": {"value": 1}},
            {"form": "duration", "start": {"value": 0, "unit": "BCE"}},
        ):
            with self.assertRaises(ValueError):
                normalize_time(bad)

    def test_open_endpoints_and_family_splits(self):
        self.assertEqual(label(None, (900, 900)), "until 900 CE")
        self.assertEqual(label((1500, 1500), None, ongoing=True), "1500 CE–present")
        self.assertEqual(
            label((-1249, -1249), (220, 220), approximate=True), "c. 1250 BCE – 220 CE"
        )
        # A century start and a year end in one era (Hittite: 19th century – c. 1200 BCE).
        self.assertEqual(
            label((-1899, -1800), (-1199, -1199), approximate=True), "c. 19th century – 1200 BCE"
        )
        self.assertEqual(label((-1899, -1800), (-1599, -1500)), "19th–16th century BCE")
        split = {
            **claim(
                "x",
                "ine",
                "crown_divergence",
                "posterior",
                {
                    "form": "distribution",
                    "unit": "BP",
                    "mean": 8120,
                    "lower": 7000,
                    "upper": 9000,
                    "interval_type": "hpd",
                    "probability": 0.95,
                    "present": 1950,
                },
            ),
            "analysis_id": "an1",
        }
        split["subject"] = {"id": "clade:ie", "name": "Indo-European", "mapping": "exact"}
        out = export_periods(bundle([split], []), "gen")
        self.assertEqual(out["languages"], [])
        (s,) = out["family_splits"]
        self.assertEqual((s["node"], s["analysis"], s["epoch"]), ("Indo-European", "an1", 1950))
        self.assertEqual(s["estimate"]["interval_type"], "hpd")


if __name__ == "__main__":
    unittest.main()
