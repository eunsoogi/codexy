"""Parse Cargo test inventories, deadline target lifecycle, and process observations."""

from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from collections import Counter
from pathlib import Path

SCRIPT_DIRECTORY = str(Path(__file__).resolve().parent)
if SCRIPT_DIRECTORY not in sys.path:
    sys.path.insert(0, SCRIPT_DIRECTORY)

from profile_rust_targets import canonical_test_name, target_name

LIST_PATTERN = re.compile(r"^(?P<name>.+): (?:test|benchmark)$")
RUN_PATTERN = re.compile(r"^test (?P<name>.+) \.\.\. (?P<result>ok|FAILED|ignored)$")
RUN_START_PATTERN = re.compile(r"^test (?P<name>.+?) \.\.\. .+$")
RUNNING_NOTICE_PATTERN = re.compile(r"^test (?P<name>.+) has been running for over 60 seconds$")
RUNNING_BINARY_PATTERN = re.compile(r"^\s*Running .+ \((?P<binary>.+)\)$")
RESULT_SUMMARY_PATTERN = re.compile(r"^test result: (?:ok|FAILED)\.")
RESULT_COUNTS_PATTERN = re.compile(
    r"^test result: (?:ok|FAILED)\. (?P<passed>\d+) passed; (?P<failed>\d+) failed; (?P<ignored>\d+) ignored;"
)


def archive_fixture_nested_cargo_build_count(root: Path) -> int:
    helper = root / "tests/support/release_archive.rs"
    try:
        return helper.read_text().count('Command::new("cargo")')
    except OSError as error:
        sys.stderr.write(f"archive fixture helper is unreadable: {error}\n")
        raise SystemExit(1) from None


def listed_test_inventory_from_completed_binaries(
    root: Path, output: str, required_targets: set[str] | None = None
) -> tuple[tuple[Counter[str], set[str]], int]:
    tests: Counter[str] = Counter()
    binaries = dict(completed_test_binaries(root, output))
    targets = (required_targets if required_targets is not None else declared_test_targets(root)) | set(binaries)
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
    targets.update(declared_test_target_order(manifest))
    return targets


def declared_test_target_order(manifest: dict[str, object]) -> tuple[str, ...]:
    return tuple(test["name"] for test in manifest.get("test", []) if isinstance(test, dict))


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
    tests, targets, _ = observed_test_records(output)
    return tests, targets


def observed_test_outcomes(output: str) -> Counter[str]:
    _, _, outcomes = observed_test_records(output)
    return outcomes


def deadline_test_context(output: str) -> tuple[str | None, str | None, list[str], str | None, set[str]]:
    current = None
    pending = None
    active: set[str] = set()
    last_completed = None
    terminal = None
    observed_targets: set[str] = set()
    for line in output.splitlines():
        if "Running " in line:
            current = target_name(line)
            pending = None
            terminal = None
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
            last_completed = completed
            pending = None
    return current, terminal, sorted(active), last_completed, observed_targets


def deadline_report_lines(output: str, declared_targets: tuple[str, ...]) -> list[str]:
    last_target, terminal, active_tests, last_completed, observed_targets = deadline_test_context(output)
    next_target = (
        next((target for target in declared_targets[declared_targets.index(last_target) + 1 :] if target not in observed_targets), None)
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


def linux_cargo_descendants_snapshot(cargo_pid: int, limit: int = 16) -> list[dict[str, int | str]]:
    processes = {}
    for path in Path("/proc").glob("[0-9]*"):
        try:
            processes[int(path.name)] = (int(path.joinpath("stat").read_text().rsplit(")", 1)[1].split()[1]), path)
        except (IndexError, OSError, ValueError):
            continue
    descendants = {cargo_pid}
    while children := {pid for pid, (parent, _) in processes.items() if parent in descendants} - descendants:
        descendants.update(children)
    snapshot = []
    for pid in sorted(descendants - {cargo_pid})[:limit]:
        parent, path = processes[pid]
        try:
            command = path.joinpath("cmdline").read_bytes().replace(b"\0", b" ").decode("utf-8", "replace").strip()[:512]
        except OSError:
            continue
        snapshot.append({"pid": pid, "ppid": parent, "command": command or "not-observed"})
    return snapshot


def observed_test_records(output: str) -> tuple[Counter[str], set[str], Counter[str]]:
    current = None
    pending = None
    tests: Counter[str] = Counter()
    targets: set[str] = set()
    outcomes: Counter[str] = Counter()
    for line in output.splitlines():
        if "Running " in line:
            current = target_name(line)
            targets.add(current)
            pending = None
        elif current and (match := RUN_PATTERN.match(line)):
            pending = None
            record_observed_test(tests, outcomes, current, match.group("name"), match.group("result"))
        elif current and (match := RUN_START_PATTERN.match(line)):
            pending = match.group("name")
        elif current and pending and line in {"ok", "FAILED", "ignored"}:
            record_observed_test(tests, outcomes, current, pending, line)
            pending = None
        elif current and pending:
            if match := RESULT_COUNTS_PATTERN.match(line):
                observed = sum(count for name, count in tests.items() if name.startswith(f"{current}::"))
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
