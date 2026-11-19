"""A released owner must reacquire capacity before a later turn."""

import concurrent.futures
import tempfile
import unittest
from pathlib import Path

from packages.getcodexy.tests.supervision_test_support import reservations, resume

Reservations, resume_decision = reservations.Reservations, resume.resume_decision


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
