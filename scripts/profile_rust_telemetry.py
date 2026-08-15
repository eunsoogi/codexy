"""Emit structured, read-only profiler environment and fixture telemetry."""

from __future__ import annotations

import json
import importlib.util
import os
from pathlib import Path
import re
import sys

SCRIPT_DIRECTORY = str(Path(__file__).resolve().parent)
if SCRIPT_DIRECTORY not in sys.path:
    sys.path.insert(0, SCRIPT_DIRECTORY)

from profile_rust_command_metrics import metrics as command_metrics

try:
    from profile_rust_interval_telemetry import configure as configure_intervals
    from profile_rust_interval_telemetry import metrics as interval_metrics
    from profile_rust_interval_telemetry import owner_metrics as interval_owner_metrics
except ModuleNotFoundError as error:
    if error.name != "profile_rust_interval_telemetry":
        raise
    helper_spec = importlib.util.spec_from_file_location(
        "profile_rust_interval_telemetry",
        Path(__file__).with_name("profile_rust_interval_telemetry.py"),
    )
    if helper_spec is None or helper_spec.loader is None:
        raise ImportError("missing profile interval telemetry helper")
    helper = importlib.util.module_from_spec(helper_spec)
    helper_spec.loader.exec_module(helper)
    configure_intervals = helper.configure
    interval_metrics = helper.metrics
    interval_owner_metrics = helper.owner_metrics


MAX_METRIC_RECORDS = 4096
MAX_RANKED_PROFILES = 16
IDENTITY_PATTERN = re.compile(
    r"^(?:full:[A-Za-z0-9_./-]+:[1-9][0-9]*|selective:[a-z0-9-]+)$"
)


def configure_metrics(
    environment: dict[str, str], directory: Path
) -> tuple[Path, Path]:
    metrics_path = directory / "fixture-metrics"
    command_metrics_path = directory / "command-metrics"
    environment["CODEXY_PROFILE_METRICS"] = str(metrics_path)
    environment["CODEXY_WINDOWS_PROFILE_METRICS"] = str(metrics_path)
    environment["CODEXY_PROFILE_COMMAND_METRICS_DIR"] = str(command_metrics_path)
    configure_intervals(environment, directory)
    return metrics_path, command_metrics_path


def telemetry(
    root: Path | None,
    environment: dict[str, str],
    metrics_path: Path | None,
    temp_root: dict[str, str] | None = None,
    command_metrics_path: Path | None = None,
    interval_metrics_path: Path | None = None,
    interval_owner_metrics_path: Path | None = None,
) -> str:
    target = environment.get("CARGO_TARGET_DIR")
    if target is None and root is not None:
        target = str((root / "target").resolve())
    files, copied_bytes, materializations, duration_seconds, ranked = fixture_metrics(
        metrics_path
    )
    command_ranked, command_unattributed = command_metrics(
        command_metrics_path
        or (
            Path(environment["CODEXY_PROFILE_COMMAND_METRICS_DIR"])
            if "CODEXY_PROFILE_COMMAND_METRICS_DIR" in environment
            else None
        )
    )
    interval_path = interval_metrics_path or (
        Path(environment["CODEXY_PROFILE_INTERVAL_METRICS_DIR"])
        if "CODEXY_PROFILE_INTERVAL_METRICS_DIR" in environment
        else None
    )
    interval_session = environment.get("CODEXY_PROFILE_INTERVAL_SESSION")
    interval_ranked, interval_families, interval_coverage = interval_metrics(
        interval_path, interval_session
    )
    owner_ranked, owner_coverage = interval_owner_metrics(
        interval_owner_metrics_path
        or (
            Path(environment["CODEXY_PROFILE_INTERVAL_OWNER_METRICS_DIR"])
            if "CODEXY_PROFILE_INTERVAL_OWNER_METRICS_DIR" in environment
            else None
        ),
        interval_session,
        interval_path,
    )
    temp_root = temp_root or {}
    workspace = environment.get(
        "GITHUB_WORKSPACE", str(root) if root else "not-observed"
    )
    return json.dumps(
        {
            "temp": temp_root.get(
                "original_temp", environment.get("TEMP", "not-observed")
            ),
            "tmp": temp_root.get(
                "original_tmp", environment.get("TMP", "not-observed")
            ),
            "runner_temp": temp_root.get(
                "runner_temp", environment.get("RUNNER_TEMP", "not-observed")
            ),
            "selected_temp_root": temp_root.get("selected_temp_root", "not-observed"),
            "temp_cleanup": temp_root.get("temp_cleanup", "not-applicable"),
            "workspace": workspace,
            "target": target or "not-observed",
            "same_volume_workspace": same_volume(
                temp_root.get("selected_temp_root"), workspace
            ),
            "same_volume_target": same_volume(
                temp_root.get("selected_temp_root"), target
            ),
            "logical_cpus": observed_cpu_count("cpu_count"),
            "available_parallelism": observed_cpu_count("process_cpu_count"),
            "rust_test_threads": environment.get("RUST_TEST_THREADS", "not-observed"),
            "fixture_materializations": materializations,
            "fixture_copied_files": files,
            "fixture_copied_bytes": copied_bytes,
            "fixture_materialization_seconds": duration_seconds,
            "fixture_materialization_ranked": ranked,
            "command_wait_ranked": command_ranked,
            "command_wait_unattributed": command_unattributed,
            "command_interval_ranked": interval_ranked,
            "command_interval_family_ranked": interval_families,
            "command_interval_coverage": interval_coverage,
            "command_interval_owner_ranked": owner_ranked,
            "command_interval_owner_coverage": owner_coverage,
        },
        sort_keys=True,
    )


