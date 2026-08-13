from __future__ import annotations

import unittest
import json

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_transaction_receipts import write_receipt
from codexy_runtime_tools.component_transaction_state import read_journal
from codexy_runtime_tools.component_transition_model import OperationReceipt
from codexy_runtime_tools.component_transaction_state import InventorySnapshot, Journal, write_journal
from packages.getcodexy.tests.component_lifecycle_support import fixture


class BootstrapTests(unittest.TestCase):
    def test_typed_idempotent_recovery_from_absent_record(self) -> None:
        with fixture({"core"}) as state:
            receipt = run_operation("bootstrap", (), state.home, state.codex, state.run, operation_id="op-bootstrap")
            repeated = run_operation("bootstrap", (), state.home, state.codex, state.run, operation_id="op-bootstrap-repeat")
        self.assertEqual(receipt["command"], "bootstrap")
        self.assertEqual(receipt["selection_before"], ["core"])
        self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
        self.assertEqual(repeated["selection_after"], ["core", "github", "devtools"])

    def test_operands_have_a_typed_rejection(self) -> None:
        with fixture() as state:
            receipt = run_operation("bootstrap", ("core",), state.home, state.codex, state.run, operation_id="op-bootstrap-operand")
        self.assertEqual(receipt["errors"], [{"code": "components-not-accepted"}])
        OperationReceipt.decode(receipt).validate(load_component_manifest())

    def test_recovery_preserves_distinct_live_and_durable_prestate(self) -> None:
        with fixture({"core"}) as state:
            (state.home / "getcodexy").mkdir(parents=True)
            (state.home / "getcodexy" / "installed-components.json").write_text('{"schema":"getcodexy.installed-component-inventory.v1","components":["core","github"]}')
            receipt = run_operation("bootstrap", (), state.home, state.codex, state.run, operation_id="op-bootstrap-mismatch")
        self.assertEqual(receipt["selection_before"], ["core"])
        self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])

    def test_corrupt_bootstrap_durable_snapshot_fails_closed(self) -> None:
        with fixture({"core"}) as state:
            invalid = json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["unknown"]}).encode()
            write_journal(state.home, Journal("op-bootstrap-corrupt", "bootstrap", (), ("core", "github", "devtools"), ("core",), ("core", "github", "devtools"), InventorySnapshot(invalid), "started"))
            with self.assertRaisesRegex(ValueError, "durable inventory"):
                run_operation("bootstrap", (), state.home, state.codex, state.run, operation_id="op-bootstrap-corrupt")
            self.assertEqual(state.mutations, [])

    def test_rolling_back_bootstrap_restores_distinct_live_and_durable_prestates(self) -> None:
        with fixture({"core"}) as state:
            durable = b'{"schema":"getcodexy.installed-component-inventory.v1","components":["core","github"]}'
            journal = Journal("op-bootstrap-rollback", "bootstrap", (), ("core", "github", "devtools"), ("core",), ("core", "github", "devtools"), InventorySnapshot(durable), "rolling-back")
            journal.validate(load_component_manifest(), lambda value: ("core", "github") if value == durable else ())
            journal.snapshot.restore(state.home)
            write_journal(state.home, journal)
            write_receipt(state.home, load_component_manifest(), journal.receipt("rolled-back", journal.before))

            receipt = run_operation("bootstrap", (), state.home, state.codex, state.run, operation_id=journal.identifier)
            self.assertIsNone(read_journal(state.home))

        self.assertEqual(receipt["outcome"], "rolled-back")

    def test_rolling_back_bootstrap_rejects_a_durable_snapshot_that_does_not_match_exact_bytes(self) -> None:
        with fixture({"core"}) as state:
            durable = b'{"schema":"getcodexy.installed-component-inventory.v1","components":["core"]}'
            snapshot = b'{"components":["core"],"schema":"getcodexy.installed-component-inventory.v1"}'
            journal = Journal("op-bootstrap-wrong-snapshot", "bootstrap", (), ("core", "github", "devtools"), ("core",), ("core", "github", "devtools"), InventorySnapshot(snapshot), "rolling-back")
            target = state.home / "getcodexy"
            target.mkdir(parents=True)
            (target / "installed-components.json").write_bytes(durable)
            write_journal(state.home, journal)
            write_receipt(state.home, load_component_manifest(), journal.receipt("rolled-back", journal.before))

            with self.assertRaisesRegex(ValueError, "restored state"):
                run_operation("bootstrap", (), state.home, state.codex, state.run, operation_id=journal.identifier)
            self.assertIsNotNone(read_journal(state.home))


if __name__ == "__main__":
    unittest.main()
