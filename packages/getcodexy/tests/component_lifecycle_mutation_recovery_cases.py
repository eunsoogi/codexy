"""Mutation rollback retry cases."""

import json

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import read_journal
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import fixture


class ComponentLifecycleMutationRecoveryCases:
    def test_failed_rollback_remains_a_rollback_on_next_recovery(self) -> None:
        with fixture(
            {"core"}, fail_add="codexy-github", fail_remove="codexy-github"
        ) as state:
            record(state.home, ["core"])
            with self.assertRaisesRegex(RuntimeError, "durable recovery"):
                run_operation(
                    "install",
                    ("github",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-retry-rollback",
                )
            self.assertEqual(read_journal(state.home).phase, "rolling-back")
            self.assertEqual(state.selection, {"core", "github"})
            state.fail_remove = None
            receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-after-retry",
            )
            self.assertEqual(
                json.loads(
                    (
                        inventory_path(state.home).parent
                        / "receipts"
                        / "op-retry-rollback.json"
                    ).read_text()
                )["outcome"],
                "rolled-back",
            )
            self.assertEqual(receipt["selection_after"], ["core", "devtools"])
