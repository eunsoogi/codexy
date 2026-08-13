from __future__ import annotations

import json
import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import InventorySnapshot, Journal, read_journal, write_journal
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecycleAdmissionTests(unittest.TestCase):
    def test_identityless_records_reject_before_new_operation_mutation_on_both_paths(self) -> None:
        records = ({}, {"name": "unrelated"}, {"pluginId": "unrelated@other"}, {"marketplaceName": "other"})
        for marketplace_present in (False, True):
            for record in records:
                with self.subTest(marketplace_present=marketplace_present, record=record), fixture(marketplace_present=marketplace_present, inventory_override={"installed": [record]}) as state:
                    receipt = run_operation("install", ("core",), state.home, state.codex, state.run, operation_id="op-identityless")
                    self.assertEqual(receipt["errors"], [{"code": "invalid-installed-inventory"}])
                    self.assertEqual(state.marketplace_present, marketplace_present)
                    self.assertEqual(state.mutations, [])
                    self.assertIsNone(read_journal(state.home))

    def test_pending_update_admits_inventory_before_recovery_mutation(self) -> None:
        for marketplace_present in (False, True):
            with self.subTest(marketplace_present=marketplace_present), fixture({"core"}, marketplace_present=marketplace_present, inventory_override={"installed": [{}]}) as state:
                target = inventory_path(state.home)
                target.parent.mkdir(parents=True)
                target.write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core"]}), encoding="utf-8")
                journal = Journal("op-pending-update", "update", (), ("core",), ("core",), ("core",), InventorySnapshot.capture(state.home), "started")
                write_journal(state.home, journal)

                receipt = run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id=f"op-after-pending-{marketplace_present}")

                self.assertEqual(receipt["errors"], [{"code": "invalid-installed-inventory"}])
                self.assertEqual(state.mutations, [])
                self.assertEqual(read_journal(state.home), journal)

    def test_same_operation_id_replays_without_a_new_journal(self) -> None:
        with fixture() as state:
            original = run_operation("install", ("core",), state.home, state.codex, state.run, operation_id="op-replay")
            with patch("codexy_runtime_tools.component_lifecycle.write_journal") as journal:
                replay = run_operation("install", ("core",), state.home, state.codex, state.run, operation_id="op-replay")
            self.assertEqual(replay, original)
            journal.assert_not_called()
            self.assertEqual(state.mutations, [("plugin", "add", "codexy@codexy", "--json")])

    def test_conflicting_operation_id_rejects_before_host_or_journal_mutation(self) -> None:
        with fixture() as state:
            run_operation("install", ("core",), state.home, state.codex, state.run, operation_id="op-conflict")
            with self.assertRaisesRegex(ValueError, "operation receipt conflicts"):
                run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-conflict")
            self.assertEqual(state.mutations, [("plugin", "add", "codexy@codexy", "--json")])
            self.assertIsNone(read_journal(state.home))


if __name__ == "__main__":
    unittest.main()
