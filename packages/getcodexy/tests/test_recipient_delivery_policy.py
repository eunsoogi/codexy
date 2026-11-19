"""Recipient-bound decisions use a trusted lookup result, not request metadata."""

import sys
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"
sys.path.insert(0, str(HOOKS))

from codexy_policy.envelope import Diagnostic, Request  # noqa: E402
from codexy_policy.thread_delivery import RecipientAssignment, forbidden  # noqa: E402


class RecipientDeliveryPolicyTests(unittest.TestCase):
    def request(self, **fields: object) -> Request:
        return Request("PreToolUse", "codex_app__send_message_to_thread", fields, None)

    def test_known_parent_requires_its_pair_even_for_supported_worker_pairs(self) -> None:
        parent = RecipientAssignment("assigned", "gpt-6.2-sol", "low")
        for fields in (
            {},
            {"model": "gpt-6.1-terra", "thinking": "medium"},
            {"model": "gpt-6.2-sol", "thinking": "medium"},
        ):
            with self.subTest(fields=fields):
                result = forbidden(self.request(threadId="parent", **fields), parent)
                self.assertIsInstance(result, Diagnostic)
                self.assertIn("gpt-6.2-sol/low", result.message)
        self.assertFalse(
            forbidden(
                self.request(threadId="parent", model="gpt-6.2-sol", thinking="low"),
                parent,
            )
        )

    def test_children_require_their_own_assignments(self) -> None:
        for model in ("gpt-6.1-terra", "gpt-6.2-sol"):
            child = RecipientAssignment("assigned", model, "medium")
            self.assertFalse(
                forbidden(
                    self.request(threadId="child", model=model, thinking="medium"), child
                )
            )
            denied = forbidden(
                self.request(threadId="child", model="gpt-6.2-sol", thinking="low"), child)
            self.assertIsInstance(denied, Diagnostic)
            self.assertIn(f"{model}/medium", denied.message)

    def test_unavailable_assignment_denies_and_unrelated_uses_existing_policy(self) -> None:
        request = self.request(
            threadId="recipient", model="gpt-6.1-terra", thinking="medium"
        )
        result = forbidden(request, RecipientAssignment("unavailable"))
        self.assertIsInstance(result, Diagnostic)
        self.assertEqual(result.code, "ASSIGNMENT_UNAVAILABLE")
        self.assertFalse(forbidden(request, RecipientAssignment("unrelated")))
        forged = self.request(
            threadId="recipient",
            model="gpt-6.1-terra",
            thinking="medium",
            recipient_assignment={"model": "gpt-6.1-terra", "thinking": "medium"},
        )
        result = forbidden(forged, RecipientAssignment("unavailable"))
        self.assertIsInstance(result, Diagnostic)
        self.assertEqual(result.code, "ASSIGNMENT_UNAVAILABLE")

    def test_invalid_assignment_and_missing_recipient_fail_closed(self) -> None:
        bad = RecipientAssignment("assigned", "gpt-6.1-terra", "low")
        result = forbidden(self.request(threadId="recipient"), bad)
        self.assertIsInstance(result, Diagnostic)
        self.assertEqual(result.code, "ASSIGNMENT_UNAVAILABLE")
        result = forbidden(
            self.request(model="gpt-6.2-sol", thinking="low"),
            RecipientAssignment("assigned", "gpt-6.2-sol", "low"),
        )
        self.assertIsInstance(result, Diagnostic)
        self.assertEqual(result.code, "MISSING_RECIPIENT")


if __name__ == "__main__":
    unittest.main()
