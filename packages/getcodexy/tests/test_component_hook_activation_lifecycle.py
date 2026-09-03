from __future__ import annotations

import json
import unittest

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_transaction_state import write_inventory
from packages.getcodexy.tests.component_lifecycle_support import fixture


class ComponentHookActivationLifecycleTests(unittest.TestCase):
    def test_upgrade_records_pending_hook_activation_without_rewriting_host_trust(
        self,
    ) -> None:
        with fixture({"core"}, versions={"core": "1.2.0"}) as state:
            write_inventory(state.home, ("core",))
            before_mutations = tuple(state.mutations)
            receipt = run_operation(
                "update",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-update-pending-hooks",
                hook_lister=lambda _executable, _home: [],
            )

            self.assertEqual(receipt["outcome"], "pending-action")
            self.assertEqual(
                receipt["errors"], [{"code": "required-hook-trust-missing"}]
            )
            self.assertEqual(receipt["selection_before"], ["core"])
            self.assertEqual(receipt["selection_after"], ["core"])
            self.assertEqual(state.selection, {"core"})
            self.assertEqual(
                tuple(state.mutations[: len(before_mutations)]), before_mutations
            )
            self.assertEqual(
                json.loads(
                    (
                        state.home / "getcodexy/receipts/op-update-pending-hooks.json"
                    ).read_text(encoding="utf-8")
                ),
                receipt,
            )


if __name__ == "__main__":
    unittest.main()
