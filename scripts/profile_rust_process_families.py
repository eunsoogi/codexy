"""Process-family and test-thread telemetry classification."""

from __future__ import annotations

from collections.abc import Iterable

_UNOBSERVED = "not-observed"
_FAMILIES = ("git", "python", "shell", "validator", "other")


def family_counts(records: Iterable[tuple[int, str]]) -> dict[str, int]:
    counts = {name: 0 for name in _FAMILIES}
    for _, image in records:
        counts[process_family(image)] += 1
    return counts


def valid_target(target: str, known: set[str]) -> bool:
    return target in known or target.startswith("other:")


def process_family(image: str) -> str:
    name = image.replace("\\", "/").rsplit("/", 1)[-1].casefold()
    if name in {"git", "git.exe"}:
        return "git"
    if name in {"python", "python.exe", "python3", "python3.exe", "py", "py.exe"}:
        return "python"
    if name in {
        "sh",
        "sh.exe",
        "bash",
        "bash.exe",
        "cmd",
        "cmd.exe",
        "pwsh",
        "pwsh.exe",
    }:
        return "shell"
    return "validator" if name.startswith("codexy-validate") else "other"


def test_threads(environment: dict[str, str]) -> dict[str, str]:
    value = environment.get("RUST_TEST_THREADS")
    if value is None:
        return {"state": "default/unobserved", "value": _UNOBSERVED}
    if not value.isascii() or not value.isdecimal() or int(value) < 1:
        raise ValueError("malformed configured test-thread value")
    return {"state": "configured", "value": value}
