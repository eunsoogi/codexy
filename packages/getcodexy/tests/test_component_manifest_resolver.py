from __future__ import annotations

import itertools
import json
import unittest
from copy import deepcopy
from pathlib import Path

from codexy_runtime_tools.component_manifest import _parse_manifest, load_component_manifest
from codexy_runtime_tools.component_resolver import (
    ComponentResolutionError,
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

    def test_reconciliation_rejects_malformed_official_unknown_future_versions_and_untrusted_roots(self) -> None:
        malformed_unknown = self._installed("unrelated")
        malformed_unknown["pluginId"] = "not-an-official-plugin-id"
        future = {"installed": [self._installed("codexy", "9.0.0")]}
        core = {"installed": [self._installed("codexy")]}
        cases = [
            ({"installed": [malformed_unknown]}, self.marketplace_root, "unknown-installed-component"),
            (future, self.marketplace_root, "component-version-mismatch"),
            (core, Path("/wrong-marketplace"), "conflicting-installed-state"),
        ]
        for inventory, root, code in cases:
            with self.subTest(code=code), self.assertRaisesRegex(ComponentResolutionError, code):
                reconcile_installed_inventory(self.manifest, inventory, root)

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
