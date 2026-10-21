"""Exercise install, doctor, and cache repair for the no-MCP core package."""

from __future__ import annotations

import json
import shutil
import unittest
from contextlib import ExitStack
from dataclasses import replace
from unittest.mock import patch

from codexy_runtime_tools import (
    component_inspection,
    component_registration_files,
    component_registration_health,
)
from codexy_runtime_tools import component_lifecycle_operation
from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_lifecycle_mcp import materialize_mcp_caches
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_mcp_materialization import (
    component_cache_plugin,
    valid_component_mcp,
    valid_component_mcp_cache,
)
from packages.getcodexy.tests import component_lifecycle_support
from packages.getcodexy.tests.component_hook_registration_fixture import (
    fixture_hook_rows,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture


class NoMcpLifecycleTests(unittest.TestCase):
    def test_install_doctor_and_cache_repair_keep_core_unregistered(self) -> None:
        version = "2.0.0"
        # An isolated candidate version selects the intentional no-MCP contract.
        current = load_component_manifest()
        manifest = replace(
            current,
            version=version,
            components=tuple(
                replace(component, version=version) for component in current.components
            ),
        )

        with patch.object(component_lifecycle_support, "VERSION", version):
            with fixture(real_registration=True) as state:
                plugin = state.marketplace / "plugins/codexy"
                package_path = plugin / ".codex-plugin/plugin.json"
                package = json.loads(package_path.read_text(encoding="utf-8"))
                package["version"] = version
                package.pop("mcpServers", None)
                package_path.write_text(json.dumps(package), encoding="utf-8")
                (plugin / ".mcp.json").unlink(missing_ok=True)

                loaders = (
                    component_lifecycle_operation,
                    component_inspection,
                    component_registration_health,
                    component_registration_files,
                )
                with ExitStack() as stack:
                    for module in loaders:
                        stack.enter_context(
                            patch.object(
                                module,
                                "load_component_manifest",
                                return_value=manifest,
                            )
                        )
                    receipt = run_operation(
                        "install",
                        ("core",),
                        state.home,
                        state.codex,
                        state.run,
                        operation_id="op-install-no-mcp-core",
                        hook_lister=lambda _executable, _home: fixture_hook_rows(
                            state.marketplace
                        ),
                    )
                    self.assertEqual(receipt["outcome"], "completed", receipt)
                    self.assertTrue(valid_component_mcp(plugin, "core", version))
                    self.assertTrue(
                        component_registration_health.valid_registration(plugin, "core")
                    )

                    cache = component_cache_plugin(state.home, "core", version)
                    cache.parent.mkdir(parents=True)
                    shutil.copytree(plugin, cache)
                    materialize_mcp_caches(
                        state.home, state.marketplace, manifest, ("core",)
                    )

                    result = doctor(
                        state.home,
                        codex=state.codex,
                        runner=state.run,
                        hook_lister=lambda _executable, _home: fixture_hook_rows(
                            state.marketplace
                        ),
                    )
                    health = result["component_health"][0]
                    self.assertTrue(health["configured"])
                    self.assertEqual(health["state"], "healthy", health)
                    self.assertNotIn("mcp:watcher", health["observed"]["capabilities"])
                    for root in (plugin, cache):
                        self.assertFalse((root / ".mcp.json").exists())
                        self.assertTrue(
                            (root / "mcp/codexy_mcp_bootstrap.py").is_file(), root
                        )
                        self.assertTrue(
                            (root / "mcp/codexy-mcp-watcher.sh").is_file(), root
                        )
                        self.assertTrue(
                            (root / "mcp/codexy-mcp-watcher.cmd").is_file(), root
                        )
                    self.assertTrue(
                        valid_component_mcp_cache(state.home, "core", version)
                    )


if __name__ == "__main__":
    unittest.main()