def observed_cpu_count(name: str) -> int | str:
    probe = getattr(os, name, None)
    value = probe() if callable(probe) else None
    return value if isinstance(value, int) else "not-observed"


def fixture_metrics(
    metrics_path: Path | None,
) -> tuple[int, int, int, float, list[dict[str, float | int | str]]]:
    files = copied_bytes = materializations = 0
    duration_seconds = 0.0
    ranked: dict[str, dict[str, float | int | str]] = {}
    if metrics_path is None:
        return files, copied_bytes, materializations, duration_seconds, []
    try:
        lines = metrics_path.open(encoding="utf-8")
    except OSError:
        return files, copied_bytes, materializations, duration_seconds, []
    for index, line in enumerate(lines):
        if index == MAX_METRIC_RECORDS:
            break
        name, *values = line.split("\t")
        if name != "fixture-materialization" or len(values) not in {2, 3, 4}:
            continue
        try:
            if len(values) == 2:
                identity, count, size, duration = (
                    "not-observed",
                    *(int(value) for value in values),
                    0.0,
                )
            elif len(values) == 3:
                identity, count, size, duration = (
                    values[0],
                    *(int(value) for value in values[1:]),
                    0.0,
                )
            else:
                identity, count, size, duration = (
                    values[0],
                    int(values[1]),
                    int(values[2]),
                    float(values[3]),
                )
        except ValueError:
            continue
        identity = identity if valid_identity(identity) else "invalid"
        materializations += 1
        files += count
        copied_bytes += size
        duration_seconds += duration
        profile = ranked.setdefault(
            identity,
            {
                "identity": identity,
                "materializations": 0,
                "files": 0,
                "bytes": 0,
                "duration_seconds": 0.0,
            },
        )
        profile["materializations"] += 1
        profile["files"] += count
        profile["bytes"] += size
        profile["duration_seconds"] += duration
    return (
        files,
        copied_bytes,
        materializations,
        duration_seconds,
        sorted(
            ranked.values(),
            key=lambda profile: (
                profile["identity"] != "invalid",
                -profile["bytes"],
                -profile["files"],
                profile["identity"],
            ),
        )[:MAX_RANKED_PROFILES],
    )


def valid_identity(identity: str) -> bool:
    return (
        len(identity) <= 160
        and ".." not in identity
        and IDENTITY_PATTERN.fullmatch(identity) is not None
    )


def same_volume(selected: str | None, destination: str | None) -> bool | str:
    if (
        not selected
        or selected == "not-observed"
        or not destination
        or destination == "not-observed"
    ):
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
