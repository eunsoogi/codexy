"""Regression coverage for frozen, version-distinct local marketplaces."""

import json
import unittest
from codexy_runtime_tools.component_lifecycle_preflight import existing_marketplace
from packages.getcodexy.tests.local_marketplace_identity_fixture import (
    LocalHost,
    MANIFEST,
)


class LocalUpgradePreflightTests(unittest.TestCase):
    def test_prior_archive_is_validated_at_its_own_version(self):
        with LocalHost() as host:
            metadata = host.marketplace / ".agents/plugins/marketplace.json"
            data = json.loads(metadata.read_text())
            for entry in data["plugins"]:
                entry["version"] = "2.2.4"
                path = (
                    host.marketplace
                    / "plugins"
                    / entry["name"]
                    / ".codex-plugin/plugin.json"
                )
                plugin = json.loads(path.read_text())
                plugin["version"] = "2.2.4"
                path.write_text(json.dumps(plugin))
            metadata.write_text(json.dumps(data))
            binding = existing_marketplace(
                host.codex, host.run, MANIFEST, allow_older_local=True
            )
            self.assertEqual(binding.root, host.marketplace)
            with self.assertRaises(ValueError):
                existing_marketplace(host.codex, host.run, MANIFEST)

    def test_mixed_archive_versions_are_not_an_upgrade_source(self):
        with LocalHost() as host:
            metadata = host.marketplace / ".agents/plugins/marketplace.json"
            data = json.loads(metadata.read_text())
            entry = data["plugins"][0]
            entry["version"] = "2.2.4"
            path = (
                host.marketplace
                / "plugins"
                / entry["name"]
                / ".codex-plugin/plugin.json"
            )
            plugin = json.loads(path.read_text())
            plugin["version"] = "2.2.4"
            path.write_text(json.dumps(plugin))
            metadata.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError, "mixed or invalid"):
                existing_marketplace(
                    host.codex, host.run, MANIFEST, allow_older_local=True
                )
            self.assertEqual(host.mutations, [])

    def test_tampered_plugin_manifest_is_rejected_before_upgrade(self):
        with LocalHost() as host:
            path = host.marketplace / "plugins/codexy/.codex-plugin/plugin.json"
            plugin = json.loads(path.read_text())
            plugin["repository"] = "https://example.invalid/foreign"
            path.write_text(json.dumps(plugin))
            with self.assertRaises(ValueError):
                existing_marketplace(
                    host.codex, host.run, MANIFEST, allow_older_local=True
                )
            self.assertEqual(host.mutations, [])
