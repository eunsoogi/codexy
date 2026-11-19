"""Pending setup identity must match the authenticated ready observation."""

import sys
import subprocess
import tempfile
import unittest
from pathlib import Path

from packages.getcodexy.tests.supervision_test_support import (
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
