"""Own the Windows profiler's Cargo test-profile boundary."""

from __future__ import annotations

from profile_rust_cargo_profile import CARGO_TEST_PROFILE_KEYS
from profile_rust_windows_launcher import configure_windows_test_runner


_THREADS = "RUST_TEST_THREADS"


def configure_windows_test_environment(
    environment: dict[str, str], temp_root: object
) -> None:
    """Fail closed on overrides, then configure the one Windows Cargo child."""
    for key in (_THREADS, *CARGO_TEST_PROFILE_KEYS):
        if key in environment:
            raise OSError(f"{key} is profiler-owned for the Windows Rust workload")
    environment.update(
        {
            "CARGO_PROFILE_TEST_DEBUG": "0",
            "CARGO_PROFILE_TEST_OPT_LEVEL": "1",
        }
    )
    configure_windows_test_runner(environment, temp_root)
