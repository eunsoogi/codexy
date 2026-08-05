"""Registered Rust acceptance shards and fail-closed receipt aggregation."""
from __future__ import annotations

import json
import math
import subprocess
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

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
PLATFORM_COUNTS = {"posix": 2018, "windows": 1912}
TOPOLOGY_AUTHORITY = "PR #516 maintainer authority supersedes only #526's monolithic-all-targets and no-shard topology clauses; every other #526 constraint remains binding."


def shard_spec(name: str | None) -> WorkloadSpec | None:
    if name is None:
        return None
    return next((spec for spec in SPECS if spec.name == name), None)


def aggregate(directory: Path, root: Path, platform_only: str | None = None) -> int:
    try:
        receipts = load(directory)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"aggregate-receipts\t0\tFAIL\t{error}")
        return 1
    platforms = {item.get("platform") for item in receipts}
    selected = {"posix"} if platform_only == "posix" else set(PLATFORM_COUNTS)
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
    def valid_timing(item: dict[str, object]) -> bool:
        values = (item.get("elapsed"), item.get("started"), item.get("finished"))
        return all(isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0 for value in values) and item["finished"] >= item["started"]
    for item in receipts:
        platform, shard = item.get("platform"), item.get("shard")
        item_tests = Counter(item.get("tests", []))
        if platform not in selected or shard not in SHARDS or item.get("state") != "PASS" or not valid_timing(item) or item.get("argv") not in (list(SHARDS[shard]), SHARDS[shard]) or item.get("head") != head or item.get("index_tree") != index_tree or item.get("digest") != digest(item_tests) or item.get("digest") != item.get("listed_digest") or set(item.get("physical_targets", [])) != owned_targets(expected_targets, shard):
            receipt_valid = False
            continue
        tests[platform].update(item_tests)
        targets[platform].update(item.get("physical_targets", []))
    duplicates = sum(sum(count - 1 for count in values.values() if count > 1) for values in tests.values())
    windows = {platform: max((float(item.get("finished", 0)) for item in receipts if item.get("platform") == platform and valid_timing(item)), default=0) - min((float(item.get("started", 0)) for item in receipts if item.get("platform") == platform and valid_timing(item)), default=0) for platform in selected}
    valid = receipt_valid and platforms == selected and found == expected and len(receipts) == len(expected) and duplicates == 0 and all(targets[platform] == expected_targets and sum(values.values()) == PLATFORM_COUNTS[platform] and windows[platform] < 300 for platform, values in tests.items()) and all(float(item.get("elapsed", 271)) <= 270 for item in receipts)
    print(f"aggregate-receipts\t{len(receipts)}\t{'PASS' if valid else 'FAIL'}")
    for platform, values in tests.items(): print(f"aggregate-{platform}\t{sum(values.values())}\t{digest(values)}")
    return 0 if valid else 1


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
