"""Mutation update rollback case."""

from codexy_runtime_tools.component_lifecycle import run_operation
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import fixture


class ComponentLifecycleUpdateFailureCases:
    def test_update_failure_restores_its_exact_selection(self) -> None:
        with fixture({"core", "github"}, fail_add="codexy-github") as state:
            record(state.home, ["core", "github"])
            receipt = run_operation(
                "update",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-update-fail",
            )
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core", "github"})
