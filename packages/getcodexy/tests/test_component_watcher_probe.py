"""Focused lifecycle tests for the core Watcher capability probe."""

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools import component_watcher_probe as probe


class WatcherProbeTests(unittest.TestCase):
    def test_valid_no_mcp_core_executes_hooks_without_starting_watcher(self):
        from codexy_runtime_tools import component_capability_probe as capability_probe
        from codexy_runtime_tools.component_capability_observation import (
            component_observations,
        )
        from codexy_runtime_tools.component_mcp_materialization import (
            MCP_SPECS,
            materialize_component_mcp,
        )
        from packages.getcodexy.tests.component_lifecycle_support import fixture

        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            manifest_path = plugin / ".codex-plugin/plugin.json"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["version"] = "2.0.0"
            manifest.pop("mcpServers", None)
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            for relative in MCP_SPECS["core"].surface_paths:
                (plugin / relative).unlink(missing_ok=True)

            # Validate the selected no-MCP package before checking its probe path.
            self.assertEqual(materialize_component_mcp(plugin, "core", "2.0.0"), plugin)
            hook_script = plugin / "hooks/codexy-thread-delivery.sh"
            # Keep the registered hook real while trapping any Watcher launch.
            with patch.object(
                capability_probe,
                "_probe_watcher",
                side_effect=AssertionError("no-MCP core must not start Watcher"),
            ) as watcher_probe:
                healthy = capability_probe.probe_component(
                    "core", plugin, {"name": "codexy", "version": "2.0.0"}
                )
                # A failed remaining hook still controls core health on this path.
                hook_script.write_text("#!/bin/sh\nexit 37\n", encoding="utf-8")
                hook_script.chmod(0o755)
                failed = capability_probe.probe_component(
                    "core", plugin, {"name": "codexy", "version": "2.0.0"}
                )

        watcher_probe.assert_not_called()
        self.assertTrue(healthy["started"])
        self.assertTrue(healthy["callable"])
        self.assertEqual(
            healthy["_capability_probes"]["hook:codexy-thread-delivery"],
            {"configured": True, "started": True, "callable": True},
        )
        self.assertNotIn("mcp:watcher", component_observations("core", True, healthy))
        self.assertTrue(failed["started"])
        self.assertFalse(failed["callable"])
        self.assertEqual(failed["reason_code"], "capability-call-failed")
        self.assertEqual(
            failed["_capability_probes"]["hook:codexy-thread-delivery"],
            {"configured": True, "started": True, "callable": False},
        )

    def test_legacy_core_keeps_watcher_failure_in_health(self):
        from codexy_runtime_tools import component_capability_probe as capability_probe
        from codexy_runtime_tools.component_mcp_materialization import (
            materialize_component_mcp,
        )
        from packages.getcodexy.tests.component_lifecycle_support import fixture

        with fixture({"core"}) as state:
            plugin = state.marketplace / "plugins/codexy"
            manifest = json.loads(
                (plugin / ".codex-plugin/plugin.json").read_text(encoding="utf-8")
            )
            version = manifest["version"]
            self.assertEqual(materialize_component_mcp(plugin, "core", version), plugin)
            hook = {
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
            watcher = {
                "started": True,
                "callable": False,
                "reason_code": "capability-call-failed",
                "_capability_probes": {
                    "mcp:watcher": {
                        "configured": True,
                        "started": True,
                        "callable": False,
                    }
                },
            }
            with (
                patch.object(capability_probe, "_probe_hook", return_value=hook),
                patch.object(
                    capability_probe, "_probe_watcher", return_value=watcher
                ) as watcher_probe,
            ):
                result = capability_probe.probe_component(
                    "core", plugin, {"name": "codexy", "version": version}
                )

        watcher_probe.assert_called_once()
        self.assertTrue(result["started"])
        self.assertFalse(result["callable"])
        self.assertEqual(result["reason_code"], "capability-call-failed")
        self.assertEqual(
            result["_capability_probes"]["mcp:watcher"],
            {"configured": True, "started": True, "callable": False},
        )

    def test_probe_preserves_the_common_uv_bootstrap_argv(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            values = probe._argv(
                "uv",
                directory,
                [
                    "run",
                    "--no-project",
                    "--script",
                    "./mcp/codexy_mcp_bootstrap.py",
                    "watcher",
                    "--stdio",
                ],
            )
        self.assertEqual(
            values,
            [
                "uv",
                "run",
                "--no-project",
                "--script",
                "./mcp/codexy_mcp_bootstrap.py",
                "watcher",
                "--stdio",
            ],
        )

    def test_repeated_probe_assignments_are_unique(self) -> None:
        with (
            patch.object(probe.os, "getpid", return_value=42),
            patch.object(probe.time, "monotonic_ns", side_effect=(100, 101)),
        ):
            assignments = (probe._assignment_id(), probe._assignment_id())
        self.assertEqual(
            assignments,
            ("getcodexy-health-42-100", "getcodexy-health-42-101"),
        )

    def test_missing_watcher_runtime_preserves_a_failed_direct_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            plugin = Path(directory)
            (plugin / ".mcp.json").write_text(
                json.dumps(
                    {
                        "watcher": {
                            "command": str(plugin / "missing-watcher"),
                            "args": [],
                        }
                    }
                ),
                encoding="utf-8",
            )

            result = probe.probe_watcher(plugin, {"started": False, "callable": False})

        self.assertFalse(result["started"])
        self.assertFalse(result["callable"])
        self.assertEqual(result["reason_code"], "component-start-failed")
        self.assertEqual(
            result["_capability_probes"]["mcp:watcher"],
            {"configured": True, "started": False, "callable": False},
        )
