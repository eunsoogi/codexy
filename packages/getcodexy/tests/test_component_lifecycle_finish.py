from __future__ import annotations

import unittest

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_transaction_state import (
    read_journal,
    write_inventory,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecycleFinishTests(unittest.TestCase):
    def test_remove_completes_without_reading_retained_hook_activation(self) -> None:
        with fixture({"core", "devtools"}) as state:
            write_inventory(state.home, ("core", "devtools"))

            def unexpected_hook_read(*_: object) -> object:
                raise AssertionError("removal queried hook activation")

            receipt = run_operation(
                "remove",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-remove-untrusted-retained-hook",
                hook_lister=unexpected_hook_read,
            )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(receipt["selection_after"], ["core"])
            self.assertEqual(receipt["errors"], [])
            self.assertIsNone(read_journal(state.home))


if __name__ == "__main__":
    unittest.main()
