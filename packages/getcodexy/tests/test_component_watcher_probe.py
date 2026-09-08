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

    def test_windows_prefers_native_binary_over_extensionless_launcher(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plugin = Path(directory)
            launcher = plugin / "mcp/codexy-mcp-watcher"
            launcher.parent.mkdir()
            for path in (launcher, Path(f"{launcher}.exe")):
                path.write_bytes(b"watcher")
            with patch.object(probe.os, "name", "nt"):
                values = probe._argv("./mcp/codexy-mcp-watcher", plugin)
        self.assertEqual(values, [f"{launcher}.exe"])

    def test_windows_falls_back_to_cmd_when_native_binary_is_missing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plugin = Path(directory)
            launcher = plugin / "mcp/codexy-mcp-watcher"
            launcher.parent.mkdir()
            Path(f"{launcher}.cmd").write_text("@echo off\n", encoding="utf-8")
            with patch.object(probe.os, "name", "nt"):
                values = probe._argv("./mcp/codexy-mcp-watcher", plugin)
        self.assertIsInstance(values, str)
        self.assertIn(f"{launcher}.cmd", values)

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
