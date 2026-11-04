"""The generation report's reading of edge-dump differences and readings."""

import json
import tempfile
import unittest
from pathlib import Path

from etymology import generation_report as G


def line(child, parent, followed="true", mode="inherited"):
    return "\t".join(
        ["la", child, "1", "entry", "itc-pro", parent, "1", "imputed", mode]
        + ["0", "true", "template", "0", "stated", followed]
    )


class Report(unittest.TestCase):
    def test_changed_words_and_tiers(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            (tmp / "diff.tsv").write_text(
                "\n".join(
                    [
                        line("canis", "kan-"),
                        "\t" + line("canis", "kan-", followed="false"),
                        line("lupus", "lukos"),
                        "\t" + line("lupus", "lukos", mode="borrowed"),
                        line("nix", "snigʷs", followed="false"),
                    ]
                )
                + "\n"
            )
            reading = {
                "word": "wiktionary:la:canis#1",
                "tier": "unsettled",
                "disputed": True,
                "primary": None,
                "etymologies": [{"claims": []}],
            }
            (tmp / "readings.jsonl").write_text(json.dumps(reading) + "\n")
            result = G.report(tmp / "readings.jsonl", tmp / "diff.tsv", [], tmp / "out")
        # canis no longer followed through kan-; lupus's mode changed; nix's line
        # was never followed
        self.assertEqual(result["words whose followed parents changed"], 2)
        self.assertEqual(result["changed, touched by claims"], 1)
        self.assertEqual(result["by tier"], {"unsettled": 1})


if __name__ == "__main__":
    unittest.main()
