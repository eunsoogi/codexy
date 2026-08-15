"""Durable recovery admission cases."""

import json
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import (
    InventorySnapshot,
    Journal,
    read_journal,
    write_journal,
)
from packages.getcodexy.tests.component_lifecycle_admission_receipts import make_receipt
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecycleAdmissionRecoveryCases:
    def test_pending_journal_receipt_is_admitted_before_recovery_for_a_distinct_caller(
        self,
    ) -> None:
        with fixture({"core"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.installed-component-inventory.v1",
                        "components": ["core"],
                    }
                ),
                encoding="utf-8",
            )
            journal = Journal(
                "op-pending-receipt",
                "update",
                (),
                ("core",),
                ("core",),
                ("core",),
                InventorySnapshot.capture(state.home),
                "started",
            )
            write_journal(state.home, journal)
            receipt = target.parent / "receipts" / "op-pending-receipt.json"
            receipt.parent.mkdir(parents=True)
            receipt.write_text(
                json.dumps(
                    make_receipt(
                        "op-pending-receipt",
                        "update",
                        (),
                        ("core",),
                        ("core",),
                        ("core",),
                        "completed",
                    ),
                    sort_keys=True,
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "pending transaction receipt"):
                run_operation(
                    "install",
                    ("devtools",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-distinct-caller",
                )
            self.assertEqual(state.mutations, [])
            self.assertEqual(read_journal(state.home), journal)

    def test_absent_snapshot_recovery_uses_durable_host_selection_not_an_empty_inventory(
        self,
    ) -> None:
        with fixture({"core"}) as state:
            journal = Journal(
                "op-no-snapshot",
                "install",
                ("github",),
                ("core", "github"),
                ("core",),
                ("core", "github"),
                InventorySnapshot.capture(state.home),
                "started",
            )
            write_journal(state.home, journal)
            receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-after-no-snapshot",
            )
            self.assertEqual(receipt["selection_after"], ["core", "devtools"])

    def test_rolling_back_empty_snapshot_receipt_cleans_up_without_inventory_file(
        self,
    ) -> None:
        with fixture() as state:
            journal = Journal(
                "op-empty-snapshot",
                "install",
                ("core",),
                ("core",),
                (),
                ("core",),
                InventorySnapshot.capture(state.home),
                "rolling-back",
            )
            write_journal(state.home, journal)
            receipt = (
                inventory_path(state.home).parent
                / "receipts"
                / "op-empty-snapshot.json"
            )
            receipt.parent.mkdir(parents=True)
            payload = make_receipt(
                "op-empty-snapshot",
                "install",
                ("core",),
                ("core",),
                (),
                (),
                "rolled-back",
                [{"code": "operation-failed"}],
            )
            receipt.write_text(json.dumps(payload, sort_keys=True), encoding="utf-8")
            self.assertEqual(
                run_operation(
                    "install",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-empty-snapshot",
                ),
                payload,
            )
