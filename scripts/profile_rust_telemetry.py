"""Emit structured, read-only profiler environment and fixture telemetry."""

from __future__ import annotations

import json
import os
from pathlib import Path


def telemetry(root: Path | None, environment: dict[str, str], metrics_path: Path | None) -> str:
    target = environment.get("CARGO_TARGET_DIR")
    if target is None and root is not None:
        target = str((root / "target").resolve())
    files, copied_bytes, materializations = fixture_metrics(metrics_path)
    return json.dumps(
        {
            "temp": environment.get("TEMP", "not-observed"),
            "tmp": environment.get("TMP", "not-observed"),
            "runner_temp": environment.get("RUNNER_TEMP", "not-observed"),
            "workspace": environment.get("GITHUB_WORKSPACE", str(root) if root else "not-observed"),
            "target": target or "not-observed",
            "logical_cpus": observed_cpu_count("cpu_count"),
            "available_parallelism": observed_cpu_count("process_cpu_count"),
            "rust_test_threads": environment.get("RUST_TEST_THREADS", "not-observed"),
            "fixture_materializations": materializations,
            "fixture_copied_files": files,
            "fixture_copied_bytes": copied_bytes,
        },
        sort_keys=True,
    )


def observed_cpu_count(name: str) -> int | str:
    probe = getattr(os, name, None)
    value = probe() if callable(probe) else None
    return value if isinstance(value, int) else "not-observed"


def fixture_metrics(metrics_path: Path | None) -> tuple[int, int, int]:
    files = copied_bytes = materializations = 0
    if metrics_path is None:
        return files, copied_bytes, materializations
    try:
        lines = metrics_path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return files, copied_bytes, materializations
    for line in lines:
        name, *values = line.split("\t")
        if name != "fixture-materialization" or len(values) != 2:
            continue
        try:
            count, size = (int(value) for value in values)
        except ValueError:
            continue
        materializations += 1
        files += count
        copied_bytes += size
    return files, copied_bytes, materializations
