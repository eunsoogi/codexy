"""Manifest inventory reconciliation cases."""

from pathlib import Path
from copy import deepcopy
import json

from codexy_runtime_tools.component_manifest import (
    _parse_manifest,
    parse_component_manifest,
)
from codexy_runtime_tools.component_resolver import (
    ComponentResolutionError,
    admit_recovery_inventory,
    classify_installed_inventory,
    preflight_unregistered_inventory,
    reconcile_installed_inventory,
    verify_post_operation_inventory,
)
from packages.getcodexy.tests.component_manifest_records import installed


class ComponentManifestReconciliationCases:
    def test_reconciliation_uses_host_inventory_and_rejects_bad_states(self) -> None:
        inventory = {"installed": [installed("codexy-github"), installed("codexy")]}
        self.assertEqual(
            reconcile_installed_inventory(
                self.manifest, inventory, Path("/marketplace")
            ),
            ("core", "github"),
        )
        cases = [
            ([installed("codexy-github")], "inconsistent-installed-state"),
            (
                [installed("codexy"), installed("codexy")],
                "conflicting-installed-state",
            ),
            (
                [installed("codexy"), installed("codexy-github", "1.2.0")],
                "mixed-version-state",
            ),
            ([installed("unrelated")], "unknown-installed-component"),
        ]
        for records, code in cases:
            with (
                self.subTest(code=code),
                self.assertRaisesRegex(ComponentResolutionError, code),
            ):
                reconcile_installed_inventory(
                    self.manifest, {"installed": records}, self.marketplace_root
                )

    def test_post_operation_inventory_must_be_fresh_target_state(self) -> None:
        old = {"installed": [installed("codexy", "1.2.0")]}
        self.assertEqual(
            reconcile_installed_inventory(self.manifest, old, self.marketplace_root),
            ("core",),
        )
        with self.assertRaisesRegex(
            ComponentResolutionError, "component-version-mismatch"
        ):
            verify_post_operation_inventory(
                self.manifest, old, ("core",), self.marketplace_root
            )
        current = {"installed": [installed("codexy"), installed("codexy-github")]}
        self.assertEqual(
            verify_post_operation_inventory(
                self.manifest, current, ("core", "github"), self.marketplace_root
            ),
            ("core", "github"),
        )

    def test_pending_update_admission_allows_only_its_own_mixed_version_selection(
        self,
    ) -> None:
        mixed = {
            "installed": [
                installed("codexy", self.manifest.version),
                installed("codexy-github", "1.2.0"),
            ]
        }
        with self.assertRaisesRegex(ComponentResolutionError, "mixed-version-state"):
            reconcile_installed_inventory(self.manifest, mixed, self.marketplace_root)
        self.assertEqual(
            admit_recovery_inventory(
                self.manifest, mixed, self.marketplace_root, ("core", "github")
            ),
            ("core", "github"),
        )
        with self.assertRaisesRegex(
            ComponentResolutionError, "inconsistent-installed-state"
        ):
            admit_recovery_inventory(
                self.manifest, mixed, self.marketplace_root, ("core",)
            )
        for inventory, code in [
            (
                {
                    "installed": [
                        installed("codexy", "1.2.0"),
                        installed("codexy-github", "1.1.0"),
                    ]
                },
                "mixed-version-state",
            ),
            (
                {
                    "installed": [
                        installed("codexy", "9.0.0"),
                        installed("codexy-github", "1.2.0"),
                    ]
                },
                "component-version-mismatch",
            ),
        ]:
            with (
                self.subTest(code=code),
                self.assertRaisesRegex(ComponentResolutionError, code),
            ):
                admit_recovery_inventory(
                    self.manifest, inventory, self.marketplace_root, ("core", "github")
                )
