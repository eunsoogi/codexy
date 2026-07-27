"""Canonical test-name and exact-coverage helpers for Rust profiling."""

from __future__ import annotations

from collections import Counter

SHOULD_PANIC_SUFFIX = " - should panic"


def canonical_name(name: str) -> str:
    return name.removesuffix(SHOULD_PANIC_SUFFIX)


def canonical_test_id(target: str, name: str) -> str:
    return f"{target}::{canonical_name(name)}"


def suite_all_assignments(tests: list[str], clusters: tuple[str, ...]) -> dict[str, set[str]]:
    assignments = {cluster: set() for cluster in clusters}
    for test in tests:
        if not test.startswith("suite_all::"):
            continue
        name = test.removeprefix("suite_all::")
        cluster = name.split("::", 1)[0]
        if cluster not in assignments:
            raise ValueError(f"unassigned suite_all test: {test}")
        assignments[cluster].add(canonical_test_id("suite_all", name))
    return assignments


def cluster_plan(
    inventory_tests: list[str],
    clusters: tuple[str, ...],
    cluster: str,
    candidates: list[str],
    selected: list[str],
) -> dict[str, list[str]]:
    if cluster not in clusters:
        raise ValueError(f"unknown suite_all cluster: {cluster}")
    expected = suite_all_assignments(inventory_tests, clusters)[cluster]
    candidate_ids = {canonical_test_id("suite_all", name) for name in candidates}
    selected_ids = {canonical_test_id("suite_all", name) for name in selected}
    exclusions = candidate_ids - expected
    if selected_ids != expected:
        raise ValueError(
            f"selected cluster tests differ from assigned set: missing={sorted(expected - selected_ids)} "
            f"extra={sorted(selected_ids - expected)}"
        )
    return {
        "expected": sorted(expected),
        "exclusions": sorted(test.removeprefix("suite_all::") for test in exclusions),
        "selected": sorted(selected_ids),
    }


def coverage_report(expected: list[str], actual: list[str]) -> dict[str, object]:
    expected_counts = Counter(expected)
    actual_counts = Counter(actual)
    missing = sorted((expected_counts - actual_counts).elements())
    extras = sorted((actual_counts - expected_counts).elements())
    duplicates = sorted(name for name, count in actual_counts.items() if count > expected_counts[name])
    return {
        "duplicates": duplicates,
        "missing": missing,
        "unexpected": extras,
        "pass": not missing and not extras,
    }
