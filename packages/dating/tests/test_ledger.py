import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from harness import ledger


def put(key, record, kind="decision"):
    return {"kind": kind, "key": key, "op": "put", "record": record}


class LedgerTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.base = Path(tmp.name)
        p = patch.dict("os.environ", {"WETY_SESSION": "test"})
        p.start()
        self.addCleanup(p.stop)

    def test_later_writes_replace_and_unchanged_writes_are_skipped(self):
        a = self.base / "a"
        ledger.append([put("k", {"claim": 1})], path=a)
        self.assertIsNone(ledger.append([put("k", {"claim": 1})], path=a))
        ledger.append([put("k", {"claim": 2})], path=a)
        self.assertEqual(ledger.current("decision", a), ({"k": {"claim": 2}}, {}))
        self.assertEqual(len(list(a.iterdir())), 2)
        ledger.append([{"kind": "decision", "key": "k", "op": "retract", "note": {"r": 1}}], path=a)
        self.assertEqual(ledger.current("decision", a), ({}, {}))
        self.assertEqual(ledger.withdrawals(a)[0]["decision"], {"claim": 2})
        with self.assertRaisesRegex(ValueError, "nothing to retract"):
            ledger.append([{"kind": "decision", "key": "k", "op": "retract"}], path=a)

    def test_concurrent_sessions_merge_by_copying_files_and_conflicts_surface(self):
        a, b = self.base / "a", self.base / "b"
        ledger.append([put("k", {"claim": 1}), put("s", {"t": 1}, "source")], path=a)
        shutil.copytree(a, b)
        # Each session writes the same key without seeing the other.
        ledger.append([put("k", {"claim": 2})], path=a)
        ledger.append([put("k", {"claim": 3}), put("other", {"claim": 9})], path=b)
        for f in b.iterdir():
            if not (a / f.name).exists():
                shutil.copy(f, a / f.name)
        records, conflicts = ledger.current("decision", a)
        self.assertEqual(records, {"other": {"claim": 9}})
        self.assertEqual(sorted(r["claim"] for r in conflicts["k"]), [2, 3])
        # A new write sees both heads and replaces them.
        ledger.append([put("k", {"claim": 3})], path=a)
        self.assertEqual(ledger.current("decision", a)[0]["k"], {"claim": 3})
        self.assertEqual(ledger.current("source", a), ({"s": {"t": 1}}, {}))

    def test_identical_concurrent_records_are_not_a_conflict(self):
        a, b = self.base / "a", self.base / "b"
        ledger.append([put("k", {"claim": 1})], path=a)
        shutil.copytree(a, b)
        ledger.append([put("k", {"claim": 2})], path=a)
        ledger.append([put("k", {"claim": 2})], path=b)
        for f in b.iterdir():
            shutil.copy(f, a / f.name)
        self.assertEqual(ledger.current("decision", a), ({"k": {"claim": 2}}, {}))

    def test_migration_keeps_records_and_withdrawals(self):
        import json

        data = self.base / "data"
        (data / "_decisions").mkdir(parents=True)
        (data / "sources").mkdir()
        d1 = {"subject_id": "wikt:hit", "event": "e", "claim_id": "new"}
        old = {**d1, "claim_id": "old"}
        (data / "_decisions/decisions.json").write_text(json.dumps([d1]))
        (data / "_decisions/withdrawals.json").write_text(
            json.dumps(
                [{"decision": old, "reviewer": "R", "rationale": "why", "withdrawn_at": "2026"}]
            )
        )
        (data / "sources/sources.json").write_text(json.dumps({"s1": {"id": "s1"}}))
        out = ledger.migrate(data, self.base / "l")
        self.assertEqual((out["events"], out["decisions"], out["sources"]), (4, 1, 1))
        self.assertEqual(ledger.current("decision", self.base / "l")[0], {"wikt:hit|e": d1})
        self.assertEqual(ledger.withdrawals(self.base / "l")[0]["rationale"], "why")
        self.assertTrue((data / "_decisions/decisions.json.migrated").exists())
        with self.assertRaisesRegex(ValueError, "already"):
            ledger.migrate(data, self.base / "l")


if __name__ == "__main__":
    unittest.main()
