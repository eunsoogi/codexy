from __future__ import annotations

import base64
import json
import unittest

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_resolver import ComponentResolutionError
from codexy_runtime_tools.component_transaction_state import (
    InventorySnapshot,
    Journal,
    inventory_path,
    write_journal,
)
from codexy_runtime_tools.component_transition_model import (
    OperationReceipt,
    plan_transition,
)
from codexy_runtime_tools.component_transition_rejections import (
    Rejection,
    RejectionStage,
    StateFailure,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture


class JournalValidationTests(unittest.TestCase):
    def test_transition_model_owns_plan_journal_and_receipt_contracts(self) -> None:
        manifest = load_component_manifest()
        plan = plan_transition(manifest, "install", ("github",), ("core",), ("core",))
        journal = plan.journal("op-model", InventorySnapshot(None))

        decoded = Journal.decode(journal.encode())
        decoded.validate(manifest, lambda _: ("core",))
        receipt = OperationReceipt.decode(decoded.receipt("completed").encode())

        receipt.validate(manifest)
        self.assertEqual(receipt.resolved, ("core", "github"))
        self.assertEqual(receipt.after, ("core", "github"))

    def test_transition_model_derives_rejections_from_plan_or_state_failures(
        self,
    ) -> None:
        manifest = load_component_manifest()
        with self.assertRaises(ComponentResolutionError) as planned:
            plan_transition(manifest, "update", (), ("core",), None)

        plan_rejection = OperationReceipt.rejected(
            "op-plan-rejection",
            "update",
            (),
            ("core",),
            Rejection.from_failure(RejectionStage.PLAN, planned.exception),
        )
        state_rejection = OperationReceipt.rejected(
            "op-state-rejection",
            "install",
            ("core",),
            (),
            Rejection.from_failure(
                RejectionStage.PRESTATE, StateFailure.INCONSISTENT_INSTALLED_STATE
            ),
        )

        self.assertEqual(plan_rejection.errors, ("no-recorded-selection",))
        self.assertEqual(state_rejection.errors, ("inconsistent-installed-state",))

    def test_rejected_receipts_must_match_a_reachable_prestate_transition(self) -> None:
        manifest = load_component_manifest()
        valid = OperationReceipt.rejected(
            "op-protected-removal",
            "remove",
            ("core",),
            ("core", "devtools"),
            Rejection.from_failure(
                RejectionStage.PLAN,
                ComponentResolutionError("dependency-protected-removal"),
            ),
        )
        impossible = OperationReceipt.rejected(
            "op-impossible-removal",
            "remove",
            ("core",),
            (),
            Rejection.from_failure(
                RejectionStage.PLAN,
                ComponentResolutionError("dependency-protected-removal"),
            ),
        )

        valid.validate(manifest)
        with self.assertRaisesRegex(ValueError, "rejection semantics"):
            impossible.validate(manifest)

    def test_transition_constructors_refuse_unreachable_journals_and_receipts(
        self,
    ) -> None:
        manifest = load_component_manifest()
        update = plan_transition(manifest, "update", (), ("core",), ("core",))
        install = plan_transition(
            manifest, "install", ("github",), ("core",), ("core",)
        )

        with self.assertRaisesRegex(ValueError, "snapshot"):
            update.journal("op-update-without-snapshot", InventorySnapshot(None))
        journal = install.journal("op-invalid-terminal", InventorySnapshot(None))
        with self.assertRaisesRegex(ValueError, "completion"):
            journal.receipt("completed", journal.before)
        with self.assertRaisesRegex(ValueError, "rollback"):
            journal.receipt("rolled-back", journal.target)
        with self.assertRaisesRegex(ValueError, "journal"):
            journal.receipt("pending")  # type: ignore[arg-type]

    def test_update_journal_without_an_inventory_snapshot_is_rejected_before_host_mutation(
        self,
    ) -> None:
        with fixture({"core"}) as state:
            write_journal(
                state.home,
                Journal(
                    "op-update-without-snapshot",
                    "update",
                    (),
                    ("core",),
                    ("core",),
                    ("core",),
                    InventorySnapshot(None),
                    "started",
                ),
            )

            with self.assertRaisesRegex(ValueError, "journal"):
                run_operation(
                    "install",
                    ("github",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-next",
                )

            self.assertEqual(state.mutations, [])

    def test_duplicate_key_inventory_snapshot_is_rejected_before_host_mutation(
        self,
    ) -> None:
        with fixture({"core"}) as state:
            target = inventory_path(state.home).parent / "inflight.json"
            target.parent.mkdir(parents=True)
            snapshot = base64.b64encode(
                b'{"schema":"bad","schema":"getcodexy.installed-component-inventory.v1","components":["core"]}'
            ).decode()
            target.write_text(
                json.dumps(
                    {
                        "schema": "getcodexy.component-transaction.v1",
                        "operation_id": "op-duplicate",
                        "command": "install",
                        "requested": ["github"],
                        "resolved": ["core", "github"],
                        "before": ["core"],
                        "target": ["core", "github"],
                        "inventory": snapshot,
                        "phase": "started",
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "duplicate keys"):
                run_operation(
                    "install",
                    ("github",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-next",
                )
            self.assertEqual(state.mutations, [])


if __name__ == "__main__":
    unittest.main()
