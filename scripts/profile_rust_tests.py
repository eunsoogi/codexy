#!/usr/bin/env python3
"""Profile one registered Rust acceptance workload and account for its coverage."""

from __future__ import annotations

import argparse
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
    declared_test_target_order,
    declared_test_targets,
    deadline_report_lines,
    listed_test_inventory_from_completed_binaries,
    observed_test_inventory,
    observed_test_outcomes,
)
from profile_rust_archive_accounting import emit_receipt_report, receipt_environment
from profile_rust_contract import (
    BUDGET_SECONDS,
    COMPILE_PATTERN,
    MINIMUM_PASSED_TESTS,
    REQUIRED_JOB_TIMEOUT_MINUTES,
    RESULT_PATTERN,
    WORKLOAD,
)
from profile_rust_lifecycle import run_workload as lifecycle_run_workload
from profile_rust_output import flush_output, observe_first_line, replay_output
from profile_rust_receipts import SCHEMA, digest, write
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
    """Compatibility façade that keeps legacy probes patchable while using one lifecycle."""
    import profile_rust_lifecycle as lifecycle

    for name in (
        "json",
        "os",
        "subprocess",
        "sys",
        "tempfile",
        "threading",
        "time",
        "RuntimeTelemetry",
        "stop_workload",
        "WindowsJob",
        "configure_metrics",
        "telemetry",
        "configure_windows_test_runner",
        "isolated_windows_test_root",
        "launch_windows_workload",
        "receipt_environment",
        "observe_first_line",
        "replay_output",
        "flush_output",
    ):
        setattr(lifecycle, name, globals()[name])
    return lifecycle_run_workload(
        root, budget_seconds, windows, workload or WORKLOAD, declared_targets
    )


def receipt_head(root: Path) -> str:
    return subprocess.check_output(
        ("git", "rev-parse", "HEAD"), cwd=root, text=True
    ).strip()


def receipt_index_tree(root: Path) -> str:
    return subprocess.check_output(("git", "write-tree"), cwd=root, text=True).strip()


def runtime_package_root(root: Path) -> Path:
    root = root.resolve()
    nested_runtime = root / "packages" / "codexy-runtime"
    if nested_runtime.joinpath("Cargo.toml").is_file():
        return nested_runtime
    if root.joinpath("Cargo.toml").is_file():
        return root
    raise ValueError(
        "--root must name a Rust package root or a repository containing "
        "packages/codexy-runtime"
    )


def github_receipt_provenance() -> tuple[int, int]:
    values = tuple(
        os.environ.get(name) for name in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")
    )
    if any(
        value is None or not value.isascii() or not value.isdecimal() or int(value) < 1
        for value in values
    ):
        raise ValueError(
            "GITHUB_RUN_ID and GITHUB_RUN_ATTEMPT must be positive integers"
        )
    return int(values[0]), int(values[1])


