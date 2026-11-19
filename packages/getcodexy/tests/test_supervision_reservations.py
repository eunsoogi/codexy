"""Exercise parent capacity, identity reconciliation, and concurrent admission."""

import concurrent.futures
import importlib.util
from pathlib import Path
import tempfile
import unittest

MODULE = (
    Path(__file__).resolve().parents[3]
    / "plugins/codexy/hooks/codexy_policy/supervision_reservations.py"
)
spec = importlib.util.spec_from_file_location("supervision_reservations", MODULE)
assert spec and spec.loader
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReservationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "reservations.sqlite"
        self.store = module.Reservations(self.path)

    def reserve(self, number, **kwargs):
        return self.store.reserve(
            "parent", str(number), f"issue-{number}", "model", "medium", **kwargs
        )

    def test_backlog_and_confirmed_release(self):
        for number in range(4):
            self.assertEqual(self.reserve(number), "reserved")
            self.store.ready("parent", str(number), f"child-{number}")
        self.assertEqual(
            [self.reserve(n) for n in range(4, 11)], ["reserved"] * 4 + ["queued"] * 3
        )
        with self.assertRaises(module.ReservationError):
            self.store.release("parent", "0", confirmed=False)
        self.assertEqual(self.reserve(8), "queued")
        self.store.release("parent", "0", confirmed=True)
        self.assertEqual(self.reserve(8), "reserved")
        self.assertEqual(self.reserve(9), "queued")
        with self.assertRaises(module.ReservationError):
            self.store.reserve("parent", "new-owner", "issue-0", "model", "medium")
        self.assertEqual(self.reserve(0), "queued")
        self.store.release("parent", "1", confirmed=True)
        self.assertEqual(self.reserve(0), "resumed")
        self.assertEqual(self.store.lookup("parent", "child-0")["state"], "active")

    def test_concurrent_last_slot(self):
        for number in range(7):
            self.reserve(number)
        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
            results = list(pool.map(self.reserve, (7, 8)))
        self.assertCountEqual(results, ["reserved", "queued"])

    def test_setup_restart_and_duplicate_owner(self):
        self.reserve(1)
        self.store.setup("parent", "1", "pending")
        restored = module.Reservations(self.path)
        self.assertEqual(
            restored.reserve("parent", "1", "issue-1", "model", "medium"), "existing"
        )
        with self.assertRaises(module.ReservationError):
            restored.reserve("parent", "another", "issue-1", "model", "medium")
        restored.ready("parent", "1", "child")
        self.assertEqual(restored.lookup("parent", "child")["model"], "model")
        with self.assertRaises(module.ReservationError):
            restored.ready("parent", "1", "other-child")

    def test_failed_setup_retries_original_request_without_duplicate_owner(self):
        self.reserve(1)
        self.store.setup("parent", "1", "pending")
        self.store.release("parent", "1", confirmed=True)
        self.assertEqual(self.reserve(1), "reserved")
        self.store.setup("parent", "1", "new-pending")
        self.store.ready("parent", "1", "child")
        self.assertEqual(self.store.lookup("parent", "child")["issue"], "issue-1")

    def test_parent_isolation_and_over_limit_preserves_owners(self):
        for number in range(3):
            self.reserve(number)
        self.assertEqual(self.reserve(3, limit=2), "queued")
        self.assertEqual(
            self.store.reserve("other-parent", "4", "issue-4", "model", "medium", 1),
            "reserved",
        )
        self.assertEqual(self.reserve(0, limit=1), "existing")

    def test_adopt_over_capacity_and_pending_identity_collision(self):
        for number in range(9):
            self.store.adopt(
                "parent",
                str(number),
                f"issue-{number}",
                "model",
                "medium",
                f"child-{number}",
            )
        self.store.adopt("parent", "0", "issue-0", "model", "medium", "child-0")
        self.assertEqual(self.reserve(9), "queued")
        self.store.release("parent", "0", confirmed=True)
        self.assertEqual(self.reserve(9), "queued")
        self.store.release("parent", "1", confirmed=True)
        self.assertEqual(self.reserve(9), "reserved")
        self.store.setup("parent", "9", "pending")
        self.store.release("parent", "2", confirmed=True)
        self.assertEqual(self.reserve(10), "reserved")
        with self.assertRaises(module.ReservationError):
            self.store.setup("parent", "10", "pending")

    def test_lower_limit_and_uncertain_outcome(self):
        self.assertEqual(self.reserve(1, limit=1), "reserved")
        self.assertEqual(self.reserve(2, limit=1), "queued")
        self.store.setup("parent", "1", "delayed")
        self.assertEqual(self.reserve(2, limit=1), "queued")


if __name__ == "__main__":
    unittest.main()
