"""Doctor must admit only validated package contracts and registrations."""

from __future__ import annotations

import json
import shutil
import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_health import health
from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_mcp_materialization import (
    component_cache_plugin,
    mcp_configuration,
)
from codexy_runtime_tools.component_registration_health import valid_registration
from packages.getcodexy.tests.component_lifecycle_support import fixture, installed
from packages.getcodexy.tests.component_marketplace_fixture import (
    configure_no_mcp_core_package,
)
from packages.getcodexy.tests.local_marketplace_identity_fixture import LocalHost


class ComponentHealthAdmissionTests(unittest.TestCase):
    def test_invalid_source_capability_cannot_admit_a_legacy_cache(self) -> None:
        version = load_component_manifest().version
        with LocalHost() as host:
            plugin = host.marketplace / "plugins/codexy"
            configure_no_mcp_core_package(plugin, version)
            receipt = host.operate("install", "op-invalid-source-capability")
            self.assertEqual(receipt["outcome"], "completed", receipt)

            # A valid legacy cache must not replace the selected source's contract.
            cache = component_cache_plugin(host.home, "core", version)
            cache.parent.mkdir(parents=True)
            shutil.copytree(plugin, cache)
            cache_manifest_path = cache / ".codex-plugin/plugin.json"
            cache_manifest = json.loads(cache_manifest_path.read_text(encoding="utf-8"))
            cache_manifest["mcpServers"] = "./.mcp.json"
            cache_manifest_path.write_text(json.dumps(cache_manifest), encoding="utf-8")
            (cache / ".codex-plugin/mcp-capability.json").unlink()
            (cache / ".mcp.json").write_text(
                json.dumps(mcp_configuration("core", version)), encoding="utf-8"
            )

            source_capability = plugin / ".codex-plugin/mcp-capability.json"
            invalid_capability = json.loads(
                source_capability.read_text(encoding="utf-8")
            )
            invalid_capability["component"] = "codexy-github"
            source_capability.write_text(
                json.dumps(invalid_capability), encoding="utf-8"
            )
            with patch(
                "codexy_runtime_tools.component_health._probe_component",
                return_value={
                    "started": True,
                    "callable": True,
                    "runtime_name": "codexy-watcher",
                    "runtime_version": version,
                },
            ) as probe:
                report = doctor(host.home, codex=host.codex, runner=host.run)

        entry = report["component_health"][0]
        self.assertFalse(entry["configured"])
        self.assertEqual(entry["first_failure_stage"], "configured")
        probe.assert_not_called()

    def test_invalid_github_registration_is_not_reported_configured(self) -> None:
        manifest = load_component_manifest()
        with fixture({"github"}) as state:
            plugin = state.marketplace / "plugins/codexy-github"
            skill = plugin / "skills/git-workflow/SKILL.md"
            contents = skill.read_text(encoding="utf-8")
            self.assertTrue(contents.startswith("---\nname: git-workflow\n"))
            skill.write_text(
                contents.replace("name: git-workflow", "name: broken-workflow", 1),
                encoding="utf-8",
            )
            record = installed(state.marketplace, "github")
            # Non-MCP plugins still need registration validation before probing.
            self.assertFalse(valid_registration(plugin, "github", manifest.version))
            with patch(
                "codexy_runtime_tools.component_health._probe_component",
                return_value={
                    "started": True,
                    "callable": True,
                    "runtime_name": "codexy-github",
                    "runtime_version": manifest.version,
                },
            ) as probe:
                entry = health(
                    manifest, ("github",), None, {"github": record}, None, False
                )[0]

        self.assertFalse(entry["configured"])
        self.assertEqual(entry["first_failure_stage"], "configured")
        probe.assert_not_called()


if __name__ == "__main__":
    unittest.main()