def print_phases(
    arguments: argparse.Namespace,
    root: Path,
    output: str,
    status: int,
    phases: dict[str, float | str | Path],
    total_elapsed: float,
) -> None:
    for name, value in (
        ("child-status", status),
        ("windows-job-active-zero", phases["windows-job-active-zero"]),
        ("cargo-root-status", phases["cargo-root-status"]),
        ("windows-job-pids-json", phases["windows-job-pids-json"]),
        ("windows-job-images-json", phases["windows-job-images-json"]),
    ):
        print(f"{name}\t{value}")
    if status == 124:
        manifest = __import__("tomllib").loads((root / "Cargo.toml").read_text())
        print(
            *deadline_report_lines(output, declared_test_target_order(manifest)),
            f"deadline-linux-cargo-descendants-json\t{phases['linux-cargo-descendants-json']}",
            sep="\n",
        )
    for phase in ("workload", "capture", "replay", "inventory", "accounting"):
        print(f"phase-{phase}-seconds\t{phases[f'{phase}-seconds']:.3f}")
    print(f"total-seconds\t{total_elapsed:.3f}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "packages/codexy-runtime",
    )
    parser.add_argument("--workflow-file", type=Path)
    parser.add_argument("--budget-seconds", type=float, default=BUDGET_SECONDS)
    parser.add_argument("--verify-coverage", action="store_true")
    parser.add_argument("--windows", action="store_true")
    parser.add_argument("--shard")
    parser.add_argument("--receipt", type=Path)
    parser.add_argument("--aggregate-receipts", type=Path)
    parser.add_argument("--aggregate-platform", choices=("posix", "windows"))
    arguments = parser.parse_args()
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
    receipt_path, started_epoch, head, index_tree = (
        (arguments.receipt, time.time(), receipt_head(root), receipt_index_tree(root))
        if spec
        else (None, 0.0, "", "")
    )
    try:
        run_id, run_attempt = (
            github_receipt_provenance() if receipt_path is not None else (0, 0)
        )
    except ValueError as error:
        parser.error(str(error))
    if receipt_path is not None:
        receipt_path.parent.mkdir(parents=True, exist_ok=True)
        write(
            receipt_path,
            {
                "schema": SCHEMA,
                "state": "PENDING",
                "shard": spec.name,
                "platform": "windows" if arguments.windows else "posix",
                "argv": workload,
                "head": head,
                "index_tree": index_tree,
                "run_id": run_id,
                "run_attempt": run_attempt,
                "physical_targets": sorted(expected_owned_targets),
                "started": started_epoch,
            },
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
    print(f"command\t{' '.join(workload)}")
    print("workload-invocations\t1")
    if arguments.verify_coverage:
        print(
            f"compiled-targets\t{len(listed_targets)}\t{'PASS' if targets_pass else 'FAIL'}"
        )
    print(
        f"tests\t{passed} passed\t{failed} failed\t{ignored} ignored\t{'PASS' if tests_pass else 'FAIL'}"
    )
    print(
        f"archive-fixture-nested-cargo-builds\t{archive_fixture_nested_cargo_builds}\t{'PASS' if archive_pass else 'FAIL'}"
    )
    if phases.get("fixture-telemetry-json") is not None:
        print(f"fixture-telemetry-json\t{phases['fixture-telemetry-json']}")
    if phases.get("workload-receipt-json"):
        print(f"workload-receipt-json\t{phases['workload-receipt-json']}")
    if arguments.windows:
        print(f"windows-telemetry-json\t{phases['windows-telemetry-json']}")
        print(
            f"windows-temp-cleanup-receipt-json\t{phases['windows-temp-cleanup-receipt-json']}"
        )
    emit_receipt_report(phases.get("archive-inspector-receipt-lines"))
    print_phases(arguments, root, output, status, phases, total_elapsed)
    print(f"compile-seconds\t{compile_seconds:.3f}")
    print(f"execution-seconds\t{max(0.0, elapsed - compile_seconds):.3f}")
    if arguments.verify_coverage:
        print(
            f"coverage-tests\t{sum(expected_tests.values())}\t{sum(observed_tests.values())}\t{'PASS' if coverage_pass else 'FAIL'}"
        )
        print(f"coverage-missing\t{len(missing)}")
        print(f"coverage-duplicates\t{len(duplicate)}")
        print(f"coverage-extra\t{len(extra)}")
        print(f"coverage-duplicate-or-extra\t{len(duplicate) + len(extra)}")
        print(
            f"coverage-targets\t{len(listed_targets)}\t{len(observed_targets)}\t{'PASS' if targets_pass else 'FAIL'}"
        )
        print(
            "coverage-report-json\t"
            + json.dumps(
                {
                    "expected_targets": sorted(listed_targets),
                    "observed_targets": sorted(observed_targets),
                    "missing": missing,
                    "duplicates": duplicate,
                    "extra": extra,
                    "inventory_status": inventory_status,
                },
                sort_keys=True,
            )
        )
    print(
        f"budget-seconds\t{arguments.budget_seconds:.3f}\t{'PASS' if budget_pass else 'FAIL'}"
    )
    if receipt_path is not None:
        receipt = {
            "schema": SCHEMA,
            "state": "PASS" if success and elapsed <= 270 else "FAIL",
            "shard": spec.name,
            "platform": "windows" if arguments.windows else "posix",
            "argv": workload,
            "head": head,
            "index_tree": index_tree,
            "run_id": run_id,
            "run_attempt": run_attempt,
            "status": status,
            "failed": failed,
            "ignored": ignored,
            "elapsed": elapsed,
            "tests": sorted(observed_tests.elements()),
            "digest": digest(observed_tests),
            "listed_digest": digest(expected_tests),
            "physical_targets": sorted(listed_targets),
            "started": phases.get("profiler-started-epoch", started_epoch),
            "finished": time.time(),
            "workload_receipt": phases.get("workload-receipt-json"),
        }
        write(receipt_path, receipt)
        print(
            f"shard\t{spec.name}\t{receipt['state']}\t{sum(expected_tests.values())}\t{receipt['digest']}"
        )
        success = success and receipt["state"] == "PASS"
    print(f"result\t{'PASS' if success else 'FAIL'}")
    return 0 if success else status or 1


if __name__ == "__main__":
    raise SystemExit(main())
