from __future__ import annotations

import itertools
import json
import unittest
from copy import deepcopy
from pathlib import Path

from codexy_runtime_tools.component_manifest import _parse_manifest, load_component_manifest, parse_component_manifest
from codexy_runtime_tools.component_resolver import (
    ComponentResolutionError,
    classify_installed_inventory,
    preflight_unregistered_inventory,
    reconcile_installed_inventory,
    resolve_components,
    verify_post_operation_inventory,
)


class ComponentManifestResolverTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = load_component_manifest()
        self.marketplace_root = Path("/marketplace")

    def test_manifest_matches_the_three_packaged_plugin_products(self) -> None:
        self.assertEqual(self.manifest.component_ids, ("core", "github", "devtools"))
        self.assertEqual(self.manifest.component("core").plugin, "codexy")
        self.assertEqual(self.manifest.component("github").dependencies, ("core",))
        self.assertEqual(self.manifest.component("devtools").dependencies, ("core",))
        self.assertTrue(self.manifest.component("core").asset.required_paths)
        self.assertEqual(self.manifest.version, "1.3.0")
        root = Path(__file__).parents[3]
        for component in self.manifest.components:
            plugin_root = root / component.asset.package_root
            plugin = json.loads((plugin_root / ".codex-plugin/plugin.json").read_text())
            self.assertEqual((plugin["name"], plugin["version"]), (component.plugin, component.version))
            for asset in component.asset.required_paths:
                self.assertTrue((plugin_root / asset).is_file())

    def test_manifest_carries_the_closed_public_domain_error_contract(self) -> None:
        self.assertEqual(
            set(self.manifest.domain_errors),
            {
                "component-version-mismatch",
                "components-not-accepted",
                "conflicting-component-request",
                "conflicting-installed-state",
                "dependency-protected-removal",
                "incompatible-component-selection",
                "inconsistent-installed-state",
                "installed-state-mismatch",
                "invalid-installed-inventory",
                "missing-removal-target",
                "mixed-version-state",
                "no-recorded-selection",
                "operation-failed",
                "unknown-component",
                "unknown-installed-component",
            },
        )

    def test_resolver_cannot_emit_an_error_outside_the_manifest_contract(self) -> None:
        with self.assertRaisesRegex(ValueError, "unknown getcodexy component domain error"):
            ComponentResolutionError("not-a-public-error")

    def test_every_subset_and_operand_order_resolves_canonically(self) -> None:
        components = self.manifest.component_ids
        for size in range(len(components) + 1):
            for subset in itertools.combinations(components, size):
                selected = subset or components
                expected = tuple(
                    component
                    for component in components
                    if component == "core" and any(item != "core" for item in selected)
                    or component in selected
                )
                for requested in itertools.permutations(subset):
                    with self.subTest(requested=requested):
                        self.assertEqual(
                            resolve_components(self.manifest, requested), expected
                        )

    def test_unknown_and_duplicate_requests_fail_before_a_mutation_plan(self) -> None:
        for requested, code in [
            (("unknown",), "unknown-component"),
            (("github", "github"), "conflicting-component-request"),
        ]:
            with self.subTest(requested=requested):
                with self.assertRaises(ComponentResolutionError) as raised:
                    resolve_components(self.manifest, requested)
                self.assertEqual(raised.exception.code, code)

    def test_reconciliation_uses_host_inventory_and_rejects_bad_states(self) -> None:
        inventory = {"installed": [self._installed("codexy-github"), self._installed("codexy")]}
        self.assertEqual(
            reconcile_installed_inventory(self.manifest, inventory, Path("/marketplace")), ("core", "github")
        )
        cases = [
            ([self._installed("codexy-github")], "inconsistent-installed-state"),
            ([self._installed("codexy"), self._installed("codexy")], "conflicting-installed-state"),
            ([self._installed("codexy"), self._installed("codexy-github", "1.2.0")], "mixed-version-state"),
            ([self._installed("unrelated")], "unknown-installed-component"),
        ]
        for installed, code in cases:
            with self.subTest(code=code), self.assertRaisesRegex(ComponentResolutionError, code):
                reconcile_installed_inventory(self.manifest, {"installed": installed}, self.marketplace_root)

    def test_post_operation_inventory_must_be_fresh_target_state(self) -> None:
        old = {"installed": [self._installed("codexy", "1.2.0")]}
        self.assertEqual(reconcile_installed_inventory(self.manifest, old, self.marketplace_root), ("core",))
        with self.assertRaisesRegex(ComponentResolutionError, "component-version-mismatch"):
            verify_post_operation_inventory(self.manifest, old, ("core",), self.marketplace_root)
        current = {"installed": [self._installed("codexy"), self._installed("codexy-github")]}
        self.assertEqual(
            verify_post_operation_inventory(self.manifest, current, ("core", "github"), self.marketplace_root),
            ("core", "github"),
        )

    def test_manifest_rejects_renamed_marketplace_duplicate_assets_and_empty_asset_requirements(self) -> None:
        canonical = json.loads(
            (Path(__file__).parents[1] / "src/codexy_runtime_tools/component-manifest.json").read_text()
        )
        renamed = deepcopy(canonical)
        renamed["marketplace"]["name"] = "renamed"
        duplicate = deepcopy(canonical)
        duplicate["components"][1]["plugin"] = "codexy"
        duplicate["components"][1]["asset"] = deepcopy(duplicate["components"][0]["asset"])
        empty_paths = deepcopy(canonical)
        empty_paths["components"][0]["asset"]["requiredPaths"] = []
        for invalid in (renamed, duplicate, empty_paths):
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    _parse_manifest(invalid)

    def test_manifest_rejects_incomplete_or_unknown_domain_error_projections(self) -> None:
        canonical = json.loads(
            (Path(__file__).parents[1] / "src/codexy_runtime_tools/component-manifest.json").read_text()
        )
        incomplete = deepcopy(canonical)
        incomplete["domainErrors"].pop("unknown-component")
        unknown = deepcopy(canonical)
        unknown["domainErrors"]["unrecognized-error"] = "not public"
        for invalid in (incomplete, unknown):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                _parse_manifest(invalid)

    def test_manifest_parser_rejects_duplicate_top_level_and_nested_keys(self) -> None:
        canonical = (Path(__file__).parents[1] / "src/codexy_runtime_tools/component-manifest.json").read_text()
        cases = [
            canonical.replace('"schema": "getcodexy.component-manifest.v1",', '"schema": "getcodexy.component-manifest.v1", "schema": "getcodexy.component-manifest.v1",', 1),
            canonical.replace('"name": "codexy",', '"name": "codexy", "name": "codexy",', 1),
        ]
        for text in cases:
            with self.assertRaisesRegex(ValueError, "duplicate key"):
                parse_component_manifest(text)

    def test_manifest_rejects_semver_components_outside_the_canonical_bound(self) -> None:
        canonical = json.loads((Path(__file__).parents[1] / "src/codexy_runtime_tools/component-manifest.json").read_text())
        for version in ("2147483648.0.0", "999999999999999999999.0.0", "01.0.0"):
            invalid = deepcopy(canonical)
            for component in invalid["components"]:
                component["version"] = version
            with self.subTest(version=version), self.assertRaises(ValueError):
                _parse_manifest(invalid)

    def test_reconciliation_rejects_malformed_official_unknown_future_versions_and_untrusted_roots(self) -> None:
        malformed_unknown = self._installed("unrelated")
        malformed_unknown["pluginId"] = "not-an-official-plugin-id"
        future = {"installed": [self._installed("codexy", "9.0.0")]}
        core = {"installed": [self._installed("codexy")]}
        cases = [
            ({"installed": [malformed_unknown]}, self.marketplace_root, "invalid-installed-inventory"),
            (future, self.marketplace_root, "component-version-mismatch"),
            (core, Path("/wrong-marketplace"), "conflicting-installed-state"),
        ]
        for inventory, root, code in cases:
            with self.subTest(code=code), self.assertRaisesRegex(ComponentResolutionError, code):
                reconcile_installed_inventory(self.manifest, inventory, root)

    def test_registered_and_unregistered_inventory_share_the_exact_identity_grammar(self) -> None:
        canonical = self._installed("codexy")
        cases = [
            ([{"name": "codexylophone", "pluginId": "codexylophone@other", "marketplaceName": "other"}], None, ()),
            ([{"name": "codexy", "pluginId": "codexy@other", "marketplaceName": "other"}], "conflicting-installed-state", "conflicting-installed-state"),
            ([{"pluginId": "codexy@other", "marketplaceName": "other"}], "conflicting-installed-state", "conflicting-installed-state"),
            ([{"name": "codexy", "pluginId": "malformed", "marketplaceName": "other"}], "conflicting-installed-state", "conflicting-installed-state"),
            ([canonical], "conflicting-installed-state", ("core",)),
            ([canonical, self._installed("codexy-github")], "conflicting-installed-state", ("core", "github")),
            ([{"name": "future", "pluginId": "future@codexy", "marketplaceName": "codexy"}], "unknown-installed-component", "unknown-installed-component"),
            ([{"name": "alpha", "pluginId": "beta@other", "marketplaceName": "other"}], "invalid-installed-inventory", "invalid-installed-inventory"),
            ([canonical, canonical], "conflicting-installed-state", "conflicting-installed-state"),
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
                    self.assertEqual(reconcile_installed_inventory(self.manifest, inventory, self.marketplace_root), registered)
                else:
                    with self.assertRaisesRegex(ComponentResolutionError, registered):
                        reconcile_installed_inventory(self.manifest, inventory, self.marketplace_root)

    @staticmethod
    def _installed(plugin: str, version: str = "1.3.0") -> dict[str, object]:
        return {
            "pluginId": f"{plugin}@codexy",
            "name": plugin,
            "marketplaceName": "codexy",
            "version": version,
            "installed": True,
            "enabled": True,
            "source": {"source": "local", "path": f"/marketplace/plugins/{plugin}"},
            "marketplaceSource": {
                "sourceType": "git", "source": "https://github.com/eunsoogi/codexy.git"
            },
        }


if __name__ == "__main__":
    unittest.main()
