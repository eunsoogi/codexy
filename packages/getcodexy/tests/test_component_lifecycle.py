from __future__ import annotations

import json
import errno
import os
import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_transaction_state import read_journal
from codexy_runtime_tools.version_lock import default_package_version
from packages.getcodexy.tests.component_lifecycle_update_failure_cases import (
    ComponentLifecycleUpdateFailureCases,
)
from packages.getcodexy.tests.component_lifecycle_mutation_recovery_cases import (
    ComponentLifecycleMutationRecoveryCases,
)
from packages.getcodexy.tests.component_lifecycle_preflight_cases import (
    ComponentLifecyclePreflightCases,
)
from packages.getcodexy.tests.component_lifecycle_recovery_cases import (
    ComponentLifecycleRecoveryCases,
)
from packages.getcodexy.tests.component_lifecycle_records import record, recorded
from packages.getcodexy.tests.component_lifecycle_support import (
    exact_marketplace_add,
    fixture,
)


class ComponentLifecycleTests(
    ComponentLifecycleUpdateFailureCases,
    ComponentLifecycleMutationRecoveryCases,
    ComponentLifecyclePreflightCases,
    ComponentLifecycleRecoveryCases,
    unittest.TestCase,
):
    def test_bare_install_records_and_reads_back_all_components(self) -> None:
        with fixture() as state:
            receipt = run_operation(
                "install",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-install",
            )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(receipt["selection_before"], [])
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
            self.assertEqual(
                receipt["installed_components"], ["core", "github", "devtools"]
            )
            self.assertEqual(state.selection, {"core", "github", "devtools"})
            self.assertEqual(recorded(state.home), ["core", "github", "devtools"])

    def test_bare_install_bootstraps_the_official_marketplace(self) -> None:
        with fixture(marketplace_present=False) as state:
            receipt = run_operation(
                "install",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-bootstrap-market",
            )
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
            self.assertTrue(state.marketplace_present)
            self.assertIn(
                exact_marketplace_add(default_package_version()),
                state.mutations,
            )

    def test_explicit_update_preserves_the_selection(self) -> None:
        with fixture({"core", "github", "devtools"}) as state:
            record(state.home, ["core", "github", "devtools"])
            receipt = run_operation(
                "update",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-update",
            )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(receipt["resolved_components"], ["core", "github"])
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
            self.assertEqual(state.selection, {"core", "github", "devtools"})
            self.assertIn(
                exact_marketplace_add(),
                state.calls,
            )
            self.assertIn(("plugin", "add", "codexy@codexy", "--json"), state.calls)

    def test_selective_install_closes_dependencies_and_keeps_existing_components(
        self,
    ) -> None:
        with fixture({"core", "devtools"}) as state:
            record(state.home, ["core", "devtools"])
            receipt = run_operation(
                "install",
                ("github",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-selective",
            )

            self.assertEqual(receipt["resolved_components"], ["core", "github"])
            self.assertEqual(receipt["selection_after"], ["core", "github", "devtools"])
            self.assertEqual(state.selection, {"core", "github", "devtools"})

    def test_remove_rejects_a_dependency_protected_component_before_mutation(
        self,
    ) -> None:
        with fixture({"core", "github"}) as state:
            record(state.home, ["core", "github"])
            receipt = run_operation(
                "remove",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-guard",
            )

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(
                receipt["errors"], [{"code": "dependency-protected-removal"}]
            )
            self.assertEqual(state.selection, {"core", "github"})
            self.assertEqual(state.mutations, [])

    def test_failed_mutation_restores_selection_and_record(self) -> None:
        with fixture({"core", "github"}, fail_add="codexy-devtools") as state:
            record(state.home, ["core", "github"])
            receipt = run_operation(
                "install",
                ("devtools",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-failure",
            )

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(receipt["selection_after"], ["core", "github"])
            self.assertEqual(state.selection, {"core", "github"})
            self.assertEqual(recorded(state.home), ["core", "github"])
            stored = inventory_path(state.home).parent / "receipts" / "op-failure.json"
            self.assertEqual(
                json.loads(stored.read_text(encoding="utf-8"))["outcome"], "rolled-back"
            )

    def test_partial_remove_failure_restores_its_exact_selection(self) -> None:
        with fixture(
            {"core", "github", "devtools"}, fail_remove="codexy-github"
        ) as state:
            record(state.home, ["core", "github", "devtools"])
            receipt = run_operation(
                "remove",
                ("github", "devtools"),
                state.home,
                state.codex,
                state.run,
                operation_id="op-remove-fail",
            )
            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(state.selection, {"core", "github", "devtools"})

    def test_multi_remove_allows_removing_core_with_its_dependent(self) -> None:
        with fixture({"core", "github"}) as state:
            record(state.home, ["core", "github"])
            receipt = run_operation(
                "remove",
                ("core", "github"),
                state.home,
                state.codex,
                state.run,
                operation_id="op-remove-both",
            )

            self.assertEqual(receipt["outcome"], "completed")
            self.assertEqual(receipt["selection_after"], [])
            self.assertEqual(recorded(state.home), [])
            removals = [
                call[2] for call in state.mutations if call[:2] == ("plugin", "remove")
            ]
            self.assertEqual(removals, ["codexy-github@codexy", "codexy@codexy"])

    def test_update_without_a_recorded_selection_is_rejected(self) -> None:
        with fixture({"core"}) as state:
            receipt = run_operation(
                "update",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-missing-record",
            )

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(receipt["errors"], [{"code": "no-recorded-selection"}])
            self.assertEqual(state.mutations, [])

    def test_remove_requires_a_component_operand(self) -> None:
        with fixture({"core"}) as state:
            record(state.home, ["core"])
            receipt = run_operation(
                "remove",
                (),
                state.home,
                state.codex,
                state.run,
                operation_id="op-missing-remove",
            )

            self.assertEqual(receipt["outcome"], "rejected")
            self.assertEqual(receipt["errors"], [{"code": "missing-removal-target"}])
            self.assertEqual(state.mutations, [])


def record(home: Path, components: list[str]) -> None:
    target = inventory_path(home)
    target.parent.mkdir(parents=True)
    target.write_text(
        json.dumps(
            {
                "schema": "getcodexy.installed-component-inventory.v1",
                "components": components,
            }
        ),
        encoding="utf-8",
    )


def recorded(home: Path) -> list[str]:
    return json.loads(inventory_path(home).read_text(encoding="utf-8"))["components"]


if __name__ == "__main__":
    unittest.main()
