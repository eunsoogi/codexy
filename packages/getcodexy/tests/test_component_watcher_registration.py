"""Doctor checks registered and cached Watcher bootstraps and probes the cache copy."""

from __future__ import annotations

import json
import os
import shutil
import unittest
from dataclasses import replace
from unittest.mock import patch

from codexy_runtime_tools import (
    component_registration_files,
    component_registration_health,
)
from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_lifecycle import run_operation
from codexy_runtime_tools.component_manifest import load_component_manifest
from codexy_runtime_tools.component_registration_catalog import _catalog_agent_files
from codexy_runtime_tools.component_registration_files import _text
from codexy_runtime_tools.component_registration_health import (
    MANAGED_MARKERS,
    valid_registration,
)
from codexy_runtime_tools.component_mcp_materialization import (
    MCP_SPECS,
    materialize_component_mcp,
    valid_component_mcp,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture
from packages.getcodexy.tests.component_lifecycle_support import VERSION
from packages.getcodexy.tests.component_hook_registration_fixture import (
    fixture_hook_rows,
)
from packages.getcodexy.tests.component_inspection_host_cases import _register_core


def _write_core_roles(home, plugin) -> None:
    root = home / "agents/codexy"
    root.mkdir(parents=True)
    catalog = _catalog_agent_files(_text(plugin / "agents/catalog.toml", plugin))
    for name in catalog:
        contents = MANAGED_MARKERS["core"] + _text(plugin / f"agents/{name}", plugin)
        (root / name).write_text(contents, encoding="utf-8")


def _make_no_mcp_core(plugin) -> tuple[str, object]:
    version = "2.0.0"
    path = plugin / ".codex-plugin/plugin.json"
    package = json.loads(path.read_text(encoding="utf-8"))
    package["version"] = version
    path.write_text(json.dumps(package), encoding="utf-8")
    for relative in MCP_SPECS["core"].surface_paths:
        (plugin / relative).unlink(missing_ok=True)
    base = load_component_manifest()
    manifest = replace(
        base,
        version=version,
        components=tuple(
            replace(component, version=version) for component in base.components
        ),
    )
    return version, manifest


class WatcherRegistrationTests(unittest.TestCase):
    def test_versioned_no_mcp_core_materializes_and_registers(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            version, manifest = _make_no_mcp_core(plugin)

            self.assertTrue(valid_component_mcp(plugin, "core", version))
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

            if os.name != "nt":
                residual = plugin / "mcp/codexy-mcp-watcher.sh"
                residual.symlink_to(plugin.parent / "missing-target")
                self.assertFalse(valid_component_mcp(plugin, "core", version))
                residual.unlink()
            package_path = plugin / ".codex-plugin/plugin.json"
            package = json.loads(package_path.read_text(encoding="utf-8"))
            package["version"] = VERSION
            package_path.write_text(json.dumps(package), encoding="utf-8")
            self.assertFalse(valid_component_mcp(plugin, "core", VERSION))
            self.assertTrue(
                all(
                    not os.path.lexists(plugin / path)
                    for path in MCP_SPECS["core"].surface_paths
                )
            )

    @unittest.skipIf(os.name == "nt", "creating a symlink requires Windows privileges")
    def test_install_rejects_a_symlinked_mcp_parent_before_materialization(
        self,
    ) -> None:
        with fixture() as state:
            core = state.marketplace / "plugins/codexy"
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

            self.assertEqual(receipt["outcome"], "rolled-back")
            self.assertEqual(marker.read_bytes(), b"preserve")

    def test_declared_watcher_rejects_a_missing_launcher(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            (plugin / "mcp/codexy-mcp-watcher.sh").unlink()

            self.assertFalse(valid_component_mcp(plugin, "core", VERSION))

    def test_sender_posttool_hook_is_registered_as_its_own_exact_concern(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            materialize_component_mcp(plugin, "core", VERSION)
            hooks = json.loads((plugin / "hooks/hooks.json").read_text())["hooks"]

            self.assertEqual(
                hooks["PostToolUse"],
                [
                    {
                        "matcher": "^mcp__codex_app__send_message_to_thread$",
                        "hooks": [
                            {
                                "type": "command",
                                "command": '"${PLUGIN_ROOT}/hooks/codexy-watcher-interrupt.sh" PostToolUse',
                                "commandWindows": '"${PLUGIN_ROOT}/hooks/codexy-watcher-interrupt.cmd" PostToolUse',
                                "timeout": 3,
                            }
                        ],
                    }
                ],
            )
            self.assertTrue(valid_registration(plugin, "core"))

    def test_doctor_rejects_a_missing_registered_watcher_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            _register_core(state, state.marketplace / "plugins/codexy")
            bootstrap = state.marketplace / "plugins/codexy/mcp/codexy_mcp_bootstrap.py"
            materialize_component_mcp(
                state.marketplace / "plugins/codexy", "core", VERSION
            )
            bootstrap.unlink()
            result = doctor(
                state.home,
                codex=state.codex,
                runner=state.run,
                hook_lister=lambda _executable, _home: fixture_hook_rows(
                    state.marketplace
                ),
            )

        health = result["component_health"][0]
        self.assertFalse(health["configured"])
        self.assertEqual(health["first_failure_stage"], "configured")
        self.assertEqual(health["reason_code"], "component-not-configured")

    def test_doctor_rejects_a_missing_cached_watcher_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            _register_core(state, state.marketplace / "plugins/codexy")
            materialize_component_mcp(
                state.marketplace / "plugins/codexy", "core", VERSION
            )
            cache = state.home / "plugins/cache/codexy/codexy" / VERSION
            shutil.copytree(state.marketplace / "plugins/codexy", cache)
            (cache / "mcp/codexy_mcp_bootstrap.py").unlink()
            result = doctor(
                state.home,
                codex=state.codex,
                runner=state.run,
                hook_lister=lambda _executable, _home: fixture_hook_rows(
                    state.marketplace
                ),
            )

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
                    "runtime_name": "codexy-watcher",
                    "runtime_version": VERSION,
                }

            with patch(
                "codexy_runtime_tools.component_health._probe_component",
                side_effect=successful_probe,
            ):
                result = doctor(
                    state.home,
                    codex=state.codex,
                    runner=state.run,
                    hook_lister=lambda _executable, _home: fixture_hook_rows(
                        state.marketplace
                    ),
                )

        self.assertEqual(probed, [cache])


if __name__ == "__main__":
    unittest.main()
