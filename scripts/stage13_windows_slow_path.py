#!/usr/bin/env python3
"""Timestamp the existing Windows profiler stream without changing its workload."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import importlib
import importlib.machinery
import importlib.util
import json
import os
import re
import sys
import time
from pathlib import Path

PROFILE = Path(__file__).with_name("profile-rust-tests")
RUN_RESULT = re.compile(r"^test (?P<name>.+) \.\.\. (?P<result>ok|FAILED|ignored)$")
RUN_START = re.compile(r"^test (?P<name>.+?) \.\.\. .+$")
RESULT_SUMMARY = re.compile(r"^test result: (?:ok|FAILED)\.")


@contextmanager
def sibling_imports():
    scripts = str(PROFILE.parent)
    inserted = scripts not in sys.path
    if inserted:
        sys.path.insert(0, scripts)
    try:
        yield
    finally:
        if inserted:
            sys.path.remove(scripts)


with sibling_imports():
    target_name = importlib.import_module("profile_rust_runtime_telemetry").target_name


def load_profile_module():
    loader = importlib.machinery.SourceFileLoader("stage13_profile_delegate", str(PROFILE))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    if spec is None:
        raise RuntimeError("existing profiler is not loadable")
    module = importlib.util.module_from_spec(spec)
    with sibling_imports():
        loader.exec_module(module)
    return module


def instrument_runtime(base_type, trace_path: Path, profile_sha256: str = "unknown"):
    class TimestampRuntimeTelemetry(base_type):
        def __init__(self, started, declared, environment):
            super().__init__(started, declared, environment)
            self._trace_path = trace_path
            self._profile_sha256 = profile_sha256
            self._current_target = None
            self._pending_test = None
            self._last_completion = None
            self._target_boundaries = []
            self._test_completions = []
            self._control_hook_ns = 0
            self._diagnostic_hook_ns = 0
            self._observer_lines = 0

        def _observe_line(self, line):
            control_started = time.perf_counter_ns()
            super()._observe_line(line)
            self._control_hook_ns += time.perf_counter_ns() - control_started
            diagnostic_started = time.perf_counter_ns()
            self._observer_lines += 1
            self._record_line(line)
            self._diagnostic_hook_ns += time.perf_counter_ns() - diagnostic_started

        def _record_line(self, line):
            moment = round(time.perf_counter() - self._started, 6)
            if "Running " in line:
                self._current_target = target_name(line)
                self._pending_test = None
                self._target_boundaries.append(
                    {"target": self._current_target, "state": "started", "seconds": moment}
                )
            elif self._current_target and (match := RUN_RESULT.match(line)):
                self._pending_test = None
                self._record_completion(match.group("name"), match.group("result"), moment)
            elif self._current_target and (match := RUN_START.match(line)):
                self._pending_test = match.group("name")
            elif self._current_target and self._pending_test and line in {"ok", "FAILED", "ignored"}:
                self._record_completion(self._pending_test, line, moment)
                self._pending_test = None
            elif self._current_target and RESULT_SUMMARY.match(line):
                self._pending_test = None
                self._target_boundaries.append(
                    {"target": self._current_target, "state": "completed", "seconds": moment}
                )

        def _record_completion(self, name, outcome, moment):
            gap = None if self._last_completion is None else round(moment - self._last_completion, 6)
            self._test_completions.append(
                {
                    "test": f"{self._current_target}::{name.removesuffix(' - should panic')}",
                    "outcome": outcome,
                    "seconds": moment,
                    "gap_seconds": gap,
                }
            )
            self._last_completion = moment

        def finish(self):
            control_receipt = super().finish()
            elapsed = max(0.0, time.perf_counter() - self._started)
            upper_bound = (self._control_hook_ns + self._diagnostic_hook_ns) / 1_000_000_000
            receipt = {
                "schema": "codexy.stage13-windows-slow-path/v1",
                "git_sha": os.environ.get("GITHUB_SHA", "not-observed"),
                "profile_sha256": self._profile_sha256,
                "workload": ["cargo", "test", "--locked", "--all-targets"],
                "budget_seconds": 300.0,
                "observer_lines": self._observer_lines,
                "observer_elapsed_seconds": round(elapsed, 6),
                "control_hook_seconds": round(self._control_hook_ns / 1_000_000_000, 9),
                "diagnostic_hook_seconds": round(self._diagnostic_hook_ns / 1_000_000_000, 9),
                "observer_upper_bound_seconds": round(upper_bound, 9),
                "perturbation_upper_bound_percent": round(100 * upper_bound / elapsed, 9)
                if elapsed
                else 0.0,
                "target_boundaries": self._target_boundaries,
                "test_completions": self._test_completions,
            }
            self._trace_path.parent.mkdir(parents=True, exist_ok=True)
            self._trace_path.write_text(json.dumps(receipt, sort_keys=True) + "\n", encoding="utf-8")
            return control_receipt

    return TimestampRuntimeTelemetry


def run(trace_path: Path, profile_arguments: list[str]) -> int:
    profile = load_profile_module()
    digest = hashlib.sha256(PROFILE.read_bytes()).hexdigest()
    profile.RuntimeTelemetry = instrument_runtime(profile.RuntimeTelemetry, trace_path, digest)
    previous = sys.argv
    try:
        sys.argv = [str(PROFILE), *profile_arguments]
        return profile.main()
    finally:
        sys.argv = previous


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace", required=True, type=Path)
    arguments, profile_arguments = parser.parse_known_args()
    return run(arguments.trace.resolve(), profile_arguments)


if __name__ == "__main__":
    raise SystemExit(main())
