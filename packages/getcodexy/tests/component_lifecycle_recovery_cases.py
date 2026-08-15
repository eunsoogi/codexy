"""Lifecycle durable journal recovery cases."""

import json

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import read_journal
from packages.getcodexy.tests.component_lifecycle_records import record, recorded
from packages.getcodexy.tests.component_lifecycle_support import fixture


class ComponentLifecycleRecoveryCases:
    def test_next_operation_recovers_a_durable_interrupted_journal(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core"])
            # The journal persists before an interrupted host mutation that never completed.
            from codexy_runtime_tools.component_transaction_state import (
                InventorySnapshot,
                Journal,
                write_journal,
            )

            write_journal(
                state.home,
                Journal(
                    "op-recover",
                    "install",
                    ("github",),
                    ("core", "github"),
                    ("core",),
                    ("core", "github"),
                    InventorySnapshot.capture(state.home),
                    "started",
                ),
            )
            receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-after-recovery",
            )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(state.selection, {"core", "devtools"})
            self.assertIsNone(read_journal(state.home))

    def test_recovery_commits_a_journal_when_host_readback_reached_target(self) -> None:
        with fixture({"core", "github"}) as state:
            from codexy_runtime_tools.component_transaction_state import (
                InventorySnapshot,
                Journal,
                write_journal,
            )

            write_journal(
                state.home,
                Journal(
                    "op-complete",
                    "install",
                    ("github",),
                    ("core", "github"),
                    (),
                    ("core", "github"),
                    InventorySnapshot.capture(state.home),
                    "started",
                ),
            )
            receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-next",
            )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(recorded(state.home), ["core", "github", "devtools"])
            prior = inventory_path(state.home).parent / "receipts" / "op-complete.json"
            self.assertEqual(
                json.loads(prior.read_text(encoding="utf-8"))["outcome"], "completed"
            )

    def test_corrupt_journal_is_rejected_without_a_host_mutation(self) -> None:
        with fixture({"core"}) as state:
            target = inventory_path(state.home).parent / "inflight.json"
            target.parent.mkdir(parents=True)
            target.write_text(
                '{"schema":"getcodexy.component-transaction.v1"}', encoding="utf-8"
            )
            with self.assertRaisesRegex(ValueError, "journal"):
                run_operation(
                    "install",
                    ("github",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-bad-journal",
                )
            self.assertEqual(state.mutations, [])

    def test_plan_inconsistent_journal_is_rejected_without_a_host_mutation(
        self,
    ) -> None:
        with fixture({"core", "github"}) as state:
            from codexy_runtime_tools.component_transaction_state import (
                InventorySnapshot,
                Journal,
                write_journal,
            )

            record(state.home, ["core", "github"])
            write_journal(
                state.home,
                Journal(
                    "op-bad-plan",
                    "install",
                    ("github",),
                    (),
                    ("core", "github"),
                    (),
                    InventorySnapshot.capture(state.home),
                    "started",
                ),
            )
            with self.assertRaisesRegex(ValueError, "journal"):
                run_operation(
                    "install",
                    ("devtools",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-after-bad",
                )
            self.assertEqual(state.mutations, [])
