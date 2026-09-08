"""Focused lifecycle tests for the core Watcher capability probe."""

import unittest
from pathlib import Path
import tempfile
from unittest.mock import patch

from codexy_runtime_tools import component_watcher_probe as probe


class WatcherProbeTests(unittest.TestCase):
    def test_source_launcher_fallback_resolves_relative_to_plugin(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plugin = Path(directory)
            launcher = plugin / "mcp/codexy-mcp-watcher.sh"
            launcher.parent.mkdir()
            launcher.write_text("#!/bin/sh\n", encoding="utf-8")
            values = probe._argv(
                "./mcp/codexy-mcp-watcher", plugin, ["--stdio"]
            )
        self.assertEqual(values, [str(launcher), "--stdio"])

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
