"""Parse Cargo test inventories and observed test output."""

from __future__ import annotations

import re
import subprocess
import tomllib
from collections import Counter
from pathlib import Path

LIST_PATTERN = re.compile(r"^(?P<name>.+): (?:test|benchmark)$")
RUN_PATTERN = re.compile(r"^test (?P<name>.+) \.\.\. (?P<result>ok|FAILED|ignored)$")
RUNNING_BINARY_PATTERN = re.compile(r"^\s*Running .+ \((?P<binary>.+)\)$")


def listed_test_inventory_from_completed_binaries(
    root: Path, output: str
) -> tuple[tuple[Counter[str], set[str]], int]:
    tests: Counter[str] = Counter()
    binaries = dict(completed_test_binaries(root, output))
    targets = declared_test_targets(root) | set(binaries)
    status = 0
    for target in sorted(targets):
        binary = binaries.get(target) or compiled_test_binary(root, target)
        if binary is None:
            status = status or 1
            continue
        process = subprocess.run(
            [str(binary), "--list"],
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        status = status or process.returncode
        targets.add(target)
        for line in (process.stdout or "").splitlines():
            if match := LIST_PATTERN.match(line):
                tests[f"{target}::{canonical_test_name(match.group('name'))}"] += 1
    return (tests, targets), status


def declared_test_targets(root: Path) -> set[str]:
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    targets = {"lib"}
    targets.update(path.stem for path in (root / "src/bin").glob("*.rs"))
    targets.update(test["name"] for test in manifest.get("test", []))
    return targets


def compiled_test_binary(root: Path, target: str) -> Path | None:
    package = tomllib.loads((root / "Cargo.toml").read_text())["package"]["name"]
    stem = (package if target == "lib" else target).replace("-", "_")
    candidates = [
        path
        for path in (root / "target/debug/deps").glob(f"{stem}-*")
        if path.is_file() and path.suffix not in {".d", ".pdb"}
    ]
    return max(candidates, key=lambda path: path.stat().st_mtime, default=None)


def completed_test_binaries(root: Path, output: str) -> list[tuple[str, Path]]:
    binaries: list[tuple[str, Path]] = []
    current_target = None
    for line in output.splitlines():
        if "Running " not in line:
            continue
        current_target = target_name(line)
        if match := RUNNING_BINARY_PATTERN.match(line):
            binary = Path(match.group("binary"))
            binaries.append((current_target, binary if binary.is_absolute() else root / binary))
    return binaries


def parse_inventory(output: str) -> tuple[Counter[str], set[str]]:
    return parse_tests(output, LIST_PATTERN)


def observed_test_inventory(output: str) -> tuple[Counter[str], set[str]]:
    return parse_tests(output, RUN_PATTERN)


def observed_test_outcomes(output: str) -> Counter[str]:
    return Counter(
        match.group("result")
        for line in output.splitlines()
        if (match := RUN_PATTERN.match(line))
    )


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
