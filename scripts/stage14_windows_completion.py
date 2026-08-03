#!/usr/bin/env python3
"""Observe one unchanged Windows profiler workload through a 900-second ceiling."""

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
from typing import Mapping


PROFILE = Path(__file__).with_name("profile-rust-tests")
WORKFLOW = PROFILE.parent.parent / ".github" / "workflows" / "rust-test.yml"
BASE_SHA = "c929249f6b44ab214a057e47284daa6bc7b08a68"
PROFILE_SHA256 = "fa6d23a35d528c9cab2ed4cfdb9569d79284eadb12e9bdc6fd53506b158fb6b1"
CANONICAL_JOBS_SHA256 = "d4fe217d930f06e267be4988b6f86c0641bb088e5d93e159d5c114ed5d2f7751"
ACCEPTANCE_SECONDS = 300.0
OBSERVATION_SECONDS = 900.0
ACCEPTANCE_INTERVAL_NS = 300_000_000_000
OBSERVATION_INTERVAL_NS = 900_000_000_000
PROFILE_ARGUMENTS = ["--windows", "--budget-seconds", "900.0"]
WORKLOAD = ["cargo", "test", "--locked", "--all-targets"]
RUN_RESULT = re.compile(r"^test (?P<name>.+) \.\.\. (?P<result>ok|FAILED|ignored)$")
RUN_START = re.compile(r"^test (?P<name>.+?) \.\.\. .+$")
RESULT_SUMMARY = re.compile(r"^test result: (?:ok|FAILED)\.")
PROTECTED_KEYS = (
    "RUST_TEST_THREADS",
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "RUSTDOCFLAGS",
    "CARGO_BUILD_JOBS",
    "CARGO_BUILD_TARGET",
    "CARGO_TARGET_DIR",
)
PROTECTED_PREFIXES = ("CARGO_PROFILE_", "CARGO_BUILD_")


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


def load_profile_modules():
    loader = importlib.machinery.SourceFileLoader("stage14_profile_delegate", str(PROFILE))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    if spec is None:
        raise RuntimeError("existing profiler is not loadable")
    profile = importlib.util.module_from_spec(spec)
    with sibling_imports():
        loader.exec_module(profile)
        interval = importlib.import_module("profile_rust_interval_telemetry")
    return profile, interval


def protected_environment(environment: Mapping[str, str]) -> dict[str, str | None]:
    names = set(PROTECTED_KEYS)
    names.update(
        name for name in environment if name.startswith(PROTECTED_PREFIXES)
    )
    return {name: environment.get(name) for name in sorted(names)}


def jobs_digest() -> str:
    workflow = WORKFLOW.read_text(encoding="utf-8")
    jobs = workflow[workflow.index("jobs:\n") :]
    return hashlib.sha256(jobs.encode()).hexdigest()


def parity_metadata() -> dict[str, object]:
    profile_sha = hashlib.sha256(PROFILE.read_bytes()).hexdigest()
    canonical_jobs_sha = jobs_digest()
    if profile_sha != PROFILE_SHA256:
        raise RuntimeError("profile source identity mismatch")
    if canonical_jobs_sha != CANONICAL_JOBS_SHA256:
        raise RuntimeError("canonical Rust job blocks changed")
    return {
        "base_sha": BASE_SHA,
        "git_sha": os.environ.get("GITHUB_SHA", "not-observed"),
        "profile_sha256": profile_sha,
        "canonical_jobs_sha256": canonical_jobs_sha,
        "acceptance_budget_seconds": ACCEPTANCE_SECONDS,
        "diagnostic_observation_seconds": OBSERVATION_SECONDS,
        "workload": WORKLOAD,
        "profile_argv": PROFILE_ARGUMENTS,
        "wrapper_delegations": 1,
        "protected_environment_expected": protected_environment(os.environ),
        "parser_ceiling_before_nanoseconds": ACCEPTANCE_INTERVAL_NS,
        "parser_ceiling_active_nanoseconds": OBSERVATION_INTERVAL_NS,
    }


def instrument_runtime(base_type, trace_path: Path, metadata: dict[str, object]):
    class CompletionRuntimeTelemetry(base_type):
        def __init__(self, started, declared, environment):
            super().__init__(started, declared, environment)
            self._trace_path = trace_path
            self._metadata = dict(metadata)
            self._protected_observed = protected_environment(environment)
            self._current_target = self._pending_test = self._last_completion = None
            self._target_boundaries, self._test_completions = [], []
            self._control_hook_ns = self._diagnostic_hook_ns = self._observer_lines = 0

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
                self._current_target, self._pending_test = target_name(line), None
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
                "schema": "codexy.stage14-windows-completion/v1",
                **self._metadata,
                "protected_environment_observed": self._protected_observed,
                "protected_environment_match": self._metadata["protected_environment_expected"]
                == self._protected_observed,
                "observer_lines": self._observer_lines,
                "observer_elapsed_seconds": round(elapsed, 6),
                "control_hook_seconds": round(self._control_hook_ns / 1_000_000_000, 9),
                "diagnostic_hook_seconds": round(self._diagnostic_hook_ns / 1_000_000_000, 9),
                "observer_upper_bound_seconds": round(upper_bound, 9),
                "perturbation_upper_bound_percent": round(100 * upper_bound / elapsed, 9)
                if elapsed
                else 0.0,
                "perturbation_limit_percent": 0.05,
                "target_boundaries": self._target_boundaries,
                "test_completions": self._test_completions,
                "phase_b": {
                    "required_metric": "conservative_union_occupancy_seconds",
                    "minimum_occupancy_seconds": 60.0,
                    "excluded_evidence": [
                        "test-completion-gap",
                        "cumulative-parallel-time",
                        "suite-residency",
                    ],
                },
            }
            self._trace_path.parent.mkdir(parents=True, exist_ok=True)
            self._trace_path.write_text(
                json.dumps(receipt, sort_keys=True) + "\n", encoding="utf-8"
            )
            if not receipt["protected_environment_match"]:
                raise RuntimeError("protected environment parity mismatch")
            return control_receipt

    return CompletionRuntimeTelemetry


def record_restored_ceiling(trace_path: Path, restored: int) -> None:
    if not trace_path.is_file():
        return
    receipt = json.loads(trace_path.read_text(encoding="utf-8"))
    receipt["parser_ceiling_after_nanoseconds"] = restored
    trace_path.write_text(json.dumps(receipt, sort_keys=True) + "\n", encoding="utf-8")


def run(trace_path: Path) -> int:
    profile, interval = load_profile_modules()
    original_runtime = profile.RuntimeTelemetry
    original_ceiling = interval.MAX_INTERVAL_NANOSECONDS
    if original_ceiling != ACCEPTANCE_INTERVAL_NS:
        raise RuntimeError("unexpected interval parser ceiling")
    profile.RuntimeTelemetry = instrument_runtime(
        original_runtime, trace_path, parity_metadata()
    )
    interval.MAX_INTERVAL_NANOSECONDS = OBSERVATION_INTERVAL_NS
    previous_argv = sys.argv
    try:
        sys.argv = [str(PROFILE), *PROFILE_ARGUMENTS]
        return profile.main()
    finally:
        sys.argv = previous_argv
        profile.RuntimeTelemetry = original_runtime
        interval.MAX_INTERVAL_NANOSECONDS = original_ceiling
        record_restored_ceiling(trace_path, interval.MAX_INTERVAL_NANOSECONDS)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace", required=True, type=Path)
    arguments = parser.parse_args()
    return run(arguments.trace.resolve())


if __name__ == "__main__":
    raise SystemExit(main())
