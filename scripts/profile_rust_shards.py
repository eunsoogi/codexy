"""Registered Rust acceptance shards and fail-closed receipt aggregation."""
from __future__ import annotations

import math
import subprocess
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from profile_rust_accounting import declared_test_targets
from profile_rust_receipts import digest, load
from profile_rust_targets import canonical_test_name

@dataclass(frozen=True)
class WorkloadSpec:
    name: str
    argv: tuple[str, ...]


SPECS = tuple(
    WorkloadSpec(name, argv) for name, argv in (
        ("support", ("cargo", "test", "--locked", "--lib", "--bins", "--test", "suite_support")),
        ("agent", ("cargo", "test", "--locked", "--test", "suite_agent")),
        ("child", ("cargo", "test", "--locked", "--test", "suite_child")),
        ("orchestration", ("cargo", "test", "--locked", "--test", "suite_orchestration")),
        ("governance", ("cargo", "test", "--locked", "--test", "suite_governance")),
        ("system", ("cargo", "test", "--locked", "--test", "suite_system")),
        ("archive", ("cargo", "test", "--locked", "--test", "suite_archive")),
    )
)
SHARDS = {spec.name: spec.argv for spec in SPECS}
CANONICAL = {f"suite_{name}": "suite_all" for name in SHARDS if name != "archive"}
CANONICAL["suite_archive"] = "suite_archive"
PLATFORMS = frozenset(("posix", "windows"))
TOPOLOGY_AUTHORITY = "PR #516 maintainer authority supersedes only #526's monolithic-all-targets and no-shard topology clauses; every other #526 constraint remains binding."


def shard_spec(name: str | None) -> WorkloadSpec | None:
    if name is None:
        return None
    return next((spec for spec in SPECS if spec.name == name), None)


def valid_provenance(item: dict[str, object]) -> bool:
    values = (item.get("run_id"), item.get("run_attempt"))
    return all(isinstance(value, int) and not isinstance(value, bool) and value > 0 for value in values)


def valid_process_status(item: dict[str, object]) -> bool:
    value = item.get("status")
    return isinstance(value, int) and not isinstance(value, bool) and value == 0


def valid_timing(item: dict[str, object]) -> bool:
    values = (item.get("elapsed"), item.get("started"), item.get("finished"))
    numeric = all(
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(value)
        and value >= 0
        for value in values
    )
    return numeric and item["finished"] >= item["started"]


def receipt_matches(
    item: dict[str, object],
    selected: set[str],
    head: str,
    index_tree: str,
    expected_targets: set[str],
) -> bool:
    platform, shard = item.get("platform"), item.get("shard")
    item_tests = Counter(item.get("tests", []))
    return (
        platform in selected
        and shard in SHARDS
        and item.get("state") == "PASS"
        and valid_process_status(item)
        and valid_timing(item)
        and valid_provenance(item)
        and item.get("argv") in (list(SHARDS[shard]), SHARDS[shard])
        and item.get("head") == head
        and item.get("index_tree") == index_tree
        and item.get("digest") == digest(item_tests)
        and item.get("digest") == item.get("listed_digest")
        and set(item.get("physical_targets", [])) == owned_targets(expected_targets, shard)
    )


def aggregate(directory: Path, root: Path, platform_only: str | None = None) -> int:
    try:
        receipts = load(directory)
    except (OSError, ValueError) as error:
        print(f"aggregate-receipts\t0\tFAIL\t{error}")
        return 1
    platforms = {item.get("platform") for item in receipts}
    selected = {"posix"} if platform_only == "posix" else PLATFORMS
    if platform_only not in {None, "posix"}:
        print(f"aggregate-receipts\t0\tFAIL\tlocal platform aggregate must be posix")
        return 1
    head = subprocess.check_output(("git", "rev-parse", "HEAD"), cwd=root, text=True).strip()
    index_tree = subprocess.check_output(("git", "write-tree"), cwd=root, text=True).strip()
    expected = {(platform, shard) for platform in selected for shard in SHARDS}
    found = {(item.get("platform"), item.get("shard")) for item in receipts}
    tests: dict[str, Counter[str]] = {platform: Counter() for platform in selected}
    targets: dict[str, set[str]] = {platform: set() for platform in selected}
    receipt_valid, expected_targets = True, declared_test_targets(root)
    for item in receipts:
        platform, shard = item.get("platform"), item.get("shard")
        item_tests = Counter(item.get("tests", []))
        if not receipt_matches(item, selected, head, index_tree, expected_targets):
            receipt_valid = False
            continue
        tests[platform].update(item_tests)
        targets[platform].update(item.get("physical_targets", []))
    duplicates = sum(sum(count - 1 for count in values.values() if count > 1) for values in tests.values())
    one_receipt_per_run = len(
        {(item.get("run_id"), item.get("run_attempt")) for item in receipts}
    ) == 1
    targets_match = all(
        targets[platform] == expected_targets
        and provenance_windows_within_budget(receipts, platform, valid_timing)
        for platform in tests
    )
    elapsed_within_budget = all(float(item.get("elapsed", 271)) <= 270 for item in receipts)
    valid = (
        receipt_valid
        and platforms == selected
        and found == expected
        and len(receipts) == len(expected)
        and duplicates == 0
        and one_receipt_per_run
        and targets_match
        and elapsed_within_budget
    )
    print(f"aggregate-receipts\t{len(receipts)}\t{'PASS' if valid else 'FAIL'}")
    for platform, values in tests.items():
        print(f"aggregate-{platform}\t{sum(values.values())}\t{digest(values)}")
    return 0 if valid else 1


def provenance_windows_within_budget(receipts: list[dict[str, object]], platform: str, valid_timing: Callable[[dict[str, object]], bool]) -> bool:
    attempts: dict[int, list[tuple[float, float]]] = {}
    for item in receipts:
        if item.get("platform") != platform:
            continue
        if not valid_timing(item) or not valid_provenance(item):
            return False
        attempts.setdefault(item["run_attempt"], []).append((float(item["started"]), float(item["finished"])))
    return bool(attempts) and all(max(finished for _, finished in spans) - min(started for started, _ in spans) < 300 for spans in attempts.values())


def owned_targets(targets: set[str], shard: str) -> set[str]:
    return {f"suite_{shard}"} if shard != "support" else targets - {f"suite_{name}" for name in SHARDS if name != "support"}


def canonical_tests(output: str) -> Counter[str]:
    target, tests = "", Counter()
    for line in output.splitlines():
        if "Running " in line:
            target = next((value for value in CANONICAL if value in line), "lib" if "src/lib.rs" in line else target)
        elif target and line.startswith("test ") and " ... ok" in line:
            tests[f"{CANONICAL.get(target, target)}::{canonical_test_name(line[5:].split(' ...', 1)[0])}"] += 1
    return tests


def canonical_inventory(tests: Counter[str]) -> Counter[str]:
    canonical = Counter()
    for identifier, count in tests.items():
        target, name = identifier.split("::", 1)
        canonical[f"{CANONICAL.get(target, target)}::{name}"] += count
    return canonical
