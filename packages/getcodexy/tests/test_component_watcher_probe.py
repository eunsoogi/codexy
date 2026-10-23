"""Current doctor reports legacy Watcher registration without executing it."""

from __future__ import annotations

import unittest
from unittest.mock import patch

from codexy_runtime_tools import component_capability_probe
from codexy_runtime_tools.component_capability_observation import (
    component_observations,
)
from packages.getcodexy.tests.component_lifecycle_support import fixture
from packages.getcodexy.tests.component_marketplace_fixture import (
    configure_legacy_core_package,
)
from packages.getcodexy.tests.component_watcher_fixture import LEGACY_VERSION


class WatcherProbeTests(unittest.TestCase):
    def test_legacy_registration_stays_unprobed_by_the_current_doctor(self) -> None:
        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            configure_legacy_core_package(plugin, LEGACY_VERSION)

            def successful_hook(_component, _plugin, base):
                return {
                    **base,
                    "started": True,
                    "callable": True,
                    "_capability_probes": {
                        "hook:codexy-thread-delivery": {
                            "configured": True,
                            "started": True,
                            "callable": True,
                        }
                    },
                }

            with patch.object(
                component_capability_probe,
                "_probe_hook",
                side_effect=successful_hook,
            ) as probe_hook:
                result = component_capability_probe.probe_component(
                    "core",
                    plugin,
                    {"name": "codexy", "version": LEGACY_VERSION},
                )

        probe_hook.assert_called_once()
        self.assertEqual(result["_capability_contract"], {"mcp:watcher": True})
        self.assertNotIn("mcp:watcher", result["_capability_probes"])
        watcher = component_observations("core", True, result)["mcp:watcher"]
        self.assertEqual(
            watcher["states"],
            {
                "configured": "configured",
                "loaded": "unknown",
                "callable": "unknown",
                "verified": "unknown",
            },
        )
        self.assertEqual(watcher["source"], "getcodexy-registration-check")
        self.assertEqual(watcher["scope"], "current-doctor-invocation")


if __name__ == "__main__":
    unittest.main()
