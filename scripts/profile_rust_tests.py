#!/usr/bin/env python3
"""Profile one registered Rust acceptance workload and account for its coverage."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import threading
import time
from collections import Counter
from pathlib import Path

from profile_rust_accounting import (
    archive_fixture_nested_cargo_build_count,
    declared_test_targets,
    listed_test_inventory_from_completed_binaries,
    observed_test_inventory,
    observed_test_outcomes,
)
from profile_rust_archive_accounting import receipt_environment
from profile_rust_contract import (
    BUDGET_SECONDS,
    COMPILE_PATTERN,
    MINIMUM_PASSED_TESTS,
    REQUIRED_JOB_TIMEOUT_MINUTES,
    RESULT_PATTERN,
    WORKLOAD,
)
from profile_rust_compat import run_patchable
from profile_rust_output import flush_output, observe_first_line, replay_output
from profile_rust_cli import parse_arguments
from profile_rust_receipt_lifecycle import begin_receipt
from profile_rust_receipt_finish import finish_receipt
from profile_rust_receipts import digest, write
from profile_rust_reporting import (
    runtime_package_root,
)
from profile_rust_summary import emit_summary
from profile_rust_runtime_telemetry import RuntimeTelemetry, stop_workload
from profile_rust_shards import (
    aggregate,
    canonical_inventory,
    owned_targets,
    shard_spec,
)
from profile_rust_telemetry import configure_metrics, telemetry
from profile_rust_windows import (
    WindowsJob,
    configure_windows_test_runner,
    isolated_windows_test_root,
    launch_windows_workload,
)
from profile_rust_workflow import enforce_workflow_contract


def run_workload(
    root: Path | None,
    budget_seconds: float,
    windows: bool = False,
    workload: tuple[str, ...] | None = None,
    declared_targets: set[str] | None = None,
) -> tuple[str, float, int, dict[str, float | str | Path]]:
    return run_patchable(
        globals(), root, budget_seconds, windows, workload, declared_targets
    )


def main() -> int:
    arguments = parse_arguments(__doc__ or "")
    try:
        root = runtime_package_root(arguments.root)
    except ValueError as error:
        parser.error(str(error))
    repository_root = (
        root.parent.parent
        if root.name == "codexy-runtime" and root.parent.name == "packages"
        else root
    )
    if arguments.aggregate_receipts:
        return aggregate(
            arguments.aggregate_receipts,
            repository_root,
            arguments.aggregate_platform,
        )
    spec = shard_spec(arguments.shard) if arguments.shard else None
    if arguments.shard and spec is None:
        parser.error("--shard must name a registered workload")
    if spec is not None and arguments.receipt is None:
        parser.error("--receipt is required with --shard")
    arguments.verify_coverage = (
        arguments.verify_coverage or arguments.windows or spec is not None
    )
    workload = spec.argv if spec else WORKLOAD
    expected_owned_targets = (
        owned_targets(declared_test_targets(root), spec.name) if spec else None
    )
    workflow = (
        arguments.workflow_file or repository_root / ".github/workflows/rust-test.yml"
    )
    enforce_workflow_contract(workflow, REQUIRED_JOB_TIMEOUT_MINUTES, WORKLOAD)
    receipt_path, started_epoch, head, index_tree, run_id, run_attempt = begin_receipt(
        arguments, root, spec, workload, expected_owned_targets
    )
    archive_fixture_nested_cargo_builds = archive_fixture_nested_cargo_build_count(root)
    started = time.perf_counter()
    workload_arguments = (
        (
            root,
            arguments.budget_seconds,
            arguments.windows,
            workload,
            expected_owned_targets,
        )
        if spec
        else (
            (root, arguments.budget_seconds, True)
            if arguments.windows
            else (root, arguments.budget_seconds)
        )
    )
    output, elapsed, status, phases = run_workload(*workload_arguments)
    expected_tests: Counter[str] = Counter()
    listed_targets: set[str] = set()
    inventory_status = 0
    inventory_started = time.perf_counter() if arguments.verify_coverage else None
    if arguments.verify_coverage:
        (expected_tests, listed_targets), inventory_status = (
            listed_test_inventory_from_completed_binaries(
                root, output, expected_owned_targets
            )
        )
    phases["inventory-seconds"] = (
        time.perf_counter() - inventory_started
        if inventory_started is not None
        else 0.0
    )
    accounting_started = time.perf_counter()
    observed_tests, observed_targets = observed_test_inventory(output)
    outcomes = observed_test_outcomes(output)
    if not outcomes and not arguments.verify_coverage:
        for passed, failed, ignored in RESULT_PATTERN.findall(output):
            outcomes.update(
                {"ok": int(passed), "FAILED": int(failed), "ignored": int(ignored)}
            )
    if spec is not None:
        expected_tests, observed_tests = (
            canonical_inventory(expected_tests),
            canonical_inventory(observed_tests),
        )
        listed_targets, observed_targets = set(listed_targets), set(observed_targets)
    passed, failed, ignored = outcomes["ok"], outcomes["FAILED"], outcomes["ignored"]
    minimum = sum(expected_tests.values()) if spec else MINIMUM_PASSED_TESTS
    tests_pass = passed >= minimum and failed == ignored == 0
    missing = sorted((expected_tests - observed_tests).elements())
    unexpected = observed_tests - expected_tests
    duplicate = sorted(
        name
        for name, count in unexpected.items()
        if expected_tests[name] > 0
        for _ in range(count)
    )
    extra = sorted(
        name
        for name, count in unexpected.items()
        if expected_tests[name] == 0
        for _ in range(count)
    )
    targets_pass = (
        bool(listed_targets)
        and listed_targets == observed_targets
        and (expected_owned_targets is None or listed_targets == expected_owned_targets)
    )
    coverage_pass = not arguments.verify_coverage or (
        inventory_status == 0
        and expected_tests
        and not missing
        and not duplicate
        and not extra
        and targets_pass
    )
    total_elapsed = time.perf_counter() - started
    phases["accounting-seconds"] = time.perf_counter() - accounting_started
    budget_pass = total_elapsed <= arguments.budget_seconds
    archive_pass = archive_fixture_nested_cargo_builds == 0
    compile_matches = COMPILE_PATTERN.findall(output)
    compile_seconds = (
        int(compile_matches[-1][0] or 0) * 60 + float(compile_matches[-1][1])
        if compile_matches
        else 0.0
    )
    success = (
        status == 0 and tests_pass and archive_pass and budget_pass and coverage_pass
    )
    emit_summary(
        arguments,
        root,
        output,
        status,
        phases,
        total_elapsed,
        workload,
        listed_targets,
        targets_pass,
        passed,
        failed,
        ignored,
        tests_pass,
        archive_fixture_nested_cargo_builds,
        archive_pass,
        compile_seconds,
        elapsed,
        coverage_pass,
        expected_tests,
        observed_tests,
        observed_targets,
        missing,
        duplicate,
        extra,
        inventory_status,
    )
    success = finish_receipt(
        receipt_path,
        spec,
        arguments,
        workload,
        head,
        index_tree,
        run_id,
        run_attempt,
        status,
        failed,
        ignored,
        elapsed,
        observed_tests,
        expected_tests,
        listed_targets,
        phases,
        started_epoch,
        success,
    )
    print(f"result\t{'PASS' if success else 'FAIL'}")
    return 0 if success else status or 1


if __name__ == "__main__":
    raise SystemExit(main())
