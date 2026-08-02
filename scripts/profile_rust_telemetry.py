"""Emit structured, read-only profiler environment and fixture telemetry."""

from __future__ import annotations

import json
import os
from pathlib import Path


def telemetry(
    root: Path | None,
    environment: dict[str, str],
    metrics_path: Path | None,
    temp_root: dict[str, str] | None = None,
) -> str:
    target = environment.get("CARGO_TARGET_DIR")
    if target is None and root is not None:
        target = str((root / "target").resolve())
    files, copied_bytes, materializations, ranked = fixture_metrics(metrics_path)
    temp_root = temp_root or {}
    workspace = environment.get("GITHUB_WORKSPACE", str(root) if root else "not-observed")
    return json.dumps(
        {
            "temp": temp_root.get("original_temp", environment.get("TEMP", "not-observed")),
            "tmp": temp_root.get("original_tmp", environment.get("TMP", "not-observed")),
            "runner_temp": temp_root.get("runner_temp", environment.get("RUNNER_TEMP", "not-observed")),
            "selected_temp_root": temp_root.get("selected_temp_root", "not-observed"),
            "temp_cleanup": temp_root.get("temp_cleanup", "not-applicable"),
            "workspace": workspace,
            "target": target or "not-observed",
            "same_volume_workspace": same_volume(temp_root.get("selected_temp_root"), workspace),
            "same_volume_target": same_volume(temp_root.get("selected_temp_root"), target),
            "logical_cpus": observed_cpu_count("cpu_count"),
            "available_parallelism": observed_cpu_count("process_cpu_count"),
            "rust_test_threads": environment.get("RUST_TEST_THREADS", "not-observed"),
            "fixture_materializations": materializations,
            "fixture_copied_files": files,
            "fixture_copied_bytes": copied_bytes,
            "fixture_materialization_ranked": ranked,
        },
        sort_keys=True,
    )


def observed_cpu_count(name: str) -> int | str:
    probe = getattr(os, name, None)
    value = probe() if callable(probe) else None
    return value if isinstance(value, int) else "not-observed"


def fixture_metrics(metrics_path: Path | None) -> tuple[int, int, int, list[dict[str, int | str]]]:
    files = copied_bytes = materializations = 0
    ranked: dict[str, dict[str, int | str]] = {}
    if metrics_path is None:
        return files, copied_bytes, materializations, []
    try:
        lines = metrics_path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return files, copied_bytes, materializations, []
    for line in lines:
        name, *values = line.split("\t")
        if name != "fixture-materialization" or len(values) not in {2, 3}:
            continue
        try:
            if len(values) == 2:
                identity, count, size = "not-observed", *(int(value) for value in values)
            else:
                identity, count, size = values[0], *(int(value) for value in values[1:])
        except ValueError:
            continue
        materializations += 1
        files += count
        copied_bytes += size
        profile = ranked.setdefault(
            identity,
            {"identity": identity, "materializations": 0, "files": 0, "bytes": 0},
        )
        profile["materializations"] += 1
        profile["files"] += count
        profile["bytes"] += size
    return files, copied_bytes, materializations, sorted(
        ranked.values(), key=lambda profile: (-profile["bytes"], -profile["files"], profile["identity"])
    )


def same_volume(selected: str | None, destination: str | None) -> bool | str:
    if not selected or selected == "not-observed" or not destination or destination == "not-observed":
        return "not-observed"
    selected_path, destination_path = Path(selected), Path(destination)
    if selected_path.drive or destination_path.drive:
        if not selected_path.drive or not destination_path.drive:
            return "not-observed"
        return selected_path.drive.casefold() == destination_path.drive.casefold()
    try:
        return os.stat(selected_path).st_dev == os.stat(destination_path).st_dev
    except OSError:
        return "not-observed"
