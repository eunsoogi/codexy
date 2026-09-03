from __future__ import annotations

import itertools
import json
import unittest
from copy import deepcopy
from pathlib import Path

from codexy_runtime_tools.component_manifest import (
    _parse_manifest,
    load_component_manifest,
    parse_component_manifest,
)
from packages.getcodexy.tests.component_manifest_grammar_cases import (
    ComponentManifestGrammarCases,
)
from packages.getcodexy.tests.component_manifest_reconciliation_cases import (
    ComponentManifestReconciliationCases,
)
from codexy_runtime_tools.component_resolver import (
    ComponentResolutionError,
    admit_recovery_inventory,
    classify_installed_inventory,
    preflight_unregistered_inventory,
    reconcile_installed_inventory,
    resolve_components,
    verify_post_operation_inventory,
)


class ComponentManifestResolverTests(
    ComponentManifestReconciliationCases,
    ComponentManifestGrammarCases,
    unittest.TestCase,
):
    def setUp(self) -> None:
        self.manifest = load_component_manifest()
        self.marketplace_root = Path("/marketplace")

    def test_manifest_matches_the_three_packaged_plugin_products(self) -> None:
        self.assertEqual(self.manifest.component_ids, ("core", "github", "devtools"))
        self.assertEqual(self.manifest.component("core").plugin, "codexy")
        self.assertEqual(self.manifest.component("github").dependencies, ("core",))
        self.assertEqual(self.manifest.component("devtools").dependencies, ("core",))
        self.assertTrue(self.manifest.component("core").asset.required_paths)
        root = Path(__file__).parents[3]
        selected_version = json.loads(
            (root / ".agents/plugins/release-publish-contract.json").read_text()
        )["bootstrap"]["selectedVersion"]
        self.assertEqual(self.manifest.version, selected_version)
        for component in self.manifest.components:
            plugin_root = root / component.asset.package_root
            plugin = json.loads((plugin_root / ".codex-plugin/plugin.json").read_text())
            self.assertEqual(
                (plugin["name"], plugin["version"]),
                (component.plugin, selected_version),
            )
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
                "required-hook-disabled",
                "required-hook-trust-missing",
                "required-hook-trust-stale",
                "hook-state-unavailable",
                "unknown-component",
                "unknown-installed-component",
            },
        )

    def test_resolver_cannot_emit_an_error_outside_the_manifest_contract(self) -> None:
        with self.assertRaisesRegex(
            ValueError, "unknown getcodexy component domain error"
        ):
            ComponentResolutionError("not-a-public-error")

    def test_every_subset_and_operand_order_resolves_canonically(self) -> None:
        components = self.manifest.component_ids
        for size in range(len(components) + 1):
            for subset in itertools.combinations(components, size):
                selected = subset or components
                expected = tuple(
                    component
                    for component in components
                    if component == "core"
                    and any(item != "core" for item in selected)
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


if __name__ == "__main__":
    unittest.main()
