"""Own the Windows profiler's explicit libtest concurrency contract."""

from __future__ import annotations

from profile_rust_windows_launcher import configure_windows_test_runner


WINDOWS_TEST_THREADS = "8"
_THREADS = "RUST_TEST_THREADS"


def configure_windows_test_environment(
    environment: dict[str, str], temp_root: object
) -> None:
    """Set the profiler-owned thread count before the one Cargo launch."""
    if _THREADS in environment:
        raise OSError(f"{_THREADS} is profiler-owned for the Windows Rust workload")
    environment[_THREADS] = WINDOWS_TEST_THREADS
    configure_windows_test_runner(environment, temp_root)
