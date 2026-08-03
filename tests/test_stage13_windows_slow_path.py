from __future__ import annotations

import importlib.util
import json
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "stage13_windows_slow_path.py"


def load_diagnostic():
    spec = importlib.util.spec_from_file_location("stage13_windows_slow_path", SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError("diagnostic module is not loadable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class FakeRuntimeTelemetry:
    def __init__(self, started, declared, environment):
        self._started = started
        self.lines = []

    def _observe_line(self, line):
        self.lines.append(line)

    def finish(self):
        return '{"schema":"control"}'


class Stage13WindowsSlowPathTests(unittest.TestCase):
    def test_trace_records_ordered_target_test_timestamps_and_gaps(self):
        diagnostic = load_diagnostic()
        with tempfile.TemporaryDirectory() as directory:
            trace = Path(directory) / "trace.json"
            runtime_type = diagnostic.instrument_runtime(FakeRuntimeTelemetry, trace)
            runtime = runtime_type(time.perf_counter(), ["suite_all"], {})
            runtime._observe_line("     Running tests/suites/all.rs (suite-all.exe)")
            runtime._observe_line("test alpha ... ok")
            runtime._observe_line("test beta ... output")
            runtime._observe_line("ok")
            runtime._observe_line(
                "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
            )
            self.assertEqual(runtime.finish(), '{"schema":"control"}')

            receipt = json.loads(trace.read_text())
            self.assertEqual(receipt["schema"], "codexy.stage13-windows-slow-path/v1")
            self.assertEqual(
                [event["test"] for event in receipt["test_completions"]],
                ["suite_all::alpha", "suite_all::beta"],
            )
            self.assertEqual(receipt["test_completions"][0]["gap_seconds"], None)
            self.assertGreaterEqual(receipt["test_completions"][1]["gap_seconds"], 0)
            self.assertEqual(
                [event["state"] for event in receipt["target_boundaries"]],
                ["started", "completed"],
            )
            self.assertGreaterEqual(receipt["diagnostic_hook_seconds"], 0)
            self.assertGreaterEqual(receipt["observer_elapsed_seconds"], 0)

    def test_run_delegates_to_exact_profile_main_with_only_runtime_replaced(self):
        diagnostic = load_diagnostic()
        profile = SimpleNamespace(RuntimeTelemetry=FakeRuntimeTelemetry)
        seen = {}

        def profile_main():
            seen["runtime"] = profile.RuntimeTelemetry
            seen["argv"] = list(diagnostic.sys.argv)
            return 17

        profile.main = profile_main
        with tempfile.TemporaryDirectory() as directory:
            trace = Path(directory) / "trace.json"
            with mock.patch.object(diagnostic, "load_profile_module", return_value=profile):
                status = diagnostic.run(trace, ["--windows"])
        self.assertEqual(status, 17)
        self.assertTrue(issubclass(seen["runtime"], FakeRuntimeTelemetry))
        self.assertEqual(seen["argv"][1:], ["--windows"])


if __name__ == "__main__":
    unittest.main()
