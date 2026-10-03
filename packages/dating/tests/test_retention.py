import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from harness.retention import apply, plan, tracked_references


def digest(n):
    return f"{n:064x}"


class RetentionTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        base = Path(tmp.name)
        self.root, self.product, self.repo, self.history = (
            base / d for d in ("generations", "product", "repo", "history")
        )
        for d in (self.root, self.product, self.repo):
            d.mkdir()
        # Generation n is published at time n; 5 is the newest, 3 is current.
        for n in range(1, 6):
            g = self.root / digest(n)
            g.mkdir()
            (g / "bundle.json").write_text("x" * 10)
            (g / "manifest.json").write_text(json.dumps({"generation": digest(n)}))
            os.utime(g / "manifest.json", (n, n))
        (self.root / "stray").mkdir()
        (self.product / "CURRENT.json").write_text(json.dumps({"generation": digest(3)}))
        subprocess.run(["git", "init", "-q"], cwd=self.repo, check=True)
        (self.repo / "packet.json").write_text(json.dumps({"baseline_generation": digest(1)}))
        (self.repo / "untracked.json").write_text(digest(2))
        inventory = self.repo / "data/registry/migrations/m"
        inventory.mkdir(parents=True)
        (inventory / "inventory.json").write_text(digest(4) + digest(2))
        subprocess.run(["git", "add", "packet.json", "data"], cwd=self.repo, check=True)

    def plan(self, **kw):
        return plan(root=self.root, product=self.product, repo=self.repo, **kw)

    def test_keeps_current_recent_and_tracked_references(self):
        result = self.plan()
        self.assertEqual(result["keep"][digest(3)], ["current"])
        self.assertEqual(result["keep"][digest(5)], ["recent"])
        self.assertEqual(result["keep"][digest(4)], ["recent"])
        self.assertEqual(result["keep"][digest(1)], ["referenced by packet.json"])
        # Only tracked files count as references.
        self.assertEqual(result["remove"], [digest(2)])
        self.assertEqual(result["unknown_directories"], ["stray"])
        self.assertEqual(result["remove_bytes"], 10 + len(json.dumps({"generation": digest(2)})))

    def test_dry_run_deletes_nothing_and_apply_logs(self):
        result = self.plan(keep_recent=0)
        # The migration inventory names 2 and 4 but does not keep them.
        self.assertEqual(sorted(result["remove"]), [digest(2), digest(4), digest(5)])
        self.assertTrue((self.root / digest(2)).exists())
        applied = apply(result, root=self.root, history=self.history)
        self.assertEqual(applied["removed"], 3)
        self.assertEqual(
            sorted(p.name for p in self.root.iterdir()), sorted([digest(1), digest(3), "stray"])
        )
        (log,) = (self.history / "gc").glob("*.json")
        self.assertEqual(len(json.loads(log.read_text())["removed"]), 3)

    def test_current_must_exist(self):
        (self.product / "CURRENT.json").write_text(json.dumps({"generation": digest(9)}))
        with self.assertRaises(ValueError):
            self.plan()

    def test_reference_scan_ignores_unknown_digests(self):
        self.assertEqual(tracked_references([digest(7)], self.repo), {})


if __name__ == "__main__":
    unittest.main()
