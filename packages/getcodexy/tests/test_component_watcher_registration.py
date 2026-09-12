from __future__ import annotations

import shutil
import unittest
from unittest.mock import patch

from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_mcp_materialization import materialize_component_mcp
from packages.getcodexy.tests.component_lifecycle_support import fixture
from packages.getcodexy.tests.component_lifecycle_support import VERSION


class WatcherRegistrationTests(unittest.TestCase):
    def test_doctor_rejects_a_missing_registered_watcher_bootstrap(self) -> None:
        with fixture({"core"}) as state:
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
