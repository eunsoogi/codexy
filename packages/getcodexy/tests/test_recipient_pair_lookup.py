"""Verify direction discovery against authenticated sender and recipient IDs."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "plugins/codexy/hooks"))
from codexy_policy.thread_delivery import RecipientAssignment, resolve_known_pair  # noqa: E402


class RecipientPairLookupTests(unittest.TestCase):
    def test_lookup_checks_both_directions_and_never_infers_unrelated(self) -> None:
        record = {
            "parent": "parent",
            "child": "child",
            "model": "gpt-6.1-terra",
            "thinking": "medium",
            "state": "active",
        }

        def lookup(parent: str, child: str) -> dict[str, object] | None:
            return record if (parent, child) == ("parent", "child") else None

        self.assertEqual(
            resolve_known_pair("parent", "child", lookup),
            RecipientAssignment("assigned", "gpt-6.1-terra", "medium", "child"),
        )
        self.assertEqual(
            resolve_known_pair("child", "parent", lookup),
            RecipientAssignment("assigned", "gpt-6.2-sol", "low", "parent"),
        )
        self.assertEqual(
            resolve_known_pair("parent", "unknown", lookup).status, "unavailable"
        )
        self.assertEqual(
            resolve_known_pair("parent", "child", lambda _p, _c: record).status,
            "unavailable",
        )
