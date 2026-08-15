"""Terminal receipt admission cases."""

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


class LifecycleAdmissionTerminalCases:
    def test_pending_receipt_table_handles_malformed_and_exact_rollback_recovery(
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
                "op-pending-rollback",
                "update",
                (),
                ("core",),
                ("core",),
                ("core",),
                InventorySnapshot.capture(state.home),
                "rolling-back",
            )
            write_journal(state.home, journal)
            receipt = target.parent / "receipts" / "op-pending-rollback.json"
            receipt.parent.mkdir(parents=True)
            payload = make_receipt(
                "op-pending-rollback",
                "update",
                (),
                ("core",),
                ("core",),
                ("core",),
                "rolled-back",
                [{"code": "operation-failed"}],
            )
            receipt.write_text(json.dumps(payload, sort_keys=True), encoding="utf-8")

            with patch(
                "codexy_runtime_tools.component_transaction_state.write_journal"
            ) as journal_write:
                replay = run_operation(
                    "update",
                    (),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-pending-rollback",
                )
            self.assertEqual(replay, payload)
            journal_write.assert_not_called()
            self.assertEqual(state.mutations, [])
            self.assertIsNone(read_journal(state.home))

    def test_terminal_update_rollback_does_not_admit_mixed_version_host_state(
        self,
    ) -> None:
        with fixture(
            {"core", "github"}, versions={"core": "1.3.0", "github": "1.2.0"}
        ) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.installed-component-inventory.v1",
                        "components": ["core", "github"],
                    }
                ),
                encoding="utf-8",
            )
            journal = Journal(
                "op-terminal-mixed",
                "update",
                ("github",),
                ("core", "github"),
                ("core", "github"),
                ("core", "github"),
                InventorySnapshot.capture(state.home),
                "rolling-back",
            )
            write_journal(state.home, journal)
            receipt = target.parent / "receipts" / "op-terminal-mixed.json"
            receipt.parent.mkdir(parents=True)
            receipt.write_text(
                json.dumps(
                    make_receipt(
                        "op-terminal-mixed",
                        "update",
                        ("github",),
                        ("core", "github"),
                        ("core", "github"),
                        ("core", "github"),
                        "rolled-back",
                        [{"code": "operation-failed"}],
                    ),
                    sort_keys=True,
                ),
                encoding="utf-8",
            )

            rejected = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-after-terminal-mixed",
            )

            self.assertEqual(rejected["errors"], [{"code": "mixed-version-state"}])
            self.assertEqual(state.mutations, [])
            self.assertEqual(read_journal(state.home), journal)

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
                "op-pending-malformed",
                "update",
                (),
                ("core",),
                ("core",),
                ("core",),
                InventorySnapshot.capture(state.home),
                "started",
            )
            write_journal(state.home, journal)
            receipt = target.parent / "receipts" / "op-pending-malformed.json"
            receipt.parent.mkdir(parents=True)
            payload = make_receipt(
                "op-pending-malformed",
                "update",
                (),
                ("core",),
                ("core",),
                ("core",),
                "completed",
            )
            payload["unknown"] = True
            receipt.write_text(json.dumps(payload, sort_keys=True), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "operation receipt"):
                run_operation(
                    "install",
                    ("devtools",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-different-after-malformed",
                )
            self.assertEqual(state.mutations, [])
            self.assertEqual(read_journal(state.home), journal)
