"""Focused lifecycle tests for the core Watcher capability probe."""

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from codexy_runtime_tools import component_watcher_probe as probe


class WatcherProbeTests(unittest.TestCase):
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
