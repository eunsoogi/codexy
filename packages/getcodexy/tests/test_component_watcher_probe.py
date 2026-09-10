"""Focused lifecycle tests for the core Watcher capability probe."""

import unittest
from pathlib import Path
import tempfile
from unittest.mock import patch

from codexy_runtime_tools import component_watcher_probe as probe


class WatcherProbeTests(unittest.TestCase):
    def test_probe_uses_registered_path_without_source_suffix_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plugin = Path(directory)
            launcher = plugin / "mcp/codexy-mcp-watcher.sh"
            launcher.parent.mkdir()
            launcher.write_text("#!/bin/sh\n", encoding="utf-8")
            values = probe._argv("./mcp/codexy-mcp-watcher", plugin, ["--stdio"])
        self.assertEqual(values, [str(launcher.with_suffix("")), "--stdio"])

    def test_windows_probe_does_not_choose_a_suffix_for_the_registered_path(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plugin = Path(directory)
            launcher = plugin / "mcp/codexy-mcp-watcher"
            launcher.parent.mkdir()
            for path in (launcher, Path(f"{launcher}.exe")):
                path.write_bytes(b"watcher")
            with patch.object(probe.os, "name", "nt"):
                values = probe._argv("./mcp/codexy-mcp-watcher", plugin)
        self.assertEqual(values, [str(launcher)])

    def test_windows_probe_does_not_fall_back_to_cmd(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plugin = Path(directory)
            launcher = plugin / "mcp/codexy-mcp-watcher"
            launcher.parent.mkdir()
            Path(f"{launcher}.cmd").write_text("@echo off\n", encoding="utf-8")
            with patch.object(probe.os, "name", "nt"):
                values = probe._argv("./mcp/codexy-mcp-watcher", plugin)
        self.assertEqual(values, [str(launcher)])

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
