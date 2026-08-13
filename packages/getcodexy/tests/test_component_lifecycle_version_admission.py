from __future__ import annotations

import unittest

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_transaction_state import read_journal, write_inventory
from packages.getcodexy.tests.component_lifecycle_support import fixture


class VersionAdmissionTests(unittest.TestCase):
    def test_coherent_older_component_version_can_update(self) -> None:
        with fixture({"core"}, versions={"core": "1.2.0"}) as state:
            write_inventory(state.home, ("core",))
            receipt = run_operation("update", (), state.home, state.codex, state.run, operation_id="op-old")

            self.assertEqual(receipt["outcome"], "completed")
            self.assertIn(("plugin", "marketplace", "upgrade", "codexy", "--json"), state.calls)

    def test_older_coherent_selection_rejects_install_and_remove_before_mutation(self) -> None:
        for command, requested in (("install", ("github",)), ("remove", ("core",))):
            with self.subTest(command=command), fixture({"core"}, versions={"core": "1.2.0"}) as state:
                write_inventory(state.home, ("core",))
                receipt = run_operation(command, requested, state.home, state.codex, state.run, operation_id=f"op-old-{command}")

                self.assertEqual(receipt["outcome"], "rejected")
                self.assertEqual(receipt["errors"], [{"code": "component-version-mismatch"}])
                self.assertEqual(state.selection, {"core"})
                self.assertEqual(state.mutations, [])
                self.assertIsNone(read_journal(state.home))


if __name__ == "__main__":
    unittest.main()
