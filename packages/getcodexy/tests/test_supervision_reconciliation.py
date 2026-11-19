"""Pending setup identity must match the authenticated ready observation."""

import sys
import subprocess
import tempfile
import unittest
from pathlib import Path

from packages.getcodexy.tests.supervision_test_support import (
    dispatch,
    reconciliation,
    reservations,
    storage,
)

resolve_pending = reconciliation.resolve_pending
ReservationError, Reservations = (
    reservations.ReservationError,
    reservations.Reservations,
)
open_store = storage.open_store


class SetupReconciliationTests(unittest.TestCase):
    def test_failed_setup_tombstones_the_old_call_before_retry(self):
        with tempfile.TemporaryDirectory() as root:
            store = open_store(root)
            bridge = dispatch.Dispatch(store)
            bridge.bootstrap("parent", [])
            payload = {"model": "m", "thinking": "t", "prompt": "x"}
            bridge.prepare("parent", "issue", payload)
            request = dispatch.fingerprint(payload)
            self.assertTrue(bridge.claim("parent", "old-call", payload))
            store.setup("parent", request, "pending")
            reconciliation.release_failed(store, "parent", request, "pending")
            self.assertEqual(bridge.prepare("parent", "issue", payload), "reserved")
            self.assertFalse(bridge.claim("parent", "old-call", payload))
            self.assertIsNone(bridge.request_for_call("parent", "old-call"))
            self.assertTrue(bridge.claim("parent", "new-call", payload))

    def test_confirmed_failure_without_pending_id_releases_reserved_slot(self):
        with tempfile.TemporaryDirectory() as root:
            store = open_store(root)
            store.reserve("parent", "request", "issue", "m", "t", 1)
            reconciliation.release_failed(store, "parent", "request", None)
            self.assertEqual(
                store.reserve("parent", "other", "other", "m", "t", 1), "reserved"
            )

    def test_confirmed_ownerless_failure_releases_only_matching_setup(self):
        with tempfile.TemporaryDirectory() as root:
            store = open_store(root)
            store.reserve("parent", "request", "issue", "model", "medium", 1)
            store.setup("parent", "request", "pending")
            with self.assertRaises(ReservationError):
                reconciliation.release_failed(store, "parent", "request", "wrong")
            self.assertEqual(
                store.reserve("parent", "other", "other", "m", "t", 1), "queued"
            )
            reconciliation.release_failed(store, "parent", "request", "pending")
            self.assertEqual(
                store.reserve("parent", "other", "other", "m", "t", 1), "reserved"
            )
            with self.assertRaises(ReservationError):
                resolve_pending(store, "parent", "request", "pending", "late-child")

    def test_failure_cli_requires_confirmation_and_exact_pending(self):
        with tempfile.TemporaryDirectory() as root:
            store = open_store(root)
            store.reserve("parent", "request", "issue", "m", "t", 1)
            store.setup("parent", "request", "pending")
            program = (
                Path(__file__).resolve().parents[3]
                / "plugins/codexy/hooks/codexy_supervision_owner.py"
            )
            args = [
                sys.executable,
                "-I",
                "-B",
                str(program),
                "--plugin-data",
                root,
                "--parent",
                "parent",
                "release-failed",
                "--request",
                "request",
                "--pending",
                "pending",
            ]
            self.assertNotEqual(subprocess.run(args, capture_output=True).returncode, 0)
            self.assertEqual(
                store.reserve("parent", "other", "other", "m", "t", 1), "queued"
            )
            self.assertEqual(
                subprocess.run(
                    [*args, "--confirmed-failed"], capture_output=True
                ).returncode,
                0,
            )
            self.assertEqual(
                store.reserve("parent", "other", "other", "m", "t", 1), "reserved"
            )

    def test_explicit_cli_mapping_requires_matching_pending_identity(self):
        with tempfile.TemporaryDirectory() as root:
            store = open_store(root)
            store.reserve("parent", "request", "issue", "model", "medium")
            store.setup("parent", "request", "pending")
            program = (
                Path(__file__).resolve().parents[3]
                / "plugins/codexy/hooks/codexy_supervision_owner.py"
            )
            result = subprocess.run(
                [
                    sys.executable,
                    "-I",
                    "-B",
                    str(program),
                    "--plugin-data",
                    root,
                    "--parent",
                    "parent",
                    "resolve-pending",
                    "--request",
                    "request",
                    "--pending",
                    "pending",
                    "--child",
                    "child",
                    "--confirmed-ready",
                ],
                text=True,
                capture_output=True,
                check=True,
            )
            self.assertIn('"result":"ready"', result.stdout)
            self.assertEqual(store.lookup("parent", "child")["state"], "active")

    def test_mismatch_preserves_reservation_then_ready_binds_same_slot(self):
        with tempfile.TemporaryDirectory() as root:
            store = Reservations(Path(root) / "state.sqlite")
            store.reserve("parent", "request", "issue", "model", "medium", 1)
            store.setup("parent", "request", "pending")
            with self.assertRaises(ReservationError):
                resolve_pending(store, "parent", "request", "wrong", "child")
            self.assertEqual(
                store.reserve("parent", "other", "other", "m", "t", 1), "queued"
            )
            resolve_pending(store, "parent", "request", "pending", "child")
            resolve_pending(store, "parent", "request", "pending", "child")
            self.assertEqual(store.lookup("parent", "child")["issue"], "issue")
            with self.assertRaises(ReservationError):
                resolve_pending(store, "parent", "request", "pending", "other-child")
