"""Focused process and launcher tests for installed capability probes."""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from packages.getcodexy.tests import component_distribution_support as support


class CapabilityProcessTests(unittest.TestCase):
    def test_windows_batch_hooks_use_explicit_clean_command_processor(self) -> None:
        from codexy_runtime_tools import component_capability_probe as probe

        with tempfile.TemporaryDirectory() as directory:
            paths, batch, raw, py, ran = support.windows_argv(probe, Path(directory))
        for launcher, command in zip(paths, batch, strict=True):
            self.assertIn(" /d /s /c ", command)
            self.assertTrue(command.endswith(f'""{launcher}" PermissionRequest"'))
        self.assertEqual(raw[0], "powershell.exe -NoProfile -File hook.ps1".split())
        self.assertEqual(raw[1], [str(py), "hook.py"])
        self.assertEqual(ran, (0, 0, 0) if os.name == "nt" else ())

    def test_process_failures_keep_timeout_exit_and_missing_distinct(self) -> None:
        from codexy_runtime_tools import component_capability_probe as probe

        cases = (
            (subprocess.TimeoutExpired(["hook"], 5), "timeout"),
            (subprocess.CompletedProcess(["hook"], 9, stdout=""), "nonzero-exit"),
            (OSError("missing"), "missing-launcher"),
        )
        self.assertNotIn("creationflags", probe._RUN_OPTIONS)
        for outcome, category in cases:
            with self.subTest(category=category):
                with patch.object(probe.subprocess, "run", side_effect=[outcome]):
                    result = probe._run(["hook"], Path.cwd(), "{}")
                self.assertEqual(result.category, category)

    def test_process_result_captures_bounded_diagnostics(self) -> None:
        from codexy_runtime_tools import component_capability_probe_process as probe

        detail = "first line\nsecond line\n" + ("x" * 300)
        completed = subprocess.CompletedProcess(["hook"], 9, stdout="", stderr=detail)
        with (
            patch.object(probe.subprocess, "run", return_value=completed),
            patch.object(probe, "perf_counter", side_effect=(10.0, 10.125)),
        ):
            result = probe._run(["hook"], Path.cwd(), "{}")
        expected = ("first line second line " + ("x" * 300))[
            : probe._PROBE_DETAIL_LIMIT
        ]
        self.assertEqual(
            (result.category, result.returncode, result.elapsed_seconds, result.detail),
            ("nonzero-exit", 9, 0.125, expected),
        )
