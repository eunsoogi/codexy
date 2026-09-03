from __future__ import annotations

import json
import unittest

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import read_journal
from codexy_runtime_tools.monolith_migration_state import (
    MigrationJournal,
    read_journal as read_migration_journal,
    write_journal as write_migration_journal,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture, installed


class LifecyclePreflightTests(unittest.TestCase):
    def test_pending_monolith_migration_rejects_lifecycle_mutation(self) -> None:
        with fixture() as state:
            write_migration_journal(
                state.home,
                MigrationJournal.capture(state.home, "1.3.0", "1.4.0", ("core",)),
            )

            receipt = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-pending-monolith-migration",
            )

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(
                receipt["errors"], [{"code": "inconsistent-installed-state"}]
            )
            self.assertEqual(state.mutations, [])
            self.assertIsNone(read_journal(state.home))
            self.assertIsNotNone(read_migration_journal(state.home))

    def test_absent_marketplace_does_not_bootstrap_for_a_rejected_request(self) -> None:
        with fixture(marketplace_present=False) as state:
            receipt = run_operation(
                "install",
                ("unknown",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-unknown-no-market",
            )
            self.assertEqual(receipt["outcome"], "rejected")
            self.assertFalse(state.marketplace_present)
            self.assertEqual(state.mutations, [])

    def test_absent_marketplace_does_not_bootstrap_for_a_corrupt_journal(self) -> None:
        with fixture(marketplace_present=False) as state:
            target = inventory_path(state.home).parent / "inflight.json"
            target.parent.mkdir(parents=True)
            target.write_text(
                '{"schema":"getcodexy.component-transaction.v1"}', encoding="utf-8"
            )
            with self.assertRaisesRegex(ValueError, "journal"):
                run_operation(
                    "install",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-bad-no-market",
                )
            self.assertFalse(state.marketplace_present)
            self.assertEqual(state.mutations, [])

    def test_older_lockstep_update_failure_preserves_the_prior_version_and_receipt(
        self,
    ) -> None:
        with fixture(
            {"core"}, fail_marketplace_add=True, versions={"core": "1.2.0"}
        ) as state:
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
            receipt = run_operation(
                "update",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-old-upgrade-fail",
            )
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core"})
            self.assertEqual(state.versions, {"core": "1.2.0"})
            self.assertEqual(json.loads(target.read_text())["components"], ["core"])
            saved = target.parent / "receipts" / "op-old-upgrade-fail.json"
            self.assertEqual(json.loads(saved.read_text())["outcome"], "rolled-back")

    def test_absent_marketplace_bootstraps_only_after_a_proved_empty_inventory(
        self,
    ) -> None:
        with fixture(marketplace_present=False) as state:
            receipt = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-clean-no-market",
            )
            self.assertEqual(receipt["outcome"], "completed")
            self.assertTrue(state.marketplace_present)

    def test_absent_marketplace_ignores_an_unrelated_prefix_record(self) -> None:
        unrelated = {
            "installed": [
                {
                    "name": "codexylophone",
                    "pluginId": "codexylophone@other",
                    "marketplaceName": "other",
                }
            ]
        }
        with fixture(
            marketplace_present=False, inventory_responses=[unrelated]
        ) as state:
            receipt = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-unrelated-no-market",
            )
            self.assertEqual(receipt["outcome"], "completed")
            self.assertTrue(state.marketplace_present)

    def test_both_marketplace_paths_reject_invalid_identity_records_before_mutation(
        self,
    ) -> None:
        cases = (
            ("conflict", "conflicting-installed-state", "conflicting-installed-state"),
            ("orphan", "conflicting-installed-state", None),
            ("unknown", "unknown-installed-component", "unknown-installed-component"),
            (
                "missing-name",
                "conflicting-installed-state",
                "conflicting-installed-state",
            ),
            ("malformed", "conflicting-installed-state", "conflicting-installed-state"),
            ("duplicate", "conflicting-installed-state", "conflicting-installed-state"),
        )
        for marketplace_present in (False, True):
            for case, unregistered_error, registered_error in cases:
                with (
                    self.subTest(marketplace_present=marketplace_present, case=case),
                    fixture(marketplace_present=marketplace_present) as state,
                ):
                    record = (
                        installed(state.marketplace, "core")
                        if case != "unknown"
                        else {
                            "name": "codexy-future",
                            "pluginId": "codexy-future@codexy",
                            "marketplaceName": "codexy",
                        }
                    )
                    if case == "conflict":
                        record["marketplaceName"] = "other-marketplace"
                    if case == "missing-name":
                        record["pluginId"] = "codexy@other-marketplace"
                        record.pop("name")
                    if case == "malformed":
                        record["pluginId"] = "malformed"
                    records = [record]
                    if case == "duplicate":
                        records.append(record.copy())
                    state.inventory_override = {"installed": records}
                    receipt = run_operation(
                        "install",
                        ("core",),
                        state.home,
                        state.codex,
                        state.run,
                        operation_id=f"op-{marketplace_present}-{case}",
                    )
                    error = (
                        registered_error if marketplace_present else unregistered_error
                    )
                    self.assertEqual(
                        receipt["errors"], [] if error is None else [{"code": error}]
                    )
                    self.assertEqual(state.marketplace_present, marketplace_present)
                    if error is not None:
                        self.assertEqual(state.mutations, [])
                        self.assertIsNone(read_journal(state.home))


if __name__ == "__main__":
    unittest.main()
