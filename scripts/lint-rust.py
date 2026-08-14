#!/usr/bin/env python3
"""Fail Rust lint only for diagnostics rooted in changed Rust sources."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def relative_path(root: Path, file_name: str, package_root: Path) -> str | None:
    candidate = Path(file_name)
    if not candidate.is_absolute():
        candidates = (root / candidate, package_root / candidate)
    else:
        candidates = (candidate,)
    for candidate in candidates:
        try:
            return candidate.resolve().relative_to(root.resolve()).as_posix()
        except ValueError:
            continue
    return None


def changed_diagnostics(
    output: str, root: Path, package_root: Path, changed: set[str]
) -> list[dict[str, object]]:
    diagnostics: list[dict[str, object]] = []
    for line in output.splitlines():
        try:
            cargo_message = json.loads(line)
        except json.JSONDecodeError:
            continue
        message = cargo_message.get("message", {})
        if cargo_message.get("reason") != "compiler-message":
            continue
        if message.get("level") not in {"warning", "error"}:
            continue
        if any(
            span.get("is_primary")
            and relative_path(root, span.get("file_name", ""), package_root) in changed
            for span in message.get("spans", [])
        ):
            diagnostics.append(message)
    return diagnostics


def print_diagnostics(diagnostics: list[dict[str, object]]) -> None:
    for message in diagnostics:
        print(message.get("message", "Rust lint diagnostic"), file=sys.stderr)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest-path", required=True)
    parser.add_argument("paths", nargs="+")
    args = parser.parse_args()
    root = Path.cwd()
    package_root = (root / args.manifest_path).parent
    changed = set(args.paths)
    command = (
        "cargo",
        "+1.85.0",
        "clippy",
        "--manifest-path",
        args.manifest_path,
        "--locked",
        "--all-targets",
        "--all-features",
        "--message-format=json",
        "--",
        "--cap-lints=warn",
    )
    completed = subprocess.run(command, cwd=root, capture_output=True, text=True)
    diagnostics = changed_diagnostics(completed.stdout, root, package_root, changed)
    if completed.returncode:
        print(completed.stderr, file=sys.stderr, end="")
        return completed.returncode
    if diagnostics:
        print_diagnostics(diagnostics)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
