import unittest
from harness.evidence import normalize_time
from harness.llm import ExtractedTime, SourceTime, _time_value
from harness.time_interpretation import native_numbers_match, UnresolvedCalendar


class TimeInterpretationTests(unittest.TestCase):
    def test_convention_distinct_from_source_epoch_and_no_mutation(self):
        raw = {
            "form": "point",
            "unit": "BP",
            "value": 5000,
            "present": None,
            "age_scale": "calendar",
            "epoch_interpretation": {
                "basis": "conventional_bp_1950",
                "notation": "BP",
                "epoch": 1950,
            },
        }
        t = normalize_time(raw)
        self.assertEqual(t.value, -3050)
        self.assertIsNone(t.original["present"])
        self.assertIsNone(raw["present"])
        self.assertTrue(native_numbers_match(raw, "An estimate of 5000 B.P."))
        self.assertFalse(native_numbers_match(raw, "An estimate of 5000 years ago."))
        with self.assertRaises(ValueError):
            normalize_time({**raw, "present": 2000})
        self.assertEqual(
            normalize_time(
                {k: v for k, v in {**raw, "present": 2000}.items() if k != "epoch_interpretation"}
            ).value,
            -3000,
        )

    def test_radiocarbon_and_unknown_scale_cannot_use_subtraction(self):
        for scale in ("radiocarbon_uncalibrated", "unknown"):
            raw = {
                "form": "point",
                "unit": "BP",
                "value": 5000,
                "present": 1950,
                "age_scale": scale,
            }
            with self.assertRaises(UnresolvedCalendar):
                normalize_time(raw)
            self.assertTrue(
                _time_value(
                    ExtractedTime(expression="5000 BP", conversion="", source_values=raw)
                ).unknown
            )
        # Legacy native records are unchanged; they do not claim a scale review.
        self.assertEqual(
            normalize_time({"form": "point", "unit": "BP", "value": 5000, "present": 1950}).value,
            -3050,
        )

    def test_no_convention_for_bare_years_ago_or_non_bp_units(self):
        for unit, notation in [("BP", "years ago"), ("BCE", "BP")]:
            with self.assertRaises(ValueError):
                SourceTime(
                    form="point",
                    unit=unit,
                    value=3000,
                    epoch_interpretation={
                        "basis": "conventional_bp_1950",
                        "notation": notation,
                        "epoch": 1950,
                    },
                )

    def test_bare_relative_age_keeps_quantity_without_guessed_epoch(self):
        for unit, value in [("years_ago", 5000), ("ka_ago", 5)]:
            raw = {"form": "point", "unit": unit, "value": value, "age_scale": "calendar"}
            t = ExtractedTime(expression="5000 years ago", conversion="", source_values=raw)
            self.assertTrue(_time_value(t).unknown)
            self.assertEqual(t.source_values.value, value)
            self.assertEqual(normalize_time({**raw, "present": 2000}).value, -3000)
            with self.assertRaises(ValueError):
                normalize_time(
                    {
                        **raw,
                        "epoch_interpretation": {
                            "basis": "conventional_bp_1950",
                            "notation": "BP",
                            "epoch": 1950,
                        },
                    }
                )

    def test_years_ago_has_its_own_labeled_convention(self):
        # Maintainer decision 2026-10-06: rough expert estimates stated only as "years
        # ago" may be read before AD 1950, labeled as such; never under the BP label.
        raw = {"form": "point", "unit": "years_ago", "value": 2500, "age_scale": "calendar"}
        convention = {
            "basis": "conventional_years_ago_1950",
            "notation": "years ago",
            "epoch": 1950,
        }
        self.assertEqual(normalize_time({**raw, "epoch_interpretation": convention}).value, -550)
        with self.assertRaises(ValueError):
            normalize_time({**raw, "unit": "BP", "epoch_interpretation": convention})
        with self.assertRaises(ValueError):
            normalize_time(
                {**raw, "epoch_interpretation": {**convention, "basis": "conventional_bp_1950"}}
            )

    def test_uncalibrated_bc_is_not_a_calendar_date_either(self):
        with self.assertRaises(UnresolvedCalendar):
            normalize_time(
                {
                    "form": "point",
                    "unit": "BCE",
                    "value": 3000,
                    "age_scale": "radiocarbon_uncalibrated",
                }
            )

    def test_coarse_bce_period_and_by_bound(self):
        raw = {"form": "event_range", "unit": "millennium_BCE", "lower": 3, "upper": 3}
        t = normalize_time(raw)
        self.assertEqual((t.lower, t.upper, t.precision), (-2999, -2000, "millennium"))
        self.assertTrue(native_numbers_match(raw, "beginning in the third millennium B.C.E."))
        bound = {"form": "bound", "unit": "century_BCE", "value": 10, "relation": "by"}
        self.assertEqual(normalize_time(bound).value, -900)
        self.assertTrue(native_numbers_match(bound, "already by the 10th century B.C.E."))
        self.assertFalse(
            native_numbers_match({**bound, "unit": "BCE"}, "already by the 10th century B.C.E.")
        )
        self.assertFalse(
            native_numbers_match(
                {**raw, "lower": 2001, "upper": 3000, "unit": "BCE"}, "third millennium BCE"
            )
        )

    def test_coarse_era_orientation_and_invalid_precision(self):
        self.assertEqual(
            normalize_time(
                {"form": "event_range", "unit": "century_CE", "lower": 1, "upper": 2}
            ).upper,
            200,
        )
        for value in (0, 1.5, -2):
            with self.assertRaises(ValueError):
                SourceTime(form="bound", unit="century_BCE", value=value, relation="by")
        with self.assertRaises(ValueError):
            SourceTime(form="point", unit="century_BCE", value=10)
        self.assertFalse(
            native_numbers_match({"value": 3, "unit": "century_BCE"}, "third millennium BCE")
        )

    def test_ninety_percent_hpd_never_becomes_hpd95(self):
        raw = {
            "form": "distribution",
            "unit": "CE",
            "lower": 1000,
            "upper": 1200,
            "interval_type": "hpd",
            "probability": 0.9,
        }
        t = ExtractedTime(expression="90% HPD 1000–1200 CE", conversion="", source_values=raw)
        self.assertIsNone(_time_value(t).hpd95)
        self.assertTrue(_time_value(t).unknown)
        self.assertEqual(normalize_time(raw).probability, 0.9)
        raw["mean"] = 1100
        self.assertEqual(
            _time_value(ExtractedTime(expression="", conversion="", source_values=raw)).mean, 1100
        )


if __name__ == "__main__":
    unittest.main()
