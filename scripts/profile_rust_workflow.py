"""Validate that the Rust workflow delegates the exact workload once."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from profile_rust_shell import invocation_count

WORKFLOW_KEY_PATTERN = re.compile(r"^(?P<key>[^:#][^:]*):(?P<value>.*)$")
WINDOWS_PREREQUISITE = "scripts/install-windows-test-prerequisites.ps1"
WINDOWS_TOOLCHAIN_BOOTSTRAP = "rustup toolchain install"
WINDOWS_GATE = "python scripts/profile-rust-tests --windows"
WINDOWS_JOB_TIMEOUT_MINUTES = 20
WorkflowStep = tuple[str, frozenset[str]]


def yaml_mapping_entry(line: str) -> tuple[str, str] | None:
    stripped = line.strip()
    if not stripped or stripped.startswith("#") or stripped.startswith("-"):
        return None
    match = WORKFLOW_KEY_PATTERN.match(stripped)
    if match is None:
        return None
    return yaml_scalar_value(match.group("key").strip()), yaml_value_without_comment(match.group("value")).strip()


def yaml_value_without_comment(value: str) -> str:
    quote: str | None = None
    for index, character in enumerate(value):
        if character in "'\"":
            quote = None if character == quote else character if quote is None else quote
        elif character == "#" and quote is None and (index == 0 or value[index - 1].isspace()):
            return value[:index]
    return value


def step_run_command(line: str) -> str | None:
    stripped = line.strip()
    if stripped.startswith("-"):
        stripped = stripped[1:].lstrip()
    entry = yaml_mapping_entry(stripped)
    return yaml_scalar_value(entry[1]) if entry is not None and entry[0] == "run" else None


def yaml_scalar_value(value: str) -> str:
    if len(value) < 2 or value[0] != value[-1]:
        return value
    if value[0] == "'":
        return value[1:-1].replace("''", "'")
    if value[0] != '"':
        return value
    try:
        decoded = json.loads(value)
    except json.JSONDecodeError:
        return value[1:-1]
    return decoded if isinstance(decoded, str) else value


def workflow_jobs(source: str) -> dict[str, list[str]]:
    jobs: dict[str, list[str]] = {}
    current_job: str | None = None
    in_jobs = False
    for line in source.splitlines():
        if not line.strip():
            if in_jobs and current_job is not None:
                jobs[current_job].append("")
            continue
        if line.lstrip().startswith("#"):
            continue
        indentation = len(line) - len(line.lstrip(" "))
        entry = yaml_mapping_entry(line)
        if indentation == 0:
            in_jobs = entry == ("jobs", "")
            current_job = None
        elif in_jobs and indentation == 2 and entry is not None and entry[1] == "":
            name = entry[0]
            if name in jobs:
                raise ValueError(f"Rust workflow defines job {name!r} more than once")
            jobs[name] = []
            current_job = name
        elif current_job is not None:
            jobs[current_job].append(line)
    return jobs


def block_scalar_command(style: str, lines: list[str]) -> str:
    content_indentation = min(
        (len(line) - len(line.lstrip(" ")) for line in lines if line.strip()), default=0
    )
    if style == "|":
        return "\n".join(line[content_indentation:] if line.strip() else "" for line in lines)
    paragraphs: list[list[str]] = []
    paragraph: list[str] = []
    for line in lines:
        indentation = len(line) - len(line.lstrip(" "))
        if not line.strip() or indentation > content_indentation:
            if paragraph:
                paragraphs.append(paragraph)
                paragraph = []
            if line.strip():
                paragraphs.append([line])
            continue
        paragraph.append(line)
    if paragraph:
        paragraphs.append(paragraph)
    return "\n".join(" ".join(line[content_indentation:] for line in lines) for lines in paragraphs)


def job_contract(lines: list[str]) -> tuple[list[str], list[WorkflowStep]]:
    timeouts: list[str] = []
    steps: list[WorkflowStep] = []
    in_steps = False
    step_open = False
    step_command: str | None = None
    step_keys: set[str] = set()
    block_run: tuple[int, str, list[str]] | None = None
    for line in lines:
        indentation = len(line) - len(line.lstrip(" "))
        if block_run is not None:
            if not line.strip():
                block_run[2].append("")
                continue
            if indentation > block_run[0]:
                block_run[2].append(line)
                continue
            step_command = block_scalar_command(block_run[1], block_run[2])
            block_run = None
        if not line.strip():
            continue
        entry = yaml_mapping_entry(line)
        if indentation == 4:
            if step_open and step_command is not None:
                steps.append((step_command, frozenset(step_keys)))
            step_open = False
            step_command = None
            step_keys = set()
            in_steps = entry == ("steps", "")
            if entry is not None and entry[0] == "timeout-minutes":
                timeouts.append(entry[1])
            continue
        if not in_steps:
            continue
        if indentation < 6:
            if step_open and step_command is not None:
                steps.append((step_command, frozenset(step_keys)))
            in_steps = False
            step_open = False
            step_command = None
            step_keys = set()
            continue
        if indentation == 6 and line.lstrip().startswith("-"):
            if step_open and step_command is not None:
                steps.append((step_command, frozenset(step_keys)))
            step_open = True
            step_command = None
            step_keys = set()
        if (indentation == 6 and line.lstrip().startswith("-")) or (
            indentation == 8 and step_open
        ):
            stripped = line.strip().removeprefix("-").lstrip()
            step_entry = yaml_mapping_entry(stripped)
            if step_entry is not None:
                step_keys.add(step_entry[0].casefold())
            command = step_run_command(line)
            if command in {"|", "|-", "|+", ">", ">-", ">+"}:
                block_run = indentation, command[0], []
            elif command is not None:
                step_command = command
    if block_run is not None:
        step_command = block_scalar_command(block_run[1], block_run[2])
    if step_open and step_command is not None:
        steps.append((step_command, frozenset(step_keys)))
    return timeouts, steps


def job_values(lines: list[str], key: str) -> list[str]:
    values: list[str] = []
    for line in lines:
        if len(line) - len(line.lstrip(" ")) != 4:
            continue
        entry = yaml_mapping_entry(line)
        if entry is not None and entry[0] == key:
            values.append(yaml_scalar_value(entry[1]))
    return values


def enforce_workflow_contract(
    workflow: Path,
    required_timeout_minutes: int,
    workload: tuple[str, ...],
) -> None:
    try:
        source = workflow.read_text()
        jobs = workflow_jobs(source)
    except (OSError, ValueError) as error:
        sys.stderr.write(f"Rust workflow is invalid: {workflow}: {error}\n")
        raise SystemExit(1) from None
    if set(jobs) != {"rust-test", "windows-rust-test"}:
        sys.stderr.write("Rust workflow must define only the Ubuntu and Windows Rust jobs\n")
        raise SystemExit(1)
    timeouts, rust_steps = job_contract(jobs["rust-test"])
    rust_runs = [command for command, _ in rust_steps]
    found = int(timeouts[0]) if len(timeouts) == 1 and timeouts[0].isdigit() else None
    if found != required_timeout_minutes:
        sys.stderr.write(
            f"Rust job timeout must be {required_timeout_minutes} minutes; found {found}\n"
        )
        raise SystemExit(1)
    if job_values(jobs["rust-test"], "runs-on") != ["ubuntu-latest"] or job_values(
        jobs["rust-test"], "strategy"
    ):
        sys.stderr.write("Rust test job must run once on ubuntu-latest without a matrix\n")
        raise SystemExit(1)
    runs = [command for lines in jobs.values() for command, _ in job_contract(lines)[1]]
    profiler = ("scripts/profile-rust-tests",)
    profiler_count = sum(invocation_count(command, profiler) for command in runs)
    rust_profiler_count = sum(invocation_count(command, profiler) for command in rust_runs)
    workload_count = sum(invocation_count(command, workload) for command in runs)
    if rust_profiler_count != 1 or profiler_count != 1:
        sys.stderr.write("Rust workflow must invoke the exact workload gate once\n")
        raise SystemExit(1)
    windows_lines = jobs["windows-rust-test"]
    windows_timeouts, windows_steps = job_contract(windows_lines)
    windows_runs = [command for command, _ in windows_steps]
    windows_workload_count = sum(
        invocation_count(command, workload) for command in windows_runs
    )
    expected_windows_runs = [WINDOWS_PREREQUISITE, WINDOWS_TOOLCHAIN_BOOTSTRAP, WINDOWS_GATE]
    required_windows_step_keys = [keys for command, keys in windows_steps if command in {WINDOWS_TOOLCHAIN_BOOTSTRAP, WINDOWS_GATE}]
    if (
        job_values(windows_lines, "runs-on") != ["windows-latest"]
        or windows_timeouts != [str(WINDOWS_JOB_TIMEOUT_MINUTES)]
        or job_values(windows_lines, "strategy")
        or job_values(windows_lines, "if")
        or job_values(windows_lines, "continue-on-error")
        or windows_runs != expected_windows_runs
        or any({"if", "continue-on-error"} & keys for keys in required_windows_step_keys)
        or windows_workload_count != 0
        or workload_count != 0
    ):
        sys.stderr.write(
            "Windows Rust job must run the exact full workload once on windows-latest\n"
        )
        raise SystemExit(1)
