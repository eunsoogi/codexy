"""Recover physical Cargo targets and their canonical logical identities."""
from __future__ import annotations

from pathlib import Path

SHARD_TARGETS = ("support", "agent", "child", "governance", "system", "archive")


def target_name(line: str) -> str:
    path = line.replace("\\", "/")
    if "suites/all.rs" in path:
        return "suite_orchestration" if "suite_orchestration" in path else "suite_all"
    if "tests/suites/" in path:
        for name in SHARD_TARGETS:
            if f"suite_{name}" in path:
                return f"suite_{name}"
    if "src/lib.rs" in path:
        return "lib"
    if "src/bin/" in path:
        return Path(path.split("src/bin/", 1)[1].split(" ", 1)[0]).stem
    source = path.split("Running ", 1)[-1].split(" (", 1)[0].strip()
    return f"other:{source}" if source else "unknown"


def canonical_test_name(name: str) -> str:
    return name.removesuffix(" - should panic")
