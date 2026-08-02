"""Describe the profiler-owned Cargo test profile without serializing its environment."""

from __future__ import annotations


CARGO_TEST_PROFILE_KEYS = (
    "CARGO_PROFILE_TEST_DEBUG",
    "CARGO_PROFILE_TEST_OPT_LEVEL",
)
_UNOBSERVED = "not-observed"


def cargo_test_profile(environment: dict[str, str]) -> dict[str, str]:
    """Return the only accepted Cargo test-profile receipt descriptor."""
    debug, opt_level = (environment.get(key) for key in CARGO_TEST_PROFILE_KEYS)
    if debug is None and opt_level is None:
        return {
            "state": "default/unobserved",
            "debug": _UNOBSERVED,
            "opt_level": _UNOBSERVED,
        }
    if debug is None or opt_level is None:
        raise ValueError("partial configured Cargo test profile")
    if (debug, opt_level) != ("0", "1"):
        raise ValueError("malformed configured Cargo test profile")
    return {"state": "configured", "debug": debug, "opt_level": opt_level}


def test_threads(environment: dict[str, str]) -> dict[str, str]:
    """Retain the existing exact configured-or-default thread descriptor."""
    value = environment.get("RUST_TEST_THREADS")
    if value is None:
        return {"state": "default/unobserved", "value": _UNOBSERVED}
    if not value.isascii() or not value.isdecimal() or int(value) < 1:
        raise ValueError("malformed configured test-thread value")
    return {"state": "configured", "value": value}
