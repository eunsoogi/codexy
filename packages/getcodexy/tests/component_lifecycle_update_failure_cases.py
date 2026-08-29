"""Update selection compatibility and rollback cases."""

from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_transition_model import plan_transition
from packages.getcodexy.tests.component_lifecycle_records import record
from packages.getcodexy.tests.component_lifecycle_support import fixture


class ComponentLifecycleUpdateFailureCases:
    def test_empty_update_preserves_the_empty_recorded_selection(self) -> None:
        plan = plan_transition(load_component_manifest(), "update", (), (), ())
        self.assertEqual((plan.resolved, plan.target, plan.adds), ((), (), ()))

        with fixture(set()) as state:
            record(state.home, [])
            receipt = run_operation(
                "update",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-update-empty",
            )
            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(receipt["resolved_components"], [])
            self.assertEqual(receipt["selection_after"], [])
            self.assertEqual(state.selection, set())

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
