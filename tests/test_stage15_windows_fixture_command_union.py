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
SCRIPT = ROOT / "scripts" / "stage15_windows_fixture_command_union.py"
CANONICAL_WORKFLOW = ROOT / ".github" / "workflows" / "rust-test.yml"
DIAGNOSTIC_WORKFLOW = (
    ROOT / ".github" / "workflows" / "stage15-windows-fixture-command-union.yml"
)
EXPECTED_JOB_DIGEST = "d4fe217d930f06e267be4988b6f86c0641bb088e5d93e159d5c114ed5d2f7751"
SESSION = "0123456789abcdef0123456789abcdef"


def load_diagnostic():
    if not SCRIPT.is_file():
        raise AssertionError("Stage-15 cross-family union wrapper does not exist")
    spec = importlib.util.spec_from_file_location("stage15_fixture_union", SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError("diagnostic module is not loadable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def row(producer, sequence, family, start, end, key="fixture-command.output"):
    return (
        f"command-interval\tv2\t{SESSION}\tsuite_all\t{producer}\t{sequence}"
        f"\t{key}\t{family}\t{start}\t{end}\n"
    )


class FakeRuntimeTelemetry:
    def __init__(self, started, declared, environment):
        self._started = started
        self._environment = environment

    def finish(self):
        return '{"schema":"control"}'


class Stage15FixtureCommandUnionTests(unittest.TestCase):
    def test_cross_family_union_merges_overlap_instead_of_summing_families(self):
        diagnostic = load_diagnostic()
        rows = [
            row("p11-1", 1, "shell", 0, 10_000_000_000),
            row("p11-1", 2, "python", 5_000_000_000, 15_000_000_000),
            row("p11-1", 3, "git", 20_000_000_000, 24_000_000_000),
            row("p11-1", 4, "validator", 21_000_000_000, 23_000_000_000),
            row("p11-1", 5, "other", 24_000_000_000, 25_000_000_000),
            row("p12-1", 1, "shell", 0, 12_000_000_000),
            row("p11-1", 6, "shell", 30_000_000_000, 31_000_000_000,
                key="fixture-command.status"),
        ]
        receipt = diagnostic.aggregate_output_rows(rows)
        self.assertEqual(receipt["raw_row_count"], 6)
        self.assertEqual(receipt["producer_count"], 2)
        self.assertEqual(receipt["cross_family_union_seconds"], 20.0)
        self.assertEqual(
            receipt["family_union_seconds"],
            {"git": 4.0, "other": 1.0, "python": 10.0, "shell": 12.0,
             "validator": 2.0},
        )
        self.assertLess(
            receipt["cross_family_union_seconds"],
            sum(receipt["family_union_seconds"].values()),
        )

    def test_raw_digest_is_stable_and_owned_phases_do_not_invent_constructor(self):
        diagnostic = load_diagnostic()
        rows = [
            row("p2-1", 2, "python", 5, 15),
            row("p1-1", 1, "shell", 0, 10),
        ]
        first = diagnostic.aggregate_output_rows(rows)
        second = diagnostic.aggregate_output_rows(reversed(rows))
        self.assertEqual(first["raw_rows_sha256"], second["raw_rows_sha256"])
        self.assertEqual(len(first["raw_rows_sha256"]), 64)
        self.assertEqual(first["owned_phase_buckets"]["constructor"], "not-observed")
        self.assertEqual(
            first["owned_phase_buckets"]["output"]["union_seconds"],
            first["cross_family_union_seconds"],
        )

    def test_runtime_captures_rows_before_cleanup_without_mutating_environment(self):
        diagnostic = load_diagnostic()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            metrics = root / "command-intervals"
            metrics.mkdir()
            (metrics / "interval-p11-1.metrics").write_text(
                row("p11-1", 1, "shell", 0, 10), encoding="utf-8"
            )
            trace = root / "trace.json"
            environment = dict(os.environ)
            environment.update(
                CODEXY_PROFILE_INTERVAL_METRICS_DIR=str(metrics),
                CODEXY_PROFILE_INTERVAL_SESSION=SESSION,
            )
            expected = dict(environment)
            runtime_type = diagnostic.instrument_runtime(
                FakeRuntimeTelemetry,
                trace,
                {"source_parity": "PASS"},
                SimpleNamespace(read_rows=lambda path, session: None),
            )
            runtime = runtime_type(time.perf_counter(), ["suite_all"], environment)
            self.assertEqual(runtime.finish(), '{"schema":"control"}')
            receipt = json.loads(trace.read_text())
        self.assertEqual(environment, expected)
        self.assertEqual(receipt["fixture_command_output"]["raw_row_count"], 1)
        self.assertEqual(receipt["source_parity"], "PASS")
        self.assertLessEqual(receipt["perturbation_limit_percent"], 0.05)

    def test_run_delegates_once_with_exact_argv_and_restores_parser_ceiling(self):
        diagnostic = load_diagnostic()
        profile = SimpleNamespace(RuntimeTelemetry=FakeRuntimeTelemetry)
        interval = SimpleNamespace(MAX_INTERVAL_NANOSECONDS=300_000_000_000)
        calls = []

        def profile_main():
            calls.append(list(diagnostic.sys.argv))
            runtime = profile.RuntimeTelemetry(time.perf_counter(), [], dict(os.environ))
            runtime.finish()
            return 17

        profile.main = profile_main
        with tempfile.TemporaryDirectory() as directory:
            trace = Path(directory) / "trace.json"
            with mock.patch.object(
                diagnostic, "load_profile_modules", return_value=(profile, interval)
            ), mock.patch.object(
                diagnostic, "parity_metadata", return_value={"source_parity": "PASS"}
            ):
                status = diagnostic.run(trace)
            receipt = json.loads(trace.read_text())
        self.assertEqual(status, 17)
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][1:], ["--windows", "--budget-seconds", "900.0"])
        self.assertIs(profile.RuntimeTelemetry, FakeRuntimeTelemetry)
        self.assertEqual(interval.MAX_INTERVAL_NANOSECONDS, 300_000_000_000)
        self.assertEqual(receipt["profile_exit"], 17)
        self.assertTrue(receipt["acceptance_300_seconds"])

    def test_workflows_ignore_only_all_diagnostic_paths_and_have_one_producer(self):
        diagnostic_module = load_diagnostic()
        parity = diagnostic_module.parity_metadata()
        self.assertEqual(parity["profile_sha256"], diagnostic_module.PROFILE_SHA256)
        self.assertEqual(parity["canonical_jobs_sha256"], EXPECTED_JOB_DIGEST)
        canonical = CANONICAL_WORKFLOW.read_text()
        expected_filter = """  pull_request:
    paths-ignore:
      - ".github/workflows/rust-test.yml"
      - ".github/workflows/stage15-windows-fixture-command-union.yml"
      - "scripts/stage15_windows_fixture_command_union.py"
      - "tests/test_stage15_windows_fixture_command_union.py"""
        self.assertIn(expected_filter, canonical)
        jobs = canonical[canonical.index("jobs:\n") :]
        self.assertEqual(hashlib.sha256(jobs.encode()).hexdigest(), EXPECTED_JOB_DIGEST)

        diagnostic = DIAGNOSTIC_WORKFLOW.read_text()
        self.assertIn(
            "branches: [codexy/526-stage15-fixture-command-union-diagnostic-recovery]",
            diagnostic,
        )
        self.assertNotIn("pull_request:", diagnostic)
        self.assertNotIn("workflow_dispatch:", diagnostic)
        self.assertNotIn("matrix:", diagnostic)
        self.assertNotIn("retry", diagnostic.casefold())
        self.assertEqual(diagnostic.count("runs-on: windows-latest"), 1)
        self.assertEqual(
            diagnostic.count("python scripts/stage15_windows_fixture_command_union.py"),
            1,
        )
        self.assertEqual(diagnostic.count("actions/upload-artifact@v7"), 1)


if __name__ == "__main__":
    unittest.main()
