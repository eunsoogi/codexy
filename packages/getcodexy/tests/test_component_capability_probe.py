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
                cwd = Path.cwd()
                with (
                    patch.object(probe.os, "name", "posix"),
                    patch.object(probe.subprocess, "run", side_effect=[outcome]),
                ):
                    result = probe._run(["hook"], cwd, "{}")
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

    def test_windows_probe_accepts_completion_near_five_second_deadline(self) -> None:
        from codexy_runtime_tools import component_capability_probe_process as probe

        process = unittest.mock.Mock()
        process.poll.return_value = 0
        process.returncode = 0
        process.communicate.return_value = ("", "")
        cwd = Path.cwd()
        with (
            patch.object(probe.os, "name", "nt"),
            patch.object(probe.subprocess, "Popen", return_value=process),
            patch.object(probe, "perf_counter", side_effect=(10.0, 10.5, 14.5)),
        ):
            result = probe._run(["hook"], cwd, "{}")
        self.assertEqual((result.category, result.returncode), ("success", 0))
        self.assertEqual(result.elapsed_seconds, 4.5)
        self.assertAlmostEqual(process.communicate.call_args.kwargs["timeout"], 4.5)

    def test_windows_timeout_cleanup_does_not_drain_inherited_pipes(self) -> None:
        from codexy_runtime_tools import component_capability_probe_process as probe

        process = unittest.mock.Mock()
        process.poll.return_value = None

        def kill() -> None:
            process.poll.return_value = 1

        process.kill.side_effect = kill
        process.communicate.side_effect = [
            subprocess.TimeoutExpired(
                ["hook"], 5, output=b"partial", stderr=b"diagnostic"
            ),
            ("", ""),
        ]
        cwd = Path.cwd()
        with (
            patch.object(probe.os, "name", "nt"),
            patch.object(probe.subprocess, "Popen", return_value=process),
            patch.object(
                probe,
                "_terminate_process_tree",
                side_effect=lambda target, deadline: target.kill(),
            ),
        ):
            result = probe._run(["hook"], cwd, "{}")
        self.assertEqual(
            (result.category, result.returncode, result.detail),
            ("timeout", None, "diagnostic"),
        )
        self.assertEqual(process.communicate.call_count, 2)
        self.assertEqual(process.communicate.call_args_list[0].args, ("{}",))
        self.assertGreater(process.communicate.call_args_list[0].kwargs["timeout"], 4.9)
        self.assertLessEqual(process.communicate.call_args_list[0].kwargs["timeout"], 5)
        self.assertLessEqual(
            process.communicate.call_args_list[1].kwargs["timeout"], 1.0
        )
        process.kill.assert_called_once_with()

    @unittest.skipUnless(os.name == "nt", "Windows process-tree regression")
    def test_windows_timeout_kills_cmd_descendant_without_pipe_overrun(self) -> None:
        from codexy_runtime_tools import component_capability_probe as probe
        from codexy_runtime_tools import component_capability_probe_process as process

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            launcher = root / "pipe-holder.cmd"
            holder = root / "pipe-holder.py"
            pid_file = root / "pipe-holder.pid"
            holder.write_text(
                "import os, subprocess, sys, time\n"
                "child = subprocess.Popen([sys.executable, '-c', "
                '"import time; time.sleep(30)"] )\n'
                "open(os.environ['CODEXY_TIMEOUT_PID_FILE'], 'w').write(str(child.pid))\n"
                "time.sleep(30)\n",
                encoding="utf-8",
            )
            launcher.write_text(
                '@echo off\r\npy -3 -I -B "%~dp0pipe-holder.py"\r\n',
                encoding="utf-8",
            )
            result = process._run(
                probe._argv(f'"{launcher}" PermissionRequest', root),
                root,
                "{}",
                os.environ | {"CODEXY_TIMEOUT_PID_FILE": str(pid_file)},
            )
            self.assertEqual(result.category, "timeout")
            self.assertIsNone(result.returncode)
            self.assertLess(
                result.elapsed_seconds, process._RUN_OPTIONS["timeout"] + 1.5
            )
            self.assertTrue(pid_file.is_file())
            self.assertFalse(support.host_process_active(pid_file))
