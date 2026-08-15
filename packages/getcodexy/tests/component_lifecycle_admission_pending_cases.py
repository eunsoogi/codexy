"""Pending inventory admission case."""

import json
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import (
    InventorySnapshot,
    Journal,
    read_journal,
    write_journal,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecycleAdmissionPendingCases:
    def test_pending_update_admits_inventory_before_recovery_mutation(self) -> None:
        for marketplace_present in (False, True):
            with (
                self.subTest(marketplace_present=marketplace_present),
                fixture(
                    {"core"},
                    marketplace_present=marketplace_present,
                    inventory_override={"installed": [{}]},
                ) as state,
            ):
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
                    "op-pending-update",
                    "update",
                    (),
                    ("core",),
                    ("core",),
                    ("core",),
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
                    operation_id=f"op-after-pending-{marketplace_present}",
                )

                self.assertEqual(
                    receipt["errors"], [{"code": "invalid-installed-inventory"}]
                )
                self.assertEqual(state.mutations, [])
                self.assertEqual(read_journal(state.home), journal)
