"""Patchable compatibility facade for profiler tests."""

from pathlib import Path

from profile_rust_contract import WORKLOAD
from profile_rust_lifecycle import run_workload as lifecycle_run_workload

PATCHABLE = (
    "json",
    "os",
    "subprocess",
    "sys",
    "tempfile",
    "threading",
    "time",
    "RuntimeTelemetry",
    "stop_workload",
    "WindowsJob",
    "configure_metrics",
    "telemetry",
    "configure_windows_test_runner",
    "isolated_windows_test_root",
    "launch_windows_workload",
    "receipt_environment",
    "observe_first_line",
    "replay_output",
    "flush_output",
)


def run_patchable(
    namespace,
    root: Path | None,
    budget_seconds: float,
    windows: bool = False,
    workload: tuple[str, ...] | None = None,
    declared_targets: set[str] | None = None,
):
    import profile_rust_lifecycle as lifecycle

    for name in PATCHABLE:
        setattr(lifecycle, name, namespace[name])
    return lifecycle_run_workload(
        root, budget_seconds, windows, workload or WORKLOAD, declared_targets
    )
