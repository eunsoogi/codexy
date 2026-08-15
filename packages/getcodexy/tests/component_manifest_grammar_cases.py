"""Manifest grammar and identity cases."""

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


class ComponentManifestGrammarCases:
    def test_manifest_rejects_renamed_marketplace_duplicate_assets_and_empty_asset_requirements(
        self,
    ) -> None:
        canonical = json.loads(
            (
                Path(__file__).parents[1]
                / "src/codexy_runtime_tools/component-manifest.json"
            ).read_text()
        )
        renamed = deepcopy(canonical)
        renamed["marketplace"]["name"] = "renamed"
        duplicate = deepcopy(canonical)
        duplicate["components"][1]["plugin"] = "codexy"
        duplicate["components"][1]["asset"] = deepcopy(
            duplicate["components"][0]["asset"]
        )
        empty_paths = deepcopy(canonical)
        empty_paths["components"][0]["asset"]["requiredPaths"] = []
        for invalid in (renamed, duplicate, empty_paths):
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    _parse_manifest(invalid)

    def test_manifest_rejects_incomplete_or_unknown_domain_error_projections(
        self,
    ) -> None:
        canonical = json.loads(
            (
                Path(__file__).parents[1]
                / "src/codexy_runtime_tools/component-manifest.json"
            ).read_text()
        )
        incomplete = deepcopy(canonical)
        incomplete["domainErrors"].pop("unknown-component")
        unknown = deepcopy(canonical)
        unknown["domainErrors"]["unrecognized-error"] = "not public"
        for invalid in (incomplete, unknown):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                _parse_manifest(invalid)

    def test_manifest_parser_rejects_duplicate_top_level_and_nested_keys(self) -> None:
        canonical = (
            Path(__file__).parents[1]
            / "src/codexy_runtime_tools/component-manifest.json"
        ).read_text()
        cases = [
            canonical.replace(
                '"schema": "getcodexy.component-manifest.v1",',
                '"schema": "getcodexy.component-manifest.v1", "schema": "getcodexy.component-manifest.v1",',
                1,
            ),
            canonical.replace(
                '"name": "codexy",', '"name": "codexy", "name": "codexy",', 1
            ),
        ]
        for text in cases:
            with self.assertRaisesRegex(ValueError, "duplicate key"):
                parse_component_manifest(text)

    def test_manifest_rejects_semver_components_outside_the_canonical_bound(
        self,
    ) -> None:
        canonical = json.loads(
            (
                Path(__file__).parents[1]
                / "src/codexy_runtime_tools/component-manifest.json"
            ).read_text()
        )
        for version in ("2147483648.0.0", "999999999999999999999.0.0", "01.0.0"):
            invalid = deepcopy(canonical)
            for component in invalid["components"]:
                component["version"] = version
            with self.subTest(version=version), self.assertRaises(ValueError):
                _parse_manifest(invalid)

    def test_reconciliation_rejects_malformed_official_unknown_future_versions_and_untrusted_roots(
        self,
    ) -> None:
        malformed_unknown = installed("unrelated")
        malformed_unknown["pluginId"] = "not-an-official-plugin-id"
        future = {"installed": [installed("codexy", "9.0.0")]}
        core = {"installed": [installed("codexy")]}
        cases = [
            (
                {"installed": [malformed_unknown]},
                self.marketplace_root,
                "invalid-installed-inventory",
            ),
            (future, self.marketplace_root, "component-version-mismatch"),
            (core, Path("/wrong-marketplace"), "conflicting-installed-state"),
        ]
        for inventory, root, code in cases:
            with (
                self.subTest(code=code),
                self.assertRaisesRegex(ComponentResolutionError, code),
            ):
                reconcile_installed_inventory(self.manifest, inventory, root)

    def test_registered_and_unregistered_inventory_share_the_exact_identity_grammar(
        self,
    ) -> None:
        canonical = installed("codexy")
        cases = [
            (
                [
                    {
                        "name": "codexylophone",
                        "pluginId": "codexylophone@other",
                        "marketplaceName": "other",
                    }
                ],
                None,
                (),
            ),
            (
                [
                    {
                        "name": "codexy",
                        "pluginId": "codexy@other",
                        "marketplaceName": "other",
                    }
                ],
                "conflicting-installed-state",
                "conflicting-installed-state",
            ),
            (
                [{"pluginId": "codexy@other", "marketplaceName": "other"}],
                "conflicting-installed-state",
                "conflicting-installed-state",
            ),
            (
                [
                    {
                        "name": "codexy",
                        "pluginId": "malformed",
                        "marketplaceName": "other",
                    }
                ],
                "conflicting-installed-state",
                "conflicting-installed-state",
            ),
            ([canonical], "conflicting-installed-state", ("core",)),
            (
                [canonical, installed("codexy-github")],
                "conflicting-installed-state",
                ("core", "github"),
            ),
            (
                [
                    {
                        "name": "future",
                        "pluginId": "future@codexy",
                        "marketplaceName": "codexy",
                    }
                ],
                "unknown-installed-component",
                "unknown-installed-component",
            ),
            (
                [
                    {
                        "name": "alpha",
                        "pluginId": "beta@other",
                        "marketplaceName": "other",
                    }
                ],
                "invalid-installed-inventory",
                "invalid-installed-inventory",
            ),
            (
                [canonical, canonical],
                "conflicting-installed-state",
                "conflicting-installed-state",
            ),
        ]
        for records, unregistered, registered in cases:
            with self.subTest(records=records):
                inventory = {"installed": records}
                classified = classify_installed_inventory(self.manifest, inventory)
                if unregistered is None:
                    preflight_unregistered_inventory(classified)
                else:
                    with self.assertRaisesRegex(ComponentResolutionError, unregistered):
                        preflight_unregistered_inventory(classified)
                if isinstance(registered, tuple):
                    self.assertEqual(
                        reconcile_installed_inventory(
                            self.manifest, inventory, self.marketplace_root
                        ),
                        registered,
                    )
                else:
                    with self.assertRaisesRegex(ComponentResolutionError, registered):
                        reconcile_installed_inventory(
                            self.manifest, inventory, self.marketplace_root
                        )
