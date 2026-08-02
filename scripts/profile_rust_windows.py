"""Windows-only profiler lifecycle facade."""

from __future__ import annotations

from profile_rust_windows_job import WindowsJob
from profile_rust_windows_launcher import (
    isolated_windows_test_root,
    launch_windows_workload,
)
from profile_rust_windows_threads import configure_windows_test_environment


def configure_windows_test_runner(
    environment: dict[str, str], temp_root: object
) -> None:
    """Configure the profiler-owned thread and test-runner environment."""
    configure_windows_test_environment(environment, temp_root)

__all__ = (
    "WindowsJob",
    "configure_windows_test_runner",
    "isolated_windows_test_root",
    "launch_windows_workload",
)
