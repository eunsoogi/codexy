"""Check that a prepared request authorizes at most one creation call."""

import concurrent.futures
import sys
import tempfile
import unittest
from pathlib import Path

previous_bytecode_setting = sys.dont_write_bytecode
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"))
from codexy_policy.supervision_dispatch import Dispatch, fingerprint
from codexy_policy.supervision_reservations import ReservationError, Reservations

sys.dont_write_bytecode = previous_bytecode_setting


class DispatchTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.store = Reservations(Path(self.directory.name) / "state.sqlite")
        self.dispatch = Dispatch(self.store)
        self.input = {"model": "model", "thinking": "medium", "prompt": "private"}
        self.assertEqual(
            self.dispatch.prepare("parent", "issue", self.input, 1), "reserved"
        )

    def test_concurrent_calls_and_duplicate_delivery(self):
        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
            results = list(
                pool.map(
                    lambda call: self.dispatch.claim("parent", call, self.input),
                    ("a", "b"),
                )
            )
        self.assertCountEqual(results, [True, False])
        winner = "a" if results[0] else "b"
        self.assertTrue(self.dispatch.claim("parent", winner, self.input))
        self.assertEqual(
            self.dispatch.request_for_call("parent", winner), fingerprint(self.input)
        )

    def test_changed_payload_parent_and_stale_call_fail_closed(self):
        self.assertFalse(self.dispatch.claim("other-parent", "a", self.input))
        self.assertFalse(
            self.dispatch.claim("parent", "a", {**self.input, "prompt": "other"})
        )
        self.assertTrue(self.dispatch.claim("parent", "a", self.input))
        request = fingerprint(self.input)
        self.store.release("parent", request, confirmed=True)
        self.assertIsNone(self.dispatch.request_for_call("parent", "a"))
        self.assertEqual(
            self.dispatch.prepare("parent", "issue", self.input, 1), "reserved"
        )
        self.assertFalse(self.dispatch.claim("parent", "a", self.input))
        self.assertTrue(self.dispatch.claim("parent", "b", self.input))
        self.assertIsNone(self.dispatch.request_for_call("parent", "a"))

    def test_issue_binding_cannot_be_replaced_by_prompt_or_title(self):
        with self.assertRaises(ReservationError):
            self.dispatch.prepare("parent", "different-issue", self.input, 1)
        self.assertFalse(
            self.dispatch.claim("parent", "call", {**self.input, "model": "other"})
        )


if __name__ == "__main__":
    unittest.main()
