from __future__ import annotations

import json
import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_resolver import classify_installed_inventory, preflight_unregistered_inventory
from codexy_runtime_tools.component_transaction_state import InventorySnapshot, Journal, read_journal, write_journal
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecycleAdmissionTests(unittest.TestCase):
    def test_identityless_records_reject_before_new_operation_mutation_on_both_paths(self) -> None:
        records = (
            {},
            {"name": "unrelated"},
            {"pluginId": "unrelated@other"},
            {"marketplaceName": "other"},
            {"name": "alpha", "pluginId": "beta@other", "marketplaceName": "other"},
            {"name": "alpha", "pluginId": "alpha@other", "marketplaceName": "different"},
            {"name": "", "pluginId": "@other", "marketplaceName": "other"},
        )
        for marketplace_present in (False, True):
            for record in records:
                with self.subTest(marketplace_present=marketplace_present, record=record), fixture(marketplace_present=marketplace_present, inventory_override={"installed": [record]}) as state:
                    receipt = run_operation("install", ("core",), state.home, state.codex, state.run, operation_id="op-identityless")
                    self.assertEqual(receipt["errors"], [{"code": "invalid-installed-inventory"}])
                    self.assertEqual(state.marketplace_present, marketplace_present)
                self.assertEqual(state.mutations, [])
                self.assertIsNone(read_journal(state.home))

    def test_complete_consistent_non_codexy_identity_is_the_only_ignored_record(self) -> None:
        record = {"name": "alpha", "pluginId": "alpha@other", "marketplaceName": "other"}
        self.assertIsNone(preflight_unregistered_inventory(classify_installed_inventory(load_component_manifest(), {"installed": [record]})))

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

    def test_semantically_malformed_replay_receipts_reject_before_host_or_journal_mutation(self) -> None:
        with fixture() as state:
            run_operation("install", ("core",), state.home, state.codex, state.run, operation_id="op-semantic-receipt")
            receipt = inventory_path(state.home).parent / "receipts" / "op-semantic-receipt.json"
            original = json.loads(receipt.read_text())
            for mutation in (
                lambda value: value.update({"unknown": True}),
                lambda value: value.update({"selection_after": [], "installed_components": []}),
                lambda value: value.update({"errors": [{"code": "operation-failed", "detail": "unexpected"}]}),
            ):
                with self.subTest(mutation=mutation):
                    payload = dict(original)
                    mutation(payload)
                    receipt.write_text(json.dumps(payload, sort_keys=True), encoding="utf-8")
                    with self.assertRaisesRegex(ValueError, "operation receipt"):
                        run_operation("install", ("core",), state.home, state.codex, state.run, operation_id="op-semantic-receipt")
                    self.assertEqual(state.mutations, [("plugin", "add", "codexy@codexy", "--json")])
                    self.assertIsNone(read_journal(state.home))
            receipt.write_text(json.dumps(original, sort_keys=True), encoding="utf-8")

    def test_pending_journal_receipt_is_admitted_before_recovery_for_a_distinct_caller(self) -> None:
        with fixture({"core"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core"]}), encoding="utf-8")
            journal = Journal("op-pending-receipt", "update", (), ("core",), ("core",), ("core",), InventorySnapshot.capture(state.home), "started")
            write_journal(state.home, journal)
            receipt = target.parent / "receipts" / "op-pending-receipt.json"
            receipt.parent.mkdir(parents=True)
            receipt.write_text(json.dumps(_receipt("op-pending-receipt", "update", (), ("core",), ("core",), ("core",), "completed"), sort_keys=True), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "pending transaction receipt"):
                run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-distinct-caller")
            self.assertEqual(state.mutations, [])
            self.assertEqual(read_journal(state.home), journal)

    def test_pending_receipt_table_handles_malformed_and_exact_rollback_recovery(self) -> None:
        with fixture({"core"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core"]}), encoding="utf-8")
            journal = Journal("op-pending-rollback", "update", (), ("core",), ("core",), ("core",), InventorySnapshot.capture(state.home), "rolling-back")
            write_journal(state.home, journal)
            receipt = target.parent / "receipts" / "op-pending-rollback.json"
            receipt.parent.mkdir(parents=True)
            payload = _receipt("op-pending-rollback", "update", (), ("core",), ("core",), ("core",), "rolled-back", [{"code": "operation-failed"}])
            receipt.write_text(json.dumps(payload, sort_keys=True), encoding="utf-8")

            with patch("codexy_runtime_tools.component_lifecycle.write_journal") as journal_write:
                replay = run_operation("update", (), state.home, state.codex, state.run, operation_id="op-pending-rollback")
            self.assertEqual(replay, payload)
            journal_write.assert_not_called()
            self.assertEqual(state.mutations, [])
            self.assertIsNone(read_journal(state.home))

        with fixture({"core"}) as state:
            target = inventory_path(state.home)
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps({"schema": "getcodexy.installed-component-inventory.v1", "components": ["core"]}), encoding="utf-8")
            journal = Journal("op-pending-malformed", "update", (), ("core",), ("core",), ("core",), InventorySnapshot.capture(state.home), "started")
            write_journal(state.home, journal)
            receipt = target.parent / "receipts" / "op-pending-malformed.json"
            receipt.parent.mkdir(parents=True)
            payload = _receipt("op-pending-malformed", "update", (), ("core",), ("core",), ("core",), "completed")
            payload["unknown"] = True
            receipt.write_text(json.dumps(payload, sort_keys=True), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "operation receipt"):
                run_operation("install", ("devtools",), state.home, state.codex, state.run, operation_id="op-different-after-malformed")
            self.assertEqual(state.mutations, [])
            self.assertEqual(read_journal(state.home), journal)


def _receipt(identifier: str, command: str, requested: tuple[str, ...], resolved: tuple[str, ...], before: tuple[str, ...], after: tuple[str, ...], outcome: str, errors: list[dict[str, str]] | None = None) -> dict[str, object]:
    return {
        "schema": "getcodexy.operation-receipt.v1",
        "operation_id": identifier,
        "command": command,
        "outcome": outcome,
        "requested_components": list(requested),
        "resolved_components": list(resolved),
        "selection_before": list(before),
        "selection_after": list(after),
        "installed_components": list(after),
        "source_of_truth": "installed-component-inventory",
        "errors": [] if errors is None else errors,
    }


if __name__ == "__main__":
    unittest.main()
