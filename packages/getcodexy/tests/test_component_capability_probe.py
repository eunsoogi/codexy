"""Focused process and launcher tests for installed capability probes."""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from packages.getcodexy.tests import component_distribution_support as support


class _OSProxy:
    def __init__(self, name: str) -> None:
        self.name = name

    def __getattr__(self, attribute: str):
        return getattr(os, attribute)


class _Pipe:
    def __init__(self, descriptor: int) -> None:
        self.descriptor = descriptor
        self.close_calls = 0

    def fileno(self) -> int:
        return self.descriptor

    def close(self) -> None:
        self.close_calls += 1


class CapabilityProcessTests(unittest.TestCase):
    def test_windows_batch_hooks_use_explicit_clean_command_processor(self) -> None:
        from codexy_runtime_tools import component_capability_probe as probe

        with tempfile.TemporaryDirectory() as directory:
            paths, batch, raw, py, ran = support.windows_argv(
                probe, Path(directory), _OSProxy("nt")
            )
        for launcher, command in zip(paths, batch, strict=True):
            self.assertIn(" /d /s /c ", command)
            self.assertTrue(command.endswith(f'""{launcher}" PermissionRequest"'))
        self.assertEqual(raw[0], "powershell.exe -NoProfile -File hook.ps1".split())
        self.assertEqual(raw[1], [str(py), "hook.py"])
        self.assertEqual(ran, (0, 0, 0) if os.name == "nt" else ())

    def test_process_failures_keep_timeout_exit_and_missing_distinct(self) -> None:
        from codexy_runtime_tools import component_capability_probe as probe
        from codexy_runtime_tools import component_capability_probe_process as process

        cases = (
            (subprocess.TimeoutExpired(["hook"], 5), "timeout"),
            (subprocess.CompletedProcess(["hook"], 9, stdout=""), "nonzero-exit"),
            (OSError("missing"), "missing-launcher"),
        )
        self.assertNotIn("creationflags", probe._RUN_OPTIONS)
        for outcome, category in cases:
            with self.subTest(category=category):
                with (
                    patch.object(process, "os", _OSProxy("posix")),
                    patch.object(process.subprocess, "run", side_effect=[outcome]),
                ):
                    result = probe._run(["hook"], Path.cwd(), "{}")
                self.assertEqual(result.category, category)

    def test_process_result_captures_bounded_diagnostics(self) -> None:
        from codexy_runtime_tools import component_capability_probe_process as probe

        detail = "first line\nsecond line\n" + ("x" * 300)
        completed = subprocess.CompletedProcess(["hook"], 9, stdout="", stderr=detail)
        with (
            patch.object(probe, "os", _OSProxy("posix")),
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
        with (
            patch.object(probe, "os", _OSProxy("nt")),
            patch.object(probe.subprocess, "Popen", return_value=process),
            patch.object(probe, "perf_counter", side_effect=(10.0, 10.5, 14.5)),
        ):
            result = probe._run(["hook"], Path.cwd(), "{}")
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
        with (
            patch.object(probe, "os", _OSProxy("nt")),
            patch.object(probe.subprocess, "Popen", return_value=process),
            patch.object(
                probe,
                "_terminate_process_tree",
                side_effect=lambda target, deadline: target.kill(),
            ),
        ):
            result = probe._run(["hook"], Path.cwd(), "{}")
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

    def test_windows_timeout_force_closes_pipe_handles_after_tree_kill_failure(
        self,
    ) -> None:
        from codexy_runtime_tools import component_capability_probe_process as probe

        streams = tuple(_Pipe(descriptor) for descriptor in (11, 12, 13))
        process = unittest.mock.Mock(
            stdin=streams[0], stdout=streams[1], stderr=streams[2]
        )
        process.poll.return_value = None

        def kill() -> None:
            process.poll.return_value = 1

        process.kill.side_effect = kill
        process.communicate.side_effect = [
            subprocess.TimeoutExpired(
                ["hook"], 5, output=b"partial", stderr=b"diagnostic"
            ),
            subprocess.TimeoutExpired(
                ["hook"], 1, output=b"partial", stderr=b"diagnostic"
            ),
        ]
        windows_os = _OSProxy("nt")
        with (
            patch.object(probe, "os", windows_os),
            patch.object(windows_os, "close") as close,
            patch.object(probe.subprocess, "Popen", return_value=process),
            patch.object(
                probe,
                "_terminate_process_tree",
                side_effect=lambda target, deadline: target.kill(),
            ),
            patch.object(
                probe, "perf_counter", side_effect=(10.0, 10.0, 10.0, 10.0, 11.5)
            ),
        ):
            result = probe._run(["hook"], Path.cwd(), "{}")

        self.assertEqual(result.category, "timeout")
        self.assertEqual(result.detail, "diagnostic")
        self.assertEqual(process.communicate.call_count, 2)
        self.assertEqual(
            close.call_args_list,
            [unittest.mock.call(11), unittest.mock.call(12), unittest.mock.call(13)],
        )
        self.assertTrue(all(stream.close_calls == 0 for stream in streams))
        process.kill.assert_called_once_with()

    @unittest.skipUnless(os.name == "nt", "Windows process-tree regression")
    def test_windows_timeout_closes_pipes_when_tree_kill_leaves_descendant(
        self,
    ) -> None:
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
                "open(os.environ['CODEXY_TIMEOUT_PID_FILE'], 'w').write(str(os.getpid()))\n"
                "time.sleep(30)\n",
                encoding="utf-8",
            )
            launcher.write_text(
                '@echo off\r\npy -3 -I -B "%~dp0pipe-holder.py"\r\n',
                encoding="utf-8",
            )
            try:
                with patch.object(
                    process.subprocess,
                    "run",
                    return_value=subprocess.CompletedProcess([], 1),
                ):
                    result = process._run(
                        probe._argv(f'"{launcher}" PermissionRequest', root),
                        root,
                        "{}",
                        os.environ | {"CODEXY_TIMEOUT_PID_FILE": str(pid_file)},
                    )
            finally:
                if pid_file.is_file():
                    subprocess.run(
                        ["taskkill", "/pid", pid_file.read_text().strip(), "/t", "/f"],
                        check=False,
                        capture_output=True,
                        timeout=5,
                    )
            self.assertEqual(result.category, "timeout")
            self.assertIsNone(result.returncode)
            self.assertLess(
                result.elapsed_seconds, process._RUN_OPTIONS["timeout"] + 1.5
            )
            self.assertTrue(pid_file.is_file())
            self.assertFalse(support.host_process_active(pid_file))
