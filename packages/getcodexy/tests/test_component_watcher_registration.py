"""Doctor checks registered and cached Watcher bootstraps and probes the cache copy."""

from __future__ import annotations

import json
import os
import shutil
import unittest
from unittest.mock import patch

from codexy_runtime_tools import (
    component_registration_files,
    component_registration_health,
)
from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_registration_catalog import _catalog_agent_files
from codexy_runtime_tools.component_registration_files import _text
from codexy_runtime_tools.component_registration_health import (
    MANAGED_MARKERS,
    valid_registration,
)
from codexy_runtime_tools.component_mcp_materialization import (
    materialize_component_mcp,
    valid_component_mcp,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture
from packages.getcodexy.tests.component_lifecycle_support import VERSION
from packages.getcodexy.tests.component_hook_registration_fixture import (
    fixture_hook_rows,
)
from packages.getcodexy.tests.component_inspection_host_cases import _register_core
from packages.getcodexy.tests.component_marketplace_fixture import (
    configure_legacy_core_package,
    make_no_mcp_core_package,
)
from packages.getcodexy.tests.component_watcher_fixture import LEGACY_VERSION


def _write_core_roles(home, plugin) -> None:
    root = home / "agents/codexy"
    root.mkdir(parents=True)
    catalog = _catalog_agent_files(_text(plugin / "agents/catalog.toml", plugin))
    for name in catalog:
        contents = MANAGED_MARKERS["core"] + _text(plugin / f"agents/{name}", plugin)
        (root / name).write_text(contents, encoding="utf-8")


class WatcherRegistrationTests(unittest.TestCase):
    def _doctor(self, state):
        """Run doctor with hook inventory from this fixture's marketplace."""
        return doctor(
            state.home,
            codex=state.codex,
            runner=state.run,
            hook_lister=lambda _executable, _home: fixture_hook_rows(state.marketplace),
        )

    def test_absent_registration_requires_an_explicit_no_mcp_contract(self) -> None:
        with fixture() as state:
            plugin = state.marketplace / "plugins/codexy"
            version, _manifest = make_no_mcp_core_package(plugin)
            capability = plugin / ".codex-plugin/mcp-capability.json"
            capability.unlink(missing_ok=True)

            self.assertFalse(valid_component_mcp(plugin, "core", version))
            with self.assertRaisesRegex(RuntimeError, "surface is invalid"):
                materialize_component_mcp(plugin, "core", version)

    def test_versioned_no_mcp_core_materializes_and_registers(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            version, manifest = make_no_mcp_core_package(plugin)
            package_path = plugin / ".codex-plugin/plugin.json"
            package = json.loads(package_path.read_text(encoding="utf-8"))

            self.assertTrue(valid_component_mcp(plugin, "core", version))
            self.assertEqual(materialize_component_mcp(plugin, "core", version), plugin)
            self.assertFalse(valid_component_mcp(plugin, "core"))
            with (
                patch.object(
                    component_registration_health,
                    "load_component_manifest",
                    return_value=manifest,
                ),
                patch.object(
                    component_registration_files,
                    "load_component_manifest",
                    return_value=manifest,
                ),
            ):
                self.assertTrue(valid_registration(plugin, "core"))
                # Only None selects the bundled release; an empty value stays invalid.
                self.assertFalse(valid_registration(plugin, "core", ""))
                if os.name != "nt":
                    residual = plugin / ".mcp.json"
                    residual.symlink_to(plugin.parent / "missing-mcp-config")
                    self.assertFalse(valid_component_mcp(plugin, "core", version))
                    residual.unlink()
                for declaration in ("./.mcp.json", "../outside/mcp.json"):
                    package["mcpServers"] = declaration
                    package_path.write_text(json.dumps(package), encoding="utf-8")
                    self.assertFalse(valid_component_mcp(plugin, "core", version))
                    self.assertFalse(valid_registration(plugin, "core"))
                    with self.assertRaisesRegex(RuntimeError, "surface is invalid"):
                        materialize_component_mcp(plugin, "core", version)

            # Keep legacy registration coverage explicit after the current package drops MCP assets.
            configure_legacy_core_package(plugin, LEGACY_VERSION)
            self.assertTrue(valid_component_mcp(plugin, "core", LEGACY_VERSION))

    @unittest.skipIf(os.name == "nt", "creating a symlink requires Windows privileges")
    def test_install_rejects_a_symlinked_mcp_parent_before_materialization(
        self,
    ) -> None:
        with fixture() as state:
            core = state.marketplace / "plugins/codexy"
            # Exercise path safety only after registering the compatibility MCP surface.
            configure_legacy_core_package(core, VERSION)
            self.assertEqual(materialize_component_mcp(core, "core", VERSION), core)
            self.assertTrue(valid_component_mcp(core, "core", VERSION))
            outside = state.root / "outside"
            outside.mkdir()
            moved = outside / "mcp"
            (core / "mcp").rename(moved)
            marker = moved / "codexy-mcp-watcher"
            marker.write_bytes(b"preserve")
            (core / "mcp").symlink_to(moved, target_is_directory=True)

            receipt = run_operation(
                "install",
                ("core",),
                state.home,
                state.codex,
                state.run,
                operation_id="op-install-symlinked-mcp-parent",
            )

            self.assertEqual(
                receipt["outcome"],
                "rolled-back",
                {"receipt": receipt, "mutations": state.mutations},
            )
            self.assertFalse(
                any(mutation[:2] == ("plugin", "add") for mutation in state.mutations)
            )
            self.assertEqual(marker.read_bytes(), b"preserve")

    def test_declared_watcher_rejects_a_missing_launcher(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            configure_legacy_core_package(plugin, LEGACY_VERSION)
            self.assertEqual(
                materialize_component_mcp(plugin, "core", LEGACY_VERSION), plugin
            )
            self.assertTrue(valid_component_mcp(plugin, "core", LEGACY_VERSION))
            (plugin / "mcp/codexy-mcp-watcher.sh").unlink()

            self.assertFalse(valid_component_mcp(plugin, "core", LEGACY_VERSION))

    def test_core_registration_contains_only_retained_routing_hooks(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            materialize_component_mcp(plugin, "core", VERSION)
            hooks = json.loads((plugin / "hooks/hooks.json").read_text())["hooks"]

            self.assertEqual(set(hooks), {"PermissionRequest", "PreToolUse"})
            expected_matchers = {
                "^(?:codex_app__|mcp__codex_app__)send_message_to_thread$",
                "^(?:codex_app__|mcp__codex_app__)create_thread$",
                "^(?:(?:agents|multi_agent_v1)__)?(?:spawn_agent|send_message|followup_task)$",
            }
            for event in ("PermissionRequest", "PreToolUse"):
                self.assertEqual(
                    {group["matcher"] for group in hooks[event]}, expected_matchers
                )
            self.assertTrue(valid_registration(plugin, "core"))

    def test_doctor_rejects_a_missing_registered_watcher_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            configure_legacy_core_package(plugin, LEGACY_VERSION)
            _register_core(state, plugin)
            bootstrap = plugin / "mcp/codexy_mcp_bootstrap.py"
            materialize_component_mcp(plugin, "core", LEGACY_VERSION)
            bootstrap.unlink()
            result = self._doctor(state)

        health = result["component_health"][0]
        self.assertFalse(health["configured"])
        self.assertEqual(health["first_failure_stage"], "configured")
        self.assertEqual(health["reason_code"], "component-not-configured")

    def test_doctor_rejects_a_missing_cached_watcher_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            # Match the host inventory so this assertion isolates the legacy MCP surface.
            configure_legacy_core_package(plugin, VERSION)
            _register_core(state, plugin)
            materialize_component_mcp(plugin, "core", VERSION)
            cache = state.home / "plugins/cache/codexy/codexy" / VERSION
            shutil.copytree(plugin, cache)
            (cache / "mcp/codexy_mcp_bootstrap.py").unlink()
            result = self._doctor(state)

        health = result["component_health"][0]
        self.assertFalse(health["configured"])
        self.assertEqual(health["state"], "stale")
        self.assertEqual(health["first_failure_stage"], "configured")
        self.assertEqual(health["reason_code"], "component-not-configured")

    def test_doctor_probes_the_host_cache_copy_when_it_exists(self) -> None:
        with fixture({"core"}) as state:
            _write_core_roles(state.home, state.marketplace / "plugins/codexy")
            materialize_component_mcp(
                state.marketplace / "plugins/codexy", "core", VERSION
            )
            cache = state.home / "plugins/cache/codexy/codexy" / VERSION
            shutil.copytree(state.marketplace / "plugins/codexy", cache)
            probed: list[object] = []

            def successful_probe(_component, plugin, _record):
                probed.append(plugin)
                return {
                    "started": True,
                    "callable": True,
                    "runtime_name": "codexy",
                    "runtime_version": VERSION,
                }

            with patch(
                "codexy_runtime_tools.component_health._probe_component",
                side_effect=successful_probe,
            ):
                result = self._doctor(state)

        self.assertEqual(probed, [cache])


if __name__ == "__main__":
    unittest.main()
