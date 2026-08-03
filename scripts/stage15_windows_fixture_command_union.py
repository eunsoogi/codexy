#!/usr/bin/env python3
"""Measure the exact conservative cross-family FixtureCommand output union."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import importlib
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import sys
import time
from typing import Iterable, Mapping


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
FAMILIES = ("git", "other", "python", "shell", "validator")
PROTECTED_KEYS = (
    "RUST_TEST_THREADS", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTDOCFLAGS",
    "CARGO_BUILD_JOBS", "CARGO_BUILD_TARGET", "CARGO_TARGET_DIR",
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


def load_profile_modules():
    loader = importlib.machinery.SourceFileLoader("stage15_profile_delegate", str(PROFILE))
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
    names.update(name for name in environment if name.startswith(PROTECTED_PREFIXES))
    return {name: environment.get(name) for name in sorted(names)}


def jobs_digest() -> str:
    workflow = WORKFLOW.read_text(encoding="utf-8")
    return hashlib.sha256(workflow[workflow.index("jobs:\n") :].encode()).hexdigest()


def parity_metadata() -> dict[str, object]:
    profile_sha = hashlib.sha256(PROFILE.read_bytes()).hexdigest()
    job_sha = jobs_digest()
    if profile_sha != PROFILE_SHA256:
        raise RuntimeError("profile source identity mismatch")
    if job_sha != CANONICAL_JOBS_SHA256:
        raise RuntimeError("canonical Rust job blocks changed")
    return {
        "schema": "codexy.stage15-windows-fixture-command-union/v1",
        "base_sha": BASE_SHA,
        "git_sha": os.environ.get("GITHUB_SHA", "not-observed"),
        "profile_sha256": profile_sha,
        "canonical_jobs_sha256": job_sha,
        "source_parity": "PASS",
        "acceptance_budget_seconds": ACCEPTANCE_SECONDS,
        "diagnostic_observation_seconds": OBSERVATION_SECONDS,
        "workload": WORKLOAD,
        "profile_argv": PROFILE_ARGUMENTS,
        "wrapper_delegations": 1,
        "protected_environment_expected": protected_environment(os.environ),
        "parser_ceiling_before_nanoseconds": ACCEPTANCE_INTERVAL_NS,
        "parser_ceiling_active_nanoseconds": OBSERVATION_INTERVAL_NS,
    }


def union_ns(intervals: Iterable[tuple[int, int]]) -> int:
    total = 0
    end = None
    for start, current_end in sorted(intervals):
        if end is None or start > end:
            total += current_end - start
            end = current_end
        elif current_end > end:
            total += current_end - end
            end = current_end
    return total


def aggregate_output_rows(lines: Iterable[str]) -> dict[str, object]:
    selected: list[str] = []
    by_producer: dict[str, list[tuple[int, int]]] = {}
    by_family: dict[str, dict[str, list[tuple[int, int]]]] = {}
    for line in lines:
        fields = line.rstrip("\n").split("\t")
        if len(fields) != 10:
            raise ValueError("malformed interval metric")
        if fields[6] != "fixture-command.output":
            continue
        producer, family = fields[4], fields[7]
        if family not in FAMILIES:
            raise ValueError("unknown fixture command family")
        start, end = int(fields[8]), int(fields[9])
        selected.append("\t".join(fields) + "\n")
        by_producer.setdefault(producer, []).append((start, end))
        by_family.setdefault(family, {}).setdefault(producer, []).append((start, end))
    family_unions = {
        family: round(max((union_ns(rows) for rows in by_family.get(family, {}).values()), default=0)
                      / 1_000_000_000, 6)
        for family in FAMILIES
    }
    cross_family = round(
        max((union_ns(rows) for rows in by_producer.values()), default=0) / 1_000_000_000,
        6,
    )
    digest = hashlib.sha256("".join(sorted(selected)).encode()).hexdigest()
    return {
        "identity": "fixture-command.output",
        "aggregation": "max-producer-union-across-all-families",
        "raw_row_count": len(selected),
        "raw_rows_sha256": digest,
        "producer_count": len(by_producer),
        "cross_family_union_seconds": cross_family,
        "family_union_seconds": family_unions,
        "owned_phase_buckets": {
            "constructor": "not-observed",
            "output": {"union_seconds": cross_family, "raw_row_count": len(selected)},
        },
    }


def capture_output_rows(environment: Mapping[str, str], interval: object) -> dict[str, object]:
    value = environment.get("CODEXY_PROFILE_INTERVAL_METRICS_DIR")
    path = Path(value) if value else None
    if path is None or not path.is_dir():
        return aggregate_output_rows(())
    interval.read_rows(path, environment.get("CODEXY_PROFILE_INTERVAL_SESSION"))
    lines = [line for file in sorted(path.iterdir()) for line in file.read_text(encoding="utf-8").splitlines(keepends=True)]
    return aggregate_output_rows(lines)


def instrument_runtime(base_type, trace_path: Path, metadata: dict[str, object], interval: object):
    class UnionRuntimeTelemetry(base_type):
        def finish(self):
            control_receipt = super().finish()
            hook_started = time.perf_counter()
            output = capture_output_rows(self._environment, interval)
            hook_seconds = time.perf_counter() - hook_started
            elapsed = max(0.0, time.perf_counter() - self._started)
            receipt = {
                **metadata,
                "protected_environment_observed": protected_environment(self._environment),
                "protected_environment_match": metadata.get("protected_environment_expected")
                in (None, protected_environment(self._environment)),
                "fixture_command_output": output,
                "observer_upper_bound_seconds": round(hook_seconds, 9),
                "observer_elapsed_seconds": round(elapsed, 6),
                "perturbation_upper_bound_percent": round(100 * hook_seconds / elapsed, 9)
                if elapsed else 0.0,
                "perturbation_limit_percent": 0.05,
            }
            trace_path.parent.mkdir(parents=True, exist_ok=True)
            trace_path.write_text(json.dumps(receipt, sort_keys=True) + "\n", encoding="utf-8")
            if not receipt["protected_environment_match"]:
                raise RuntimeError("protected environment parity mismatch")
            return control_receipt

    return UnionRuntimeTelemetry


def finalize_trace(trace_path: Path, status: int, elapsed: float, restored: int) -> None:
    receipt = json.loads(trace_path.read_text(encoding="utf-8"))
    receipt.update(
        profile_exit=status,
        diagnostic_total_seconds=round(elapsed, 6),
        acceptance_300_seconds=elapsed <= ACCEPTANCE_SECONDS,
        parser_ceiling_after_nanoseconds=restored,
    )
    trace_path.write_text(json.dumps(receipt, sort_keys=True) + "\n", encoding="utf-8")


def run(trace_path: Path) -> int:
    profile, interval = load_profile_modules()
    original_runtime = profile.RuntimeTelemetry
    original_ceiling = interval.MAX_INTERVAL_NANOSECONDS
    if original_ceiling != ACCEPTANCE_INTERVAL_NS:
        raise RuntimeError("unexpected interval parser ceiling")
    profile.RuntimeTelemetry = instrument_runtime(
        original_runtime, trace_path, parity_metadata(), interval
    )
    interval.MAX_INTERVAL_NANOSECONDS = OBSERVATION_INTERVAL_NS
    previous_argv = sys.argv
    started = time.perf_counter()
    try:
        sys.argv = [str(PROFILE), *PROFILE_ARGUMENTS]
        status = profile.main()
    finally:
        sys.argv = previous_argv
        profile.RuntimeTelemetry = original_runtime
        interval.MAX_INTERVAL_NANOSECONDS = original_ceiling
    finalize_trace(trace_path, status, time.perf_counter() - started, interval.MAX_INTERVAL_NANOSECONDS)
    return status


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace", required=True, type=Path)
    return run(parser.parse_args().trace.resolve())


if __name__ == "__main__":
    raise SystemExit(main())
