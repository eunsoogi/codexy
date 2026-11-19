"""Correlate synthetic create responses without releasing uncertain outcomes."""

import json
import sys
import tempfile
import unittest
from pathlib import Path

previous_bytecode_setting = sys.dont_write_bytecode
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"))
from codexy_policy.supervision_observation import admit_create, observe_create
from codexy_policy.supervision_dispatch import Dispatch
from codexy_policy.supervision_reservations import Reservations

sys.dont_write_bytecode = previous_bytecode_setting


class CreateObservationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.store = Reservations(Path(self.directory.name) / "state.sqlite")
        self.input = {"model": "model", "thinking": "medium", "prompt": "assignment"}
        dispatch = Dispatch(self.store)
        dispatch.prepare("parent", "issue", self.input, 1)
        self.assertTrue(dispatch.claim("parent", "call", self.input))
        self.event = {
            "hook_event_name": "PostToolUse",
            "tool_name": "mcp__codex_app__create_thread",
            "session_id": "parent",
            "tool_use_id": "call",
            "tool_input": self.input,
        }

    def observe(self, result):
        self.event["tool_response"] = result
        return observe_create(self.store, self.event)

    def test_pre_call_must_match_the_claimed_request(self):
        pre = {**self.event, "hook_event_name": "PreToolUse"}
        self.assertTrue(admit_create(self.store, pre))
        self.assertFalse(admit_create(self.store, {**pre, "tool_use_id": "other"}))
        self.assertFalse(admit_create(self.store, {**pre, "tool_input": {}}))

    def test_pending_then_ready_keeps_same_slot_and_route(self):
        self.assertEqual(
            self.observe({"structuredContent": {"clientThreadId": "pending"}}),
            "pending",
        )
        self.assertEqual(
            self.observe(
                {
                    "content": [
                        {"type": "text", "text": json.dumps({"threadId": "child"})}
                    ]
                }
            ),
            "ready",
        )
        self.assertEqual(self.store.lookup("parent", "child")["model"], "model")
        self.assertEqual(
            self.store.reserve("parent", "other", "other-issue", "m", "t", 1), "queued"
        )

    def test_ambiguous_failed_and_wrong_call_never_release(self):
        for result in (
            {"isError": True, "structuredContent": {"threadId": "child"}},
            {"content": [{"type": "text", "text": '{"threadId":"a","threadId":"b"}'}]},
            {"structuredContent": {"threadId": "child", "clientThreadId": "pending"}},
            {"structuredContent": {}},
        ):
            self.assertEqual(self.observe(result), "uncertain")
        self.event["tool_use_id"] = "wrong-call"
        self.assertEqual(
            self.observe({"structuredContent": {"threadId": "child"}}), "uncertain"
        )
        self.assertEqual(
            self.store.reserve("parent", "other", "other-issue", "m", "t", 1), "queued"
        )


if __name__ == "__main__":
    unittest.main()
