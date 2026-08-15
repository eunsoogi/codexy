"""Receipt replay admission cases."""

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


class LifecycleAdmissionReplayCases:
    def test_same_operation_id_replays_without_a_new_journal(self) -> None:
        with fixture() as state:
            original = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-replay",
            )
            with patch(
                "codexy_runtime_tools.component_transaction_state.write_journal"
            ) as journal:
                replay = run_operation(
                    "install",
                    ("core",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-replay",
                )
            self.assertEqual(replay, original)
            journal.assert_not_called()
            self.assertEqual(
                state.mutations, [("plugin", "add", "codexy@codexy", "--json")]
            )

    def test_conflicting_operation_id_rejects_before_host_or_journal_mutation(
        self,
    ) -> None:
        with fixture() as state:
            run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-conflict",
            )
            with self.assertRaisesRegex(ValueError, "operation receipt conflicts"):
                run_operation(
                    "install",
                    ("devtools",),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-conflict",
                )
            self.assertEqual(
                state.mutations, [("plugin", "add", "codexy@codexy", "--json")]
            )
            self.assertIsNone(read_journal(state.home))

    def test_semantically_malformed_replayreceipts_reject_before_host_or_journal_mutation(
        self,
    ) -> None:
        with fixture() as state:
            run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-semantic-receipt",
            )
            receipt = (
                inventory_path(state.home).parent
                / "receipts"
                / "op-semantic-receipt.json"
            )
            original = json.loads(receipt.read_text())
            for mutation in (
                lambda value: value.update({"unknown": True}),
                lambda value: value.update(
                    {"schema": "getcodexy.operation-receipt.v0"}
                ),
                lambda value: value.update(
                    {"selection_after": [], "installed_components": []}
                ),
                lambda value: value.update(
                    {"errors": [{"code": "operation-failed", "detail": "unexpected"}]}
                ),
                lambda value: value.update(
                    {
                        "outcome": "rejected",
                        "resolved_components": [],
                        "errors": [{"code": "missing-removal-target"}],
                    }
                ),
            ):
                with self.subTest(mutation=mutation):
                    payload = dict(original)
                    mutation(payload)
                    receipt.write_text(
                        json.dumps(payload, sort_keys=True), encoding="utf-8"
                    )
                    with self.assertRaisesRegex(ValueError, "operation receipt"):
                        run_operation(
                            "install",
                            ("core",),
                            state.home,
                            state.codex,
                            state.run,
                            operation_id="op-semantic-receipt",
                        )
                    self.assertEqual(
                        state.mutations, [("plugin", "add", "codexy@codexy", "--json")]
                    )
                    self.assertIsNone(read_journal(state.home))
            receipt.write_text(json.dumps(original, sort_keys=True), encoding="utf-8")
