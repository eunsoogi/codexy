from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "stage14_windows_completion.py"
CANONICAL_WORKFLOW = ROOT / ".github" / "workflows" / "rust-test.yml"
DIAGNOSTIC_WORKFLOW = ROOT / ".github" / "workflows" / "stage14-windows-completion.yml"
EXPECTED_JOB_DIGEST = "d4fe217d930f06e267be4988b6f86c0641bb088e5d93e159d5c114ed5d2f7751"


def load_diagnostic():
    spec = importlib.util.spec_from_file_location("stage14_windows_completion", SCRIPT)
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


class Stage14WindowsCompletionTests(unittest.TestCase):
    def test_trace_records_ordered_timeline_and_receipt_schema(self):
        diagnostic = load_diagnostic()
        with tempfile.TemporaryDirectory() as directory:
            trace = Path(directory) / "trace.json"
            runtime_type = diagnostic.instrument_runtime(
                FakeRuntimeTelemetry,
                trace,
                diagnostic.parity_metadata(),
            )
            runtime = runtime_type(
                time.perf_counter(),
                ["suite_all", "suite_archive"],
                dict(os.environ),
            )
            runtime._observe_line("     Running tests/suites/all.rs (suite-all.exe)")
            runtime._observe_line("test alpha ... ok")
            runtime._observe_line(
                "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
            )
            runtime._observe_line("     Running tests/suites/archive.rs (suite-archive.exe)")
            runtime._observe_line("test beta ... output")
            runtime._observe_line("ok")
            runtime._observe_line(
                "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
            )
            self.assertEqual(runtime.finish(), '{"schema":"control"}')

            receipt = json.loads(trace.read_text())
            self.assertEqual(receipt["schema"], "codexy.stage14-windows-completion/v1")
            self.assertEqual(receipt["acceptance_budget_seconds"], 300.0)
            self.assertEqual(receipt["diagnostic_observation_seconds"], 900.0)
            self.assertEqual(receipt["workload"], ["cargo", "test", "--locked", "--all-targets"])
            self.assertEqual(receipt["profile_argv"], ["--windows", "--budget-seconds", "900.0"])
            self.assertEqual(receipt["wrapper_delegations"], 1)
            self.assertEqual(
                [event["test"] for event in receipt["test_completions"]],
                ["suite_all::alpha", "suite_archive::beta"],
            )
            self.assertEqual(
                [event["state"] for event in receipt["target_boundaries"]],
                ["started", "completed", "started", "completed"],
            )
            self.assertLessEqual(receipt["perturbation_limit_percent"], 0.05)
            self.assertEqual(receipt["phase_b"]["minimum_occupancy_seconds"], 60.0)
            self.assertEqual(
                receipt["phase_b"]["required_metric"],
                "conservative_union_occupancy_seconds",
            )

    def test_run_delegates_once_with_exact_argv_and_restores_only_ceiling(self):
        diagnostic = load_diagnostic()
        original_runtime = FakeRuntimeTelemetry
        profile = SimpleNamespace(RuntimeTelemetry=original_runtime)
        interval = SimpleNamespace(MAX_INTERVAL_NANOSECONDS=300_000_000_000)
        calls = []

        def profile_main():
            calls.append(list(diagnostic.sys.argv))
            self.assertTrue(issubclass(profile.RuntimeTelemetry, original_runtime))
            self.assertEqual(interval.MAX_INTERVAL_NANOSECONDS, 900_000_000_000)
            runtime = profile.RuntimeTelemetry(
                time.perf_counter(), ["suite_all"], dict(os.environ)
            )
            runtime.finish()
            return 17

        profile.main = profile_main
        with tempfile.TemporaryDirectory() as directory:
            trace = Path(directory) / "trace.json"
            with mock.patch.object(
                diagnostic, "load_profile_modules", return_value=(profile, interval)
            ):
                status = diagnostic.run(trace)

            receipt = json.loads(trace.read_text())
        self.assertEqual(status, 17)
        self.assertEqual(len(calls), 1)
        self.assertEqual(
            calls[0][1:], ["--windows", "--budget-seconds", "900.0"]
        )
        self.assertIs(profile.RuntimeTelemetry, original_runtime)
        self.assertEqual(interval.MAX_INTERVAL_NANOSECONDS, 300_000_000_000)
        self.assertEqual(receipt["parser_ceiling_before_nanoseconds"], 300_000_000_000)
        self.assertEqual(receipt["parser_ceiling_active_nanoseconds"], 900_000_000_000)
        self.assertEqual(receipt["parser_ceiling_after_nanoseconds"], 300_000_000_000)

    def test_protected_environment_is_observed_without_mutation(self):
        diagnostic = load_diagnostic()
        protected = {
            "RUST_TEST_THREADS": "7",
            "RUSTFLAGS": "-Cdebuginfo=1",
            "CARGO_BUILD_JOBS": "3",
            "CARGO_PROFILE_TEST_OPT_LEVEL": "1",
        }
        with mock.patch.dict(os.environ, protected, clear=False):
            expected = diagnostic.protected_environment(os.environ)
            observed = diagnostic.protected_environment(dict(os.environ))
        self.assertEqual(observed, expected)
        self.assertEqual(set(protected).intersection(expected), set(protected))

    def test_workflows_have_exact_trigger_filter_and_one_producer(self):
        canonical = CANONICAL_WORKFLOW.read_text()
        expected_filter = """  pull_request:
    paths-ignore:
      - \".github/workflows/rust-test.yml\"
      - \".github/workflows/stage14-windows-completion.yml\"
      - \"scripts/stage14_windows_completion.py\"
      - \"tests/test_stage14_windows_completion.py\""""
        self.assertIn(expected_filter, canonical)
        jobs = canonical[canonical.index("jobs:\n") :]
        self.assertEqual(hashlib.sha256(jobs.encode()).hexdigest(), EXPECTED_JOB_DIGEST)

        diagnostic = DIAGNOSTIC_WORKFLOW.read_text()
        self.assertIn(
            "branches: [codexy/526-stage14-windows-completion]", diagnostic
        )
        self.assertNotIn("pull_request:", diagnostic)
        self.assertNotIn("workflow_dispatch:", diagnostic)
        self.assertNotIn("matrix:", diagnostic)
        self.assertNotIn("retry", diagnostic.casefold())
        self.assertEqual(diagnostic.count("runs-on: windows-latest"), 1)
        self.assertEqual(
            diagnostic.count("python scripts/stage14_windows_completion.py"), 1
        )
        self.assertEqual(diagnostic.count("actions/upload-artifact@v7"), 1)
        self.assertEqual(diagnostic.count("if: always()"), 1)
        for required in (
            "scripts/install-windows-test-prerequisites.ps1",
            "rustup toolchain install",
            "cargo fetch --locked",
            "timeout-minutes: 20",
        ):
            self.assertIn(required, diagnostic)


if __name__ == "__main__":
    unittest.main()
