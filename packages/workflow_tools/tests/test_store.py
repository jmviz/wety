import json
import os
import stat
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from workflow_tools import store

FAKE_RESTIC = """#!/bin/sh
echo "$@" >> "$FAKE_RESTIC_LOG"
case "$1" in
  backup) echo '{"message_type":"status"}'
          echo '{"message_type":"summary","snapshot_id":"abc123","total_files_processed":2,"total_bytes_processed":10}';;
  snapshots) echo '[{"time":"2026-10-09T00:00:00Z","paths":["/data"]}]';;
  restore) mkdir -p "$4" && echo restored;;
esac
"""


class StoreTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.base = Path(tmp.name)
        bin_dir = self.base / "bin"
        bin_dir.mkdir()
        fake = bin_dir / "restic"
        fake.write_text(FAKE_RESTIC)
        fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
        self.log = self.base / "restic.log"
        env = {
            "PATH": f"{bin_dir}{os.pathsep}{os.environ['PATH']}",
            "RESTIC_REPOSITORY": "/tmp/repo",
            "FAKE_RESTIC_LOG": str(self.log),
        }
        p = patch.dict(os.environ, env)
        p.start()
        self.addCleanup(p.stop)
        self.repo = self.base / "checkout"
        (self.repo / "data/workspaces").mkdir(parents=True)
        (self.repo / "data/workspaces/x.json").write_text("{}")

    def test_snapshot_writes_the_pointer_and_restore_uses_it(self):
        with patch.object(store, "git_head", return_value="deadbeef"):
            pointer = store.snapshot(self.repo / "data", note="first", repo=self.repo)
        self.assertEqual(
            (pointer["snapshot"], pointer["files"], pointer["git_commit"]),
            ("abc123", 2, "deadbeef"),
        )
        self.assertEqual(json.loads((self.repo / store.POINTER).read_text())["snapshot"], "abc123")
        backup = self.log.read_text().splitlines()[0]
        self.assertIn("--exclude *.lock", backup)
        self.assertIn(str((self.repo / "data").resolve()), backup)
        store.restore(self.base / "restored", repo=self.repo)
        self.assertIn(
            f"restore abc123:{(self.repo / 'data').resolve()} --target", self.log.read_text()
        )
        self.assertEqual(store.check(self.repo)["snapshot"], "abc123")
        (self.base / "restored/keep").write_text("x")
        with self.assertRaises(SystemExit):
            store.restore(self.base / "restored", repo=self.repo)

    def test_snapshot_refuses_a_worktree_data_dir_with_links(self):
        (self.repo / "data/cache").symlink_to(self.base)
        with self.assertRaises(SystemExit):
            store.snapshot(self.repo / "data", repo=self.repo)

    def test_sync_copies_missing_files_and_never_overwrites(self):
        src, dst = self.base / "a/data", self.base / "b/data"
        for d in (src, dst):
            (d / "workspaces/dating/_ledger").mkdir(parents=True)
        (src / "workspaces/dating/_ledger/1.json").write_text("one")
        (src / "workspaces/dating/_ledger/2.json").write_text("two")
        (dst / "workspaces/dating/_ledger/2.json").write_text("TWO")
        (src / "registry").mkdir()
        (src / "registry/holdings.json").write_text("new")
        (dst / "registry").mkdir()
        (dst / "registry/holdings.json").write_text("old")
        (src / "linked").symlink_to(self.base)
        report = store.sync(src, dst)
        self.assertEqual((report["to_copy"], report["applied"]), (1, False))
        self.assertFalse((dst / "workspaces/dating/_ledger/1.json").exists())
        kinds = {c["path"]: c["kind"] for c in report["conflicts"]}
        self.assertEqual(
            kinds,
            {"workspaces/dating/_ledger/2.json": "immutable", "registry/holdings.json": "mutable"},
        )
        report = store.sync(src, dst, apply=True)
        self.assertEqual((dst / "workspaces/dating/_ledger/1.json").read_text(), "one")
        self.assertEqual((dst / "registry/holdings.json").read_text(), "old")
        self.assertEqual(store.sync(src, dst)["to_copy"], 0)


if __name__ == "__main__":
    unittest.main()
