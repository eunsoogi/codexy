"""Focused lifecycle tests for the core Watcher capability probe."""

import unittest
import tempfile
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
