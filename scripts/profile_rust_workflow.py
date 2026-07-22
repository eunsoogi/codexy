"""Validate that the Rust workflow delegates the exact workload once."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from profile_rust_shell import invocation_count

WORKFLOW_KEY_PATTERN = re.compile(r"^(?P<key>[^:#][^:]*):(?P<value>.*)$")


def yaml_mapping_entry(line: str) -> tuple[str, str] | None:
    stripped = line.strip()
    if not stripped or stripped.startswith("#") or stripped.startswith("-"):
        return None
    match = WORKFLOW_KEY_PATTERN.match(stripped)
    if match is None:
        return None
    return match.group("key").strip(), yaml_value_without_comment(match.group("value")).strip()


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
    if style == "|":
        return "\n".join(lines)
    content_indentation = min(
        len(line) - len(line.lstrip(" ")) for line in lines if line.strip()
    )
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
    return "\n".join(" ".join(lines) for lines in paragraphs)


def job_contract(lines: list[str]) -> tuple[list[str], list[str]]:
    timeouts: list[str] = []
    runs: list[str] = []
    in_steps = False
    step_open = False
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
            runs.append(block_scalar_command(block_run[1], block_run[2]))
            block_run = None
        if not line.strip():
            continue
        entry = yaml_mapping_entry(line)
        if indentation == 4:
            step_open = False
            in_steps = entry == ("steps", "")
            if entry is not None and entry[0] == "timeout-minutes":
                timeouts.append(entry[1])
            continue
        if not in_steps:
            continue
        if indentation < 6:
            in_steps = False
            step_open = False
            continue
        if indentation == 6 and line.lstrip().startswith("-"):
            step_open = True
        if (indentation == 6 and line.lstrip().startswith("-")) or (
            indentation == 8 and step_open
        ):
            command = step_run_command(line)
            if command in {"|", "|-", "|+", ">", ">-", ">+"}:
                block_run = indentation, command[0], []
            elif command is not None:
                runs.append(command)
    if block_run is not None:
        runs.append(block_scalar_command(block_run[1], block_run[2]))
    return timeouts, runs


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
    if "rust-test" not in jobs:
        sys.stderr.write("Rust workflow must define the rust-test job\n")
        raise SystemExit(1)
    timeouts, rust_runs = job_contract(jobs["rust-test"])
    found = int(timeouts[0]) if len(timeouts) == 1 and timeouts[0].isdigit() else None
    if found != required_timeout_minutes:
        sys.stderr.write(
            f"Rust job timeout must be {required_timeout_minutes} minutes; found {found}\n"
        )
        raise SystemExit(1)
    runs = [command for lines in jobs.values() for command in job_contract(lines)[1]]
    profiler = ("scripts/profile-rust-tests",)
    profiler_count = sum(invocation_count(command, profiler) for command in runs)
    rust_profiler_count = sum(invocation_count(command, profiler) for command in rust_runs)
    workload_count = sum(invocation_count(command, workload) for command in runs)
    if rust_profiler_count != 1 or profiler_count != 1:
        sys.stderr.write("Rust workflow must invoke the exact workload gate once\n")
        raise SystemExit(1)
    windows_lines = jobs.get("windows-rust-test")
    if windows_lines is None:
        if workload_count:
            sys.stderr.write("Rust workflow must not run the full workload outside its gate\n")
            raise SystemExit(1)
        return
    windows_runs = job_contract(windows_lines)[1]
    windows_workload_count = sum(
        invocation_count(command, workload) for command in windows_runs
    )
    exact_workload = " ".join(workload)
    if (
        job_values(windows_lines, "runs-on") != ["windows-latest"]
        or windows_runs != [exact_workload]
        or windows_workload_count != 1
        or workload_count != windows_workload_count
    ):
        sys.stderr.write(
            "Windows Rust job must run the exact full workload once on windows-latest\n"
        )
        raise SystemExit(1)
