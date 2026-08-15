from __future__ import annotations

import json
import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import (
    clear_journal,
    read_journal,
    write_journal,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecycleFinalizationTests(unittest.TestCase):
    def test_failed_committed_journal_write_recovers_without_a_false_receipt(
        self,
    ) -> None:
        with fixture() as state:
            import codexy_runtime_tools.component_lifecycle as lifecycle

            original = lifecycle.write_journal

            def reject_commit(home: object, journal: object) -> None:
                if getattr(journal, "phase") == "committed":
                    raise OSError("committed journal write failed")
                original(home, journal)  # type: ignore[arg-type]

            with (
                patch.object(lifecycle, "write_journal", side_effect=reject_commit),
                self.assertRaisesRegex(OSError, "committed journal"),
            ):
                run_operation(
                    "install",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-commit-write",
                )
            receipt = (
                inventory_path(state.home).parent / "receipts" / "op-commit-write.json"
            )
            self.assertFalse(receipt.exists())
            self.assertEqual(read_journal(state.home).phase, "started")

            next_receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-after-commit-write",
            )
            self.assertEqual(json.loads(receipt.read_text())["outcome"], "completed")
            self.assertEqual(next_receipt["selection_after"], ["core", "devtools"])

    def test_failed_cleanup_keeps_committed_state_for_idempotent_recovery(self) -> None:
        with fixture() as state:
            import codexy_runtime_tools.component_lifecycle as lifecycle

            with (
                patch.object(
                    lifecycle, "clear_journal", side_effect=OSError("cleanup failed")
                ),
                self.assertRaisesRegex(OSError, "cleanup failed"),
            ):
                run_operation(
                    "install",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-cleanup",
                )
            receipt = inventory_path(state.home).parent / "receipts" / "op-cleanup.json"
            self.assertEqual(json.loads(receipt.read_text())["outcome"], "completed")
            self.assertEqual(read_journal(state.home).phase, "committed")

            next_receipt = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-cleanup",
            )
            self.assertEqual(next_receipt, json.loads(receipt.read_text()))
            self.assertFalse(
                (inventory_path(state.home).parent / "inflight.json").exists()
            )

            next_receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-after-cleanup",
            )
            self.assertEqual(next_receipt["selection_after"], ["core", "devtools"])
            self.assertFalse(
                (inventory_path(state.home).parent / "inflight.json").exists()
            )

    def test_receipt_collision_after_commit_recovers_on_the_next_invocation(
        self,
    ) -> None:
        with fixture() as state:
            with (
                patch(
                    "codexy_runtime_tools.component_lifecycle.write_receipt",
                    side_effect=ValueError("operation receipt already exists"),
                ),
                self.assertRaisesRegex(ValueError, "receipt already exists"),
            ):
                run_operation(
                    "install",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-receipt-collision",
                )
            receipt = (
                inventory_path(state.home).parent
                / "receipts"
                / "op-receipt-collision.json"
            )
            self.assertFalse(receipt.exists())
            self.assertEqual(read_journal(state.home).phase, "committed")

            next_receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-after-collision",
            )
            self.assertEqual(json.loads(receipt.read_text())["outcome"], "completed")
            self.assertEqual(next_receipt["selection_after"], ["core", "devtools"])

    def test_committed_recovery_rejects_a_conflicting_existing_receipt_before_local_rewrite(
        self,
    ) -> None:
        with fixture() as state:
            import codexy_runtime_tools.component_lifecycle as lifecycle

            with (
                patch.object(
                    lifecycle, "clear_journal", side_effect=OSError("cleanup failed")
                ),
                self.assertRaisesRegex(OSError, "cleanup failed"),
            ):
                run_operation(
                    "install",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-committed-conflict",
                )
            receipt = (
                inventory_path(state.home).parent
                / "receipts"
                / "op-committed-conflict.json"
            )
            payload = json.loads(receipt.read_text())
            payload["selection_after"] = []
            receipt.write_text(json.dumps(payload, sort_keys=True), encoding="utf-8")

            with (
                patch.object(lifecycle, "write_inventory") as inventory,
                self.assertRaisesRegex(ValueError, "operation receipt"),
            ):
                run_operation(
                    "install",
                    ("devtools",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-after-committed-conflict",
                )
            inventory.assert_not_called()
            self.assertEqual(
                state.mutations, [("plugin", "add", "codexy@codexy", "--json")]
            )
            self.assertEqual(read_journal(state.home).phase, "committed")

    def test_journal_deletion_syncs_its_parent_directory(self) -> None:
        with fixture() as state:
            from codexy_runtime_tools.component_transaction_state import (
                InventorySnapshot,
                Journal,
            )

            journal = Journal(
                "op-delete-sync",
                "install",
                ("core",),
                ("core",),
                (),
                ("core",),
                InventorySnapshot.capture(state.home),
                "started",
            )
            write_journal(state.home, journal)
            with patch(
                "codexy_runtime_tools.component_transaction_state.sync_parent_directory"
            ) as synced:
                clear_journal(state.home)
            synced.assert_called_once_with(inventory_path(state.home).parent)


if __name__ == "__main__":
    unittest.main()
