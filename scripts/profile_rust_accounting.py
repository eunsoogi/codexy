"""Parse Cargo test inventories and observed test output."""

from __future__ import annotations

import re
import subprocess
from collections import Counter
from pathlib import Path

LIST_PATTERN = re.compile(r"^(?P<name>.+): (?:test|benchmark)$")
RUN_PATTERN = re.compile(r"^test (?P<name>.+) \.\.\. (?P<result>ok|FAILED|ignored)$")


def listed_test_inventory(
    root: Path, workload: tuple[str, ...]
) -> tuple[Counter[str], set[str], int]:
    process = subprocess.run(
        [*workload, "--", "--list"],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    )
    return parse_inventory(process.stdout or ""), process.returncode


def parse_inventory(output: str) -> tuple[Counter[str], set[str]]:
    return parse_tests(output, LIST_PATTERN)


def observed_test_inventory(output: str) -> tuple[Counter[str], set[str]]:
    return parse_tests(output, RUN_PATTERN)


def parse_tests(
    output: str, pattern: re.Pattern[str]
) -> tuple[Counter[str], set[str]]:
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


def target_name(line: str) -> str:
    path = line.replace("\\", "/")
    if "tests/suites/all.rs" in path:
        return "suite_all"
    if "tests/suites/archive.rs" in path:
        return "suite_archive"
    if "src/lib.rs" in path:
        return "lib"
    if "src/bin/" in path:
        return Path(path.split("src/bin/", 1)[1].split(" ", 1)[0]).stem
    source = path.split("Running ", 1)[-1].split(" (", 1)[0]
    return f"other:{source}"


def canonical_test_name(name: str) -> str:
    return name.removesuffix(" - should panic")
