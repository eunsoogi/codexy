from __future__ import annotations

import shutil
import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_registration_catalog import _catalog_agent_files
from codexy_runtime_tools.component_registration_files import _text
from codexy_runtime_tools.component_registration_health import MANAGED_MARKERS
from codexy_runtime_tools.component_mcp_materialization import materialize_component_mcp
from packages.getcodexy.tests.component_lifecycle_support import fixture
from packages.getcodexy.tests.component_lifecycle_support import VERSION
from packages.getcodexy.tests.component_inspection_host_cases import _register_core


def _write_core_roles(home, plugin) -> None:
    root = home / "agents/codexy"
    root.mkdir(parents=True)
    catalog = _catalog_agent_files(_text(plugin / "agents/catalog.toml", plugin))
    for name in catalog:
        contents = MANAGED_MARKERS["core"] + _text(plugin / f"agents/{name}", plugin)
        (root / name).write_text(contents, encoding="utf-8")


class WatcherRegistrationTests(unittest.TestCase):
    def test_doctor_rejects_a_missing_registered_watcher_bootstrap(self) -> None:
        with fixture({"core"}) as state:
            _register_core(state, state.marketplace / "plugins/codexy")
            bootstrap = state.marketplace / "plugins/codexy/mcp/codexy_mcp_bootstrap.py"
            materialize_component_mcp(
                state.marketplace / "plugins/codexy", "core", VERSION
            )
            bootstrap.unlink()
            result = doctor(state.home, codex=state.codex, runner=state.run)

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
            result = doctor(state.home, codex=state.codex, runner=state.run)

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
                result = doctor(state.home, codex=state.codex, runner=state.run)

        self.assertEqual(probed, [cache])


if __name__ == "__main__":
    unittest.main()
