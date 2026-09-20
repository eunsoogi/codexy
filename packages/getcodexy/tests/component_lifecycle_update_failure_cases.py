"""Update selection compatibility and rollback cases."""

import os
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path
from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_transition_model import plan_transition
from codexy_runtime_tools.updater import compare_managed_files
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


class ComponentLifecycleRegistrationCases:
    def test_install_synchronizes_catalog_roles_and_preserves_unmanaged_files(
        self,
    ) -> None:
        with fixture(real_registration=True) as state:
            unmanaged = state.home / "agents" / "personal.toml"
            unmanaged.parent.mkdir(parents=True)
            unmanaged.write_bytes(b"personal = true\n")
            first = run_operation(
                "install",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-register",
            )
            roles_root = state.home / "agents" / "codexy"
            before = {
                path.name: path.read_bytes() for path in roles_root.glob("*.toml")
            }
            report = compare_managed_files(
                state.marketplace / "plugins" / "codexy", state.home, "core"
            )
            second = run_operation(
                "install",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-register-repeat",
            )
            self.assertEqual(first["outcome"], second["outcome"])
            self.assertEqual(first["outcome"], "completed")
            self.assertEqual(report["state"], "exact")
            self.assertEqual(report["exact_count"], report["expected_count"])
            self.assertEqual(
                before,
                {path.name: path.read_bytes() for path in roles_root.glob("*.toml")},
            )
            self.assertEqual(unmanaged.read_bytes(), b"personal = true\n")

    def test_registration_failure_rolls_back_roles_inventory_and_allows_retry(
        self,
    ) -> None:
        with fixture(real_registration=True) as state:
            unmanaged = state.home / "agents" / "personal.toml"
            unmanaged.parent.mkdir(parents=True)
            unmanaged.write_bytes(b"personal = true\n")
            with patch.dict(os.environ, {"CODEXY_AGENT_REGISTRATION_FAIL_AFTER": "1"}):
                failed = run_operation(
                    "install",
                    (),
                    state.home,
                    state.codex,
                    state.run,
                    operation_id="op-register-failure",
                )
            self.assertEqual(failed["outcome"], "rolled-back")
            self.assertFalse(inventory_path(state.home).exists())
            self.assertFalse((state.home / "agents" / "codexy").exists())
            self.assertEqual(unmanaged.read_bytes(), b"personal = true\n")
            retried = run_operation(
                "install",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-register-retry",
            )
            report = compare_managed_files(
                state.marketplace / "plugins" / "codexy", state.home, "core"
            )
            self.assertEqual(retried["outcome"], "completed")
            self.assertEqual(report["state"], "exact")

    def test_update_and_bootstrap_synchronize_roles_before_completion(self) -> None:
        with fixture({"core"}, real_registration=True) as state:
            record(state.home, ["core"])
            receipt = run_operation(
                "update",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-update-register",
            )
            report = compare_managed_files(
                state.marketplace / "plugins" / "codexy", state.home, "core"
            )
            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(report["state"], "exact")
        with fixture(real_registration=True) as state:
            receipt = run_operation(
                "bootstrap",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-bootstrap-register",
            )
            report = compare_managed_files(
                state.marketplace / "plugins" / "codexy", state.home, "core"
            )
            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(report["state"], "exact")
