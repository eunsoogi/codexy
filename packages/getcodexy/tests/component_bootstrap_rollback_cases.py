"""Durable bootstrap rollback scenarios shared by the bootstrap test case."""

from __future__ import annotations

from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_transaction_receipts import write_receipt
from codexy_runtime_tools.component_transaction_state import (
    InventorySnapshot,
    Journal,
    read_inventory,
    read_journal,
    write_inventory,
    write_journal,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture


class BootstrapRollbackCases:
    def test_rolling_back_bootstrap_restores_distinct_live_and_durable_prestates(
        self,
    ) -> None:
        with fixture({"core"}) as state:
            durable = b'{"schema":"getcodexy.installed-component-inventory.v1","components":["core","github"]}'
            journal = Journal(
                "op-bootstrap-rollback",
                "bootstrap",
                (),
                ("core", "github", "devtools"),
                ("core",),
                ("core", "github", "devtools"),
                InventorySnapshot(durable),
                "rolling-back",
            )
            journal.validate(
                load_component_manifest(),
                lambda value: ("core", "github") if value == durable else (),
            )
            journal.snapshot.restore(state.home)
            write_journal(state.home, journal)
            write_receipt(
                state.home,
                load_component_manifest(),
                journal.receipt("rolled-back", journal.before),
            )
            receipt = run_operation(
                "bootstrap",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id=journal.identifier,
            )
            self.assertIsNone(read_journal(state.home))
        self.assertEqual(receipt["outcome"], "rolled-back")

    def test_rolling_back_bootstrap_rejects_a_durable_snapshot_that_does_not_match_exact_bytes(
        self,
    ) -> None:
        with fixture({"core"}) as state:
            durable = b'{"schema":"getcodexy.installed-component-inventory.v1","components":["core"]}'
            snapshot = b'{"components":["core"],"schema":"getcodexy.installed-component-inventory.v1"}'
            journal = Journal(
                "op-bootstrap-wrong-snapshot",
                "bootstrap",
                (),
                ("core", "github", "devtools"),
                ("core",),
                ("core", "github", "devtools"),
                InventorySnapshot(snapshot),
                "rolling-back",
            )
            target = state.home / "getcodexy"
            target.mkdir(parents=True)
            (target / "installed-components.json").write_bytes(durable)
            write_journal(state.home, journal)
            write_receipt(
                state.home,
                load_component_manifest(),
                journal.receipt("rolled-back", journal.before),
            )
            with self.assertRaisesRegex(ValueError, "restored state"):
                run_operation(
                    "bootstrap",
                    (),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id=journal.identifier,
                )
            self.assertIsNotNone(read_journal(state.home))

    def test_rollback_fails_closed_when_durable_restore_does_not_restore_snapshot(
        self,
    ) -> None:
        with fixture({"core"}, fail_add="codexy-github") as state:
            executed = []

            def corrupt(_: InventorySnapshot, home: object) -> None:
                executed.append(home)
                write_inventory(home, ("core",))

            with (
                patch.object(InventorySnapshot, "restore", corrupt),
                self.assertRaisesRegex(RuntimeError, "durable recovery"),
            ):
                run_operation(
                    "bootstrap",
                    (),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-bootstrap-restore-fault",
                )
            self.assertEqual(executed, [state.home])
            self.assertEqual(read_inventory(state.home), ("core",))
            self.assertIsNotNone(read_journal(state.home))
