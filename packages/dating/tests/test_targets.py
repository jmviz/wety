import gzip
from pathlib import Path
import tempfile
import unittest

from harness.targets import graph_counts, lang_ids


class TargetTests(unittest.TestCase):
    def test_lang_ids_follow_sorted_main_codes(self):
        langs = {
            "la": {"mainCode": "la"},
            "en": {"mainCode": "en"},
            "ang": {"mainCode": "ang"},
        }
        self.assertEqual(lang_ids(langs), {0: "ang", 1: "en", 2: "la"})

    def test_graph_counts_stream_across_blocks(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "g.json.gz"
            items = ",".join('{"lang":%d,"term":"x"}' % (i % 3) for i in range(1000))
            with gzip.open(path, "wt") as f:
                f.write('{"items":[' + items + "]}")
            counts = graph_counts(path, chunk=97)
        self.assertEqual(dict(counts), {0: 334, 1: 333, 2: 333})


if __name__ == "__main__":
    unittest.main()
