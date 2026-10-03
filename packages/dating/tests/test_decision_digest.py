import copy
import unittest

from harness.registry import decision_digest


def claim(cid, subject, value=1):
    return {"id": cid, "subject": {"id": subject}, "source_id": "s" + cid, "time": {"value": value}}


class DecisionDigestTests(unittest.TestCase):
    def setUp(self):
        self.claims = [claim("a", "wikt:ang"), claim("b", "wikt:ang"), claim("c", "wikt:enm")]
        self.sources = {"sa": {"sha256": "1"}, "sb": {"sha256": "2"}, "sc": {"sha256": "3"}}
        self.catalog = {"links": [{"target": "iso639-3:ang"}]}
        self.base = self.digest()

    def digest(self, claims=None, sources=None, catalog=None):
        return decision_digest(
            claims or self.claims, sources or self.sources, catalog or self.catalog, "wikt:ang"
        )

    def test_other_subjects_and_order_do_not_matter(self):
        claims = copy.deepcopy(self.claims)
        claims[2]["time"]["value"] = 99
        sources = {**self.sources, "sc": {"sha256": "changed"}}
        self.assertEqual(self.digest(claims[::-1], sources), self.base)

    def test_subject_evidence_sources_and_mapping_matter(self):
        claims = copy.deepcopy(self.claims)
        claims[1]["time"]["value"] = 2
        self.assertNotEqual(self.digest(claims), self.base)
        self.assertNotEqual(self.digest(self.claims + [claim("d", "wikt:ang")]), self.base)
        self.assertNotEqual(self.digest(sources={**self.sources, "sa": {"sha256": "x"}}), self.base)
        self.assertNotEqual(self.digest(catalog={"links": []}), self.base)


if __name__ == "__main__":
    unittest.main()
