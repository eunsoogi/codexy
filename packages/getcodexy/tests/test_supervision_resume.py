"""A released owner must reacquire capacity before a later turn."""

import concurrent.futures
import sys
import tempfile
import unittest
from pathlib import Path

previous_bytecode_setting = sys.dont_write_bytecode
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"))
from codexy_policy.supervision_reservations import Reservations
from codexy_policy.supervision_resume import resume_decision

sys.dont_write_bytecode = previous_bytecode_setting


class ResumeTests(unittest.TestCase):
    def test_release_and_competing_resume_preserve_owner(self):
        with tempfile.TemporaryDirectory() as root:
            store = Reservations(Path(root) / "state.sqlite")
            store.adopt("parent", "old", "issue", "model", "medium", "child")
            self.assertEqual(resume_decision(store, "parent", "child"), "active")
            store.release("parent", "old", confirmed=True)
            self.assertEqual(resume_decision(store, "parent", "child"), "released")
            self.assertEqual(
                store.reserve("parent", "other", "other-issue", "m", "t", 1), "reserved"
            )
            self.assertEqual(
                store.reserve("parent", "old", "issue", "model", "medium", 1), "queued"
            )
            store.release("parent", "other", confirmed=True)
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
                results = list(
                    pool.map(
                        lambda _: store.reserve(
                            "parent", "old", "issue", "model", "medium", 1
                        ),
                        range(2),
                    )
                )
            self.assertCountEqual(results, ["resumed", "existing"])
            self.assertEqual(resume_decision(store, "parent", "child"), "active")
            self.assertEqual(resume_decision(store, "parent", "unknown"), "unknown")
