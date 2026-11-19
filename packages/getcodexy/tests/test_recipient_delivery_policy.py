"""Recipient-bound decisions use a trusted lookup result, not request metadata."""

import sys
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"
sys.path.insert(0, str(HOOKS))

from codexy_policy.envelope import Diagnostic, Request  # noqa: E402
from codexy_policy.thread_delivery import (  # noqa: E402
    RecipientAssignment,
    forbidden,
    resolve_supervised_assignment,
)


class RecipientDeliveryPolicyTests(unittest.TestCase):
    def request(self, **fields: object) -> Request:
        return Request("PreToolUse", "codex_app__send_message_to_thread", fields, None)

    def test_known_parent_requires_its_pair_even_for_supported_worker_pairs(
        self,
    ) -> None:
        parent = RecipientAssignment("assigned", "gpt-6.2-sol", "low", "parent")
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
            child = RecipientAssignment("assigned", model, "medium", "child")
            self.assertFalse(
                forbidden(
                    self.request(threadId="child", model=model, thinking="medium"),
                    child,
                )
            )
            denied = forbidden(
                self.request(threadId="child", model="gpt-6.2-sol", thinking="low"),
                child,
            )
            self.assertIsInstance(denied, Diagnostic)
            self.assertIn(f"{model}/medium", denied.message)

    def test_unavailable_assignment_denies_and_unrelated_uses_existing_policy(
        self,
    ) -> None:
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

    def test_parent_scoped_lookup_binds_both_delivery_directions(self) -> None:
        for model in ("gpt-6.1-terra", "gpt-6.2-sol"):
            record = {
                "parent": "parent",
                "child": "child",
                "model": model,
                "thinking": "medium",
                "state": "active",
            }

            def lookup(parent: str, child: str) -> dict[str, object]:
                self.assertEqual((parent, child), ("parent", "child"))
                return record

            to_child = resolve_supervised_assignment(
                "parent", "parent", "child", lookup
            )
            self.assertEqual(
                to_child, RecipientAssignment("assigned", model, "medium", "child")
            )
            to_parent = resolve_supervised_assignment(
                "parent", "child", "parent", lookup
            )
            self.assertEqual(
                to_parent,
                RecipientAssignment("assigned", "gpt-6.2-sol", "low", "parent"),
            )

    def test_missing_mismatched_and_inactive_records_are_unavailable(self) -> None:
        valid = {
            "parent": "parent",
            "child": "child",
            "model": "gpt-6.1-terra",
            "thinking": "medium",
            "state": "active",
        }
        for record in (
            None,
            {**valid, "parent": "other"},
            {**valid, "child": "other"},
            {**valid, "state": "released"},
            {**valid, "model": {}},
        ):
            with self.subTest(record=record):
                result = resolve_supervised_assignment(
                    "parent", "parent", "child", lambda _parent, _child: record
                )
                self.assertEqual(result.status, "unavailable")

        def unavailable(_parent: str, _child: str) -> None:
            raise OSError("store unavailable")

        self.assertEqual(
            resolve_supervised_assignment(
                "parent", "child", "parent", unavailable
            ).status,
            "unavailable",
        )

    def test_assignment_cannot_be_reused_for_another_recipient(self) -> None:
        record = {
            "parent": "parent",
            "child": "child",
            "model": "gpt-6.1-terra",
            "thinking": "medium",
            "state": "active",
        }
        assignment = resolve_supervised_assignment(
            "parent", "parent", "child", lambda _parent, _child: record
        )
        result = forbidden(
            self.request(threadId="other", model="gpt-6.1-terra", thinking="medium"),
            assignment,
        )
        self.assertIsInstance(result, Diagnostic)
        self.assertEqual(result.code, "ASSIGNMENT_UNAVAILABLE")


if __name__ == "__main__":
    unittest.main()
