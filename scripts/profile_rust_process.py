"""Process capture and timing for Rust profiling commands."""

from __future__ import annotations

import re
import os
import subprocess
import sys
import threading
import time
from collections import Counter
from pathlib import Path

from profile_rust_coverage import canonical_test_id

RUN_TEST = re.compile(r"^test (?P<name>.+) \.\.\. ok$")
RESULT = re.compile(r"test result: (?:ok|FAILED)\. (?P<passed>\d+) passed; (?P<failed>\d+) failed; (?P<ignored>\d+) ignored")
COMPILE = re.compile(r"Finished `test` profile .* in (?:(?P<minutes>\d+)m )?(?P<seconds>[0-9.]+)s")


def metric_counts(path: Path) -> dict[str, int]:
    if not path.exists():
        return {}
    return dict(Counter(line.strip() for line in path.read_text().splitlines() if line.strip()))


def run_command(command: list[str], target: str, metrics_path: Path) -> dict[str, object]:
    started = time.perf_counter()
    environment = os.environ | {
        "CODEXY_WINDOWS_PROFILE_METRICS": str(metrics_path),
        "CODEXY_ARCHIVE_HEADER_EVIDENCE": str(
            metrics_path.with_suffix(".archive-headers.jsonl")
        ),
    }
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, bufsize=1, env=environment)
    stdout_lines: list[str] = []
    stderr_lines: list[str] = []
    events: list[tuple[float, str]] = []

    def drain(stream: object, destination: list[str], surface: object) -> None:
        for line in stream:  # type: ignore[union-attr]
            destination.append(line)
            events.append((time.perf_counter() - started, line))
            surface.write(line)  # type: ignore[union-attr]

    assert process.stdout is not None and process.stderr is not None
    stdout_thread = threading.Thread(target=drain, args=(process.stdout, stdout_lines, sys.stdout))
    stderr_thread = threading.Thread(target=drain, args=(process.stderr, stderr_lines, sys.stderr))
    stdout_thread.start()
    stderr_thread.start()
    exit_code = process.wait()
    stdout_thread.join()
    stderr_thread.join()
    elapsed = time.perf_counter() - started
    stdout = "".join(stdout_lines)
    stderr = "".join(stderr_lines)
    events.sort(key=lambda event: event[0])
    lines = [line for _, line in events]
    first_compile = next((elapsed for elapsed, line in events if "Compiling " in line), None)
    finished = next((elapsed for elapsed, line in events if COMPILE.search(line)), None)
    tests = [canonical_test_id(target, match.group("name")) for line in lines if (match := RUN_TEST.match(line.strip()))]
    results = [{key: int(value) for key, value in match.groupdict().items()} for line in lines if (match := RESULT.search(line))]
    compile_seconds = sum(int(match["minutes"] or 0) * 60 + float(match["seconds"]) for line in lines if (match := COMPILE.search(line)))
    return {"command": command, "target": target, "exitCode": exit_code, "durationSeconds": elapsed,
            "setupSeconds": first_compile or finished or elapsed, "compileSeconds": compile_seconds,
            "executionSeconds": max(0.0, elapsed - (finished or 0.0)), "tests": sorted(tests),
            "passed": sum(result["passed"] for result in results), "failed": sum(result["failed"] for result in results),
            "ignored": sum(result["ignored"] for result in results), "metrics": metric_counts(metrics_path),
            "stdout": stdout, "stderr": stderr}
