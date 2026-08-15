"""Terminal profile summary emission."""

import json
from collections import Counter
from pathlib import Path

from profile_rust_archive_accounting import emit_receipt_report
from profile_rust_reporting import print_phases


def emit_summary(
    arguments,
    root: Path,
    output: str,
    status: int,
    phases,
    total_elapsed: float,
    workload,
    listed_targets,
    targets_pass: bool,
    passed: int,
    failed: int,
    ignored: int,
    tests_pass: bool,
    archive_fixture_nested_cargo_builds: int,
    archive_pass: bool,
    compile_seconds: float,
    elapsed: float,
    coverage_pass: bool,
    expected_tests: Counter[str],
    observed_tests: Counter[str],
    observed_targets,
    missing,
    duplicate,
    extra,
    inventory_status: int,
) -> None:
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
