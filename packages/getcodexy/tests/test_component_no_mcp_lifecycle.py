"""Exercise the checked-in no-MCP candidate through a local package lifecycle."""

from __future__ import annotations

import json
import shutil
import unittest
from pathlib import Path

from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_lifecycle_mcp import materialize_mcp_caches
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_mcp_materialization import (
    component_cache_plugin,
    materialize_component_mcp,
    mcp_configuration,
    valid_component_mcp_cache,
)
from packages.getcodexy.tests.local_marketplace_identity_fixture import LocalHost

REPOSITORY = Path(__file__).resolve().parents[3]


class NoMcpLifecycleTests(unittest.TestCase):
    def test_install_doctor_and_cache_repair_use_the_checked_in_candidate(self) -> None:
        version = load_component_manifest().version
        with LocalHost() as host:
            plugin = host.marketplace / "plugins/codexy"
            # The default local-host fixture models an older release; replace only its
            # temporary package with the exact candidate source under test.
            shutil.rmtree(plugin)
            shutil.copytree(REPOSITORY / "plugins/codexy", plugin)
            package_path = plugin / ".codex-plugin/plugin.json"
            package = json.loads(package_path.read_text(encoding="utf-8"))
            self.assertEqual(package["version"], version)
            self.assertNotIn("mcpServers", package)
            self.assertNotIn("mcp_servers", package)
            self.assertFalse((plugin / ".mcp.json").exists())
            self.assertEqual(materialize_component_mcp(plugin, "core", version), plugin)

            receipt = host.operate("install", "op-install-no-mcp-core")
            self.assertEqual(receipt["outcome"], "completed", receipt)

            cache = component_cache_plugin(host.home, "core", version)
            cache.parent.mkdir(parents=True)
            shutil.copytree(plugin, cache)
            # Model a same-version stale cache left by an earlier registered package.
            cached_manifest_path = cache / ".codex-plugin/plugin.json"
            cached_manifest = json.loads(
                cached_manifest_path.read_text(encoding="utf-8")
            )
            cached_manifest["mcpServers"] = "./.mcp.json"
            cached_manifest_path.write_text(
                json.dumps(cached_manifest), encoding="utf-8"
            )
            (cache / ".mcp.json").write_text(
                json.dumps(mcp_configuration("core", version)), encoding="utf-8"
            )
            before_repair = doctor(host.home, codex=host.codex, runner=host.run)
            self.assertFalse(before_repair["component_health"][0]["configured"])

            materialize_mcp_caches(
                host.home,
                host.marketplace,
                load_component_manifest(),
                ("core",),
            )
            self.assertTrue(
                valid_component_mcp_cache(
                    host.home, "core", version, source_plugin=plugin
                )
            )
            after_repair = doctor(host.home, codex=host.codex, runner=host.run)
            health = after_repair["component_health"][0]
            self.assertTrue(health["configured"], health)
            self.assertEqual(health["state"], "healthy", health)
            self.assertNotIn("mcp:watcher", health["observed"]["capabilities"])
            for root in (plugin, cache):
                self.assertFalse((root / ".mcp.json").exists())
                self.assertNotIn(
                    "mcpServers",
                    json.loads((root / ".codex-plugin/plugin.json").read_text()),
                )
                self.assertTrue((root / ".codex-plugin/mcp-capability.json").is_file())


if __name__ == "__main__":
    unittest.main()
