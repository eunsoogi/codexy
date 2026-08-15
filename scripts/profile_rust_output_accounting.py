"""Interpret Cargo test output for profile accounting."""

from __future__ import annotations

from collections import Counter

from profile_rust_targets import canonical_test_name, target_name
from profile_rust_test_output import (
    LIST_PATTERN,
    RESULT_COUNTS_PATTERN,
    RESULT_SUMMARY_PATTERN,
    RUNNING_NOTICE_PATTERN,
    RUN_PATTERN,
    RUN_START_PATTERN,
)


def parse_inventory(output: str) -> tuple[Counter[str], set[str]]:
    return parse_tests(output, LIST_PATTERN)


def observed_test_inventory(output: str) -> tuple[Counter[str], set[str]]:
    tests, targets, _ = observed_test_records(output)
    return tests, targets


def observed_test_outcomes(output: str) -> Counter[str]:
    _, _, outcomes = observed_test_records(output)
    return outcomes


def deadline_test_context(
    output: str,
) -> tuple[str | None, str | None, list[str], str | None, set[str]]:
    current = pending = last_completed = terminal = None
    active: set[str] = set()
    observed_targets: set[str] = set()
    for line in output.splitlines():
        if "Running " in line:
            current, pending, terminal = target_name(line), None, None
            observed_targets.add(current)
        elif current and (match := RUN_PATTERN.match(line)):
            pending = None
            completed = f"{current}::{canonical_test_name(match.group('name'))}"
            active.discard(completed)
            last_completed = completed
        elif current and RESULT_SUMMARY_PATTERN.match(line):
            terminal = current
        elif current and (match := RUNNING_NOTICE_PATTERN.match(line)):
            pending = None
            active.add(f"{current}::{canonical_test_name(match.group('name'))}")
        elif current and (match := RUN_START_PATTERN.match(line)):
            pending = match.group("name")
        elif current and pending and line in {"ok", "FAILED", "ignored"}:
            completed = f"{current}::{canonical_test_name(pending)}"
            active.discard(completed)
            last_completed, pending = completed, None
    return current, terminal, sorted(active), last_completed, observed_targets


def deadline_report_lines(output: str, declared_targets: tuple[str, ...]) -> list[str]:
    last_target, terminal, active_tests, last_completed, observed_targets = (
        deadline_test_context(output)
    )
    next_target = (
        next(
            (
                target
                for target in declared_targets[
                    declared_targets.index(last_target) + 1 :
                ]
                if target not in observed_targets
            ),
            None,
        )
        if last_target in declared_targets
        else None
    )
    return [
        f"deadline-last-running-target\t{last_target or 'not-observed'}",
        f"deadline-terminal-target\t{terminal or 'not-observed'}",
        f"deadline-next-target-not-started\t{next_target or 'not-observed'}",
        *(f"deadline-active-test\t{test}" for test in active_tests),
        f"deadline-last-completed-test\t{last_completed or 'not-observed'}",
    ]


def observed_test_records(output: str) -> tuple[Counter[str], set[str], Counter[str]]:
    current = pending = None
    tests: Counter[str] = Counter()
    targets: set[str] = set()
    outcomes: Counter[str] = Counter()
    for line in output.splitlines():
        if "Running " in line:
            current, pending = target_name(line), None
            targets.add(current)
        elif current and (match := RUN_PATTERN.match(line)):
            pending = None
            record_observed_test(
                tests, outcomes, current, match.group("name"), match.group("result")
            )
        elif current and (match := RUN_START_PATTERN.match(line)):
            pending = match.group("name")
        elif current and pending and line in {"ok", "FAILED", "ignored"}:
            record_observed_test(tests, outcomes, current, pending, line)
            pending = None
        elif current and pending:
            if match := RESULT_COUNTS_PATTERN.match(line):
                observed = sum(
                    count
                    for name, count in tests.items()
                    if name.startswith(f"{current}::")
                )
                if (
                    match.group("failed") == "0"
                    and match.group("ignored") == "0"
                    and int(match.group("passed")) == observed + 1
                ):
                    record_observed_test(tests, outcomes, current, pending, "ok")
            pending = None
    return tests, targets, outcomes


def record_observed_test(
    tests: Counter[str], outcomes: Counter[str], target: str, name: str, result: str
) -> None:
    tests[f"{target}::{canonical_test_name(name)}"] += 1
    outcomes[result] += 1


def parse_tests(output: str, pattern: object) -> tuple[Counter[str], set[str]]:
    current = None
    tests: Counter[str] = Counter()
    targets: set[str] = set()
    for line in output.splitlines():
        if "Running " in line:
            current = target_name(line)
            targets.add(current)
        elif current and (match := pattern.match(line)):
            tests[f"{current}::{canonical_test_name(match.group('name'))}"] += 1
    return tests, targets
