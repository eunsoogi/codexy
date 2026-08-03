from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "stage13_windows_slow_path.py"
WORKFLOW = ROOT / ".github" / "workflows" / "rust-test.yml"
DIAGNOSTIC_WORKFLOW = ROOT / ".github" / "workflows" / "stage13-windows-slow-path.yml"


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
    def test_trace_records_ordered_target_test_timestamps_gaps_and_hook_overhead(self):
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
            self.assertEqual(receipt["workload"], ["cargo", "test", "--locked", "--all-targets"])
            self.assertEqual(
                [event["test"] for event in receipt["test_completions"]],
                ["suite_all::alpha", "suite_all::beta"],
            )
            self.assertIsNone(receipt["test_completions"][0]["gap_seconds"])
            self.assertGreaterEqual(receipt["test_completions"][1]["gap_seconds"], 0)
            self.assertEqual(
                [event["state"] for event in receipt["target_boundaries"]],
                ["started", "completed"],
            )
            self.assertGreaterEqual(receipt["observer_upper_bound_seconds"], 0)
            self.assertGreaterEqual(receipt["perturbation_upper_bound_percent"], 0)

    def test_run_delegates_to_existing_profile_main_with_only_runtime_replaced(self):
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

    def test_workflows_preserve_four_path_filter_and_canonical_windows_contract(self):
        workflow = WORKFLOW.read_text()
        expected_filter = """  pull_request:
    paths-ignore:
      - \".github/workflows/rust-test.yml\"
      - \".github/workflows/stage13-windows-slow-path.yml\"
      - \"scripts/stage13_windows_slow_path.py\"
      - \"tests/test_stage13_windows_slow_path.py\""""
        self.assertIn(expected_filter, workflow)
        self.assertIn("  push:\n    branches: [main]", workflow)

        sys.path.insert(0, str(ROOT / "scripts"))
        try:
            from profile_rust_workflow import enforce_workflow_contract

            enforce_workflow_contract(
                WORKFLOW, 6, ("cargo", "test", "--locked", "--all-targets")
            )
        finally:
            sys.path.pop(0)

        diagnostic = DIAGNOSTIC_WORKFLOW.read_text()
        self.assertIn("branches: [codexy/526-stage13b-windows-slow-path]", diagnostic)
        self.assertIn("python scripts/stage13_windows_slow_path.py --trace $trace --windows", diagnostic)
        self.assertIn("actions/upload-artifact@v7", diagnostic)


if __name__ == "__main__":
    unittest.main()
