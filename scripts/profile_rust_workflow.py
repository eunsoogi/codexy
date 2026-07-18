"""Validate that the Rust workflow delegates the exact workload once."""

from __future__ import annotations

import re
import shlex
import sys
from pathlib import Path

WORKFLOW_KEY_PATTERN = re.compile(r"^(?P<key>[^:#][^:]*):(?P<value>.*)$")


def yaml_mapping_entry(line: str) -> tuple[str, str] | None:
    stripped = line.strip()
    if not stripped or stripped.startswith("#") or stripped.startswith("-"):
        return None
    match = WORKFLOW_KEY_PATTERN.match(stripped)
    if match is None:
        return None
    return match.group("key").strip(), match.group("value").split("#", 1)[0].strip()


def step_run_command(line: str) -> str | None:
    stripped = line.strip()
    if stripped.startswith("-"):
        stripped = stripped[1:].lstrip()
    entry = yaml_mapping_entry(stripped)
    return entry[1] if entry is not None and entry[0] == "run" else None


def workflow_jobs(source: str) -> dict[str, list[str]]:
    jobs: dict[str, list[str]] = {}
    current_job: str | None = None
    in_jobs = False
    for line in source.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
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


def job_contract(lines: list[str]) -> tuple[list[str], list[str]]:
    timeouts: list[str] = []
    runs: list[str] = []
    in_steps = False
    step_open = False
    block_run: tuple[int, str, list[str]] | None = None
    for line in lines:
        indentation = len(line) - len(line.lstrip(" "))
        if block_run is not None:
            if not line.strip() or indentation > block_run[0]:
                if line.strip():
                    block_run[2].append(line)
                continue
            runs.append((" " if block_run[1] == ">" else "\n").join(block_run[2]))
            block_run = None
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
        runs.append((" " if block_run[1] == ">" else "\n").join(block_run[2]))
    return timeouts, runs


def shell_commands(command: str) -> list[list[str]]:
    commands: list[list[str]] = []
    for line in command.splitlines():
        lexer = shlex.shlex(line, posix=True, punctuation_chars=";&|")
        lexer.whitespace_split = True
        lexer.commenters = "#"
        current: list[str] = []
        for token in lexer:
            if token and set(token) <= {";", "&", "|"}:
                if current:
                    commands.append(current)
                    current = []
            else:
                current.append(token)
        if current:
            commands.append(current)
    return commands


def invocation_count(command: str, invocation: tuple[str, ...]) -> int:
    return sum(
        executable_tokens(tokens)[: len(invocation)] == list(invocation)
        for tokens in shell_commands(command)
    )


def executable_tokens(tokens: list[str]) -> list[str]:
    return tokens[1:] if tokens[:1] == ["command"] else tokens


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
    if workload_count:
        sys.stderr.write("Rust workflow must not run the full workload outside its gate\n")
        raise SystemExit(1)
