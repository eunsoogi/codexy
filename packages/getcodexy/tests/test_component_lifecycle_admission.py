from __future__ import annotations

import json
import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_lifecycle import inventory_path, run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_resolver import (
    classify_installed_inventory,
    preflight_unregistered_inventory,
)
from codexy_runtime_tools.component_transaction_state import (
    InventorySnapshot,
    Journal,
    read_journal,
    write_journal,
)
from packages.getcodexy.tests.component_lifecycle_admission_pending_cases import (
    LifecycleAdmissionPendingCases,
)
from packages.getcodexy.tests.component_lifecycle_admission_recovery_cases import (
    LifecycleAdmissionRecoveryCases,
)
from packages.getcodexy.tests.component_lifecycle_admission_replay_cases import (
    LifecycleAdmissionReplayCases,
)
from packages.getcodexy.tests.component_lifecycle_admission_terminal_cases import (
    LifecycleAdmissionTerminalCases,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture


class LifecycleAdmissionTests(
    LifecycleAdmissionPendingCases,
    LifecycleAdmissionReplayCases,
    LifecycleAdmissionRecoveryCases,
    LifecycleAdmissionTerminalCases,
    unittest.TestCase,
):
    def test_identityless_records_reject_before_new_operation_mutation_on_both_paths(
        self,
    ) -> None:
        records = (
            {},
            {"name": "unrelated"},
            {"pluginId": "unrelated@other"},
            {"marketplaceName": "other"},
            {"name": "alpha", "pluginId": "beta@other", "marketplaceName": "other"},
            {
                "name": "alpha",
                "pluginId": "alpha@other",
                "marketplaceName": "different",
            },
            {"name": "", "pluginId": "@other", "marketplaceName": "other"},
        )
        for marketplace_present in (False, True):
            for record in records:
                with (
                    self.subTest(
                        marketplace_present=marketplace_present, record=record
                    ),
                    fixture(
                        marketplace_present=marketplace_present,
                        inventory_override={"installed": [record]},
                    ) as state,
                ):
                    receipt = run_operation(
                        "install",
                        ("core",),
                        state.home,
                        state.codex,
                        state.run,
                        operation_id="op-identityless",
                    )
                    self.assertEqual(
                        receipt["errors"], [{"code": "invalid-installed-inventory"}]
                    )
                    self.assertEqual(state.marketplace_present, marketplace_present)
                self.assertEqual(state.mutations, [])
                self.assertIsNone(read_journal(state.home))

    def test_complete_consistent_non_codexy_identity_is_the_only_ignored_record(
        self,
    ) -> None:
        record = {
            "name": "alpha",
            "pluginId": "alpha@other",
            "marketplaceName": "other",
        }
        self.assertIsNone(
            preflight_unregistered_inventory(
                classify_installed_inventory(
                    load_component_manifest(), {"installed": [record]}
                )
            )
        )


def _receipt(
    identifier: str,
    command: str,
    requested: tuple[str, ...],
    resolved: tuple[str, ...],
    before: tuple[str, ...],
    after: tuple[str, ...],
    outcome: str,
    errors: list[dict[str, str]] | None = None,
) -> dict[str, object]:
    return {
        "schema": "getcodexy.operation-receipt.v1",
        "operation_id": identifier,
        "command": command,
        "outcome": outcome,
        "requested_components": list(requested),
        "resolved_components": list(resolved),
        "selection_before": list(before),
        "selection_after": list(after),
        "installed_components": list(after),
        "source_of_truth": "installed-component-inventory",
        "errors": [] if errors is None else errors,
    }


if __name__ == "__main__":
    unittest.main()
