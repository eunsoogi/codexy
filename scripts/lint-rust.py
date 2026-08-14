#!/usr/bin/env python3
"""Fail Rust lint only for diagnostics rooted in changed Rust sources."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path


def relative_paths(root: Path, file_name: str, package_root: Path) -> set[str]:
    candidate = Path(file_name)
    if not candidate.is_absolute():
        candidates = (root / candidate, package_root / candidate)
    else:
        candidates = (candidate,)
    paths: set[str] = set()
    for candidate in candidates:
        try:
            paths.add(candidate.resolve().relative_to(root.resolve()).as_posix())
        except ValueError:
            continue
    return paths


def changed_line_numbers(root: Path, paths: set[str]) -> dict[str, set[int]] | None:
    base = os.environ.get("CODEXY_LINT_CHANGED_SINCE")
    if not base:
        return None
    output = subprocess.run(
        ["git", "diff", "--unified=0", "--no-ext-diff", f"{base}...HEAD", "--", *paths],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    lines: dict[str, set[int]] = {}
    path: str | None = None
    for line in output.splitlines():
        if line.startswith("+++ b/"):
            path = line.removeprefix("+++ b/")
            lines.setdefault(path, set())
            continue
        match = re.match(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@", line)
        if path is None or match is None:
            continue
        start = int(match.group(1))
        count = int(match.group(2) or "1")
        lines[path].update(range(start, start + count))
    return lines


def span_is_changed(
    span: dict[str, object],
    root: Path,
    package_root: Path,
    changed: set[str],
    lines: dict[str, set[int]] | None,
) -> bool:
    if not span.get("is_primary"):
        return False
    paths = changed & relative_paths(root, str(span.get("file_name", "")), package_root)
    if not paths:
        return False
    if lines is None:
        return True
    start = span.get("line_start")
    end = span.get("line_end", start)
    if not isinstance(start, int) or not isinstance(end, int):
        return False
    return any(set(range(start, end + 1)) & lines.get(path, set()) for path in paths)


def changed_diagnostics(
    output: str,
    root: Path,
    package_root: Path,
    changed: set[str],
    lines: dict[str, set[int]] | None = None,
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
            span_is_changed(span, root, package_root, changed, lines)
            for span in message.get("spans", [])
        ):
            diagnostics.append(message)
    return diagnostics


def print_diagnostics(diagnostics: list[dict[str, object]]) -> None:
    for message in diagnostics:
        primary = next(
            (span for span in message.get("spans", []) if span.get("is_primary")),
            {},
        )
        location = ":".join(
            str(value)
            for value in (
                primary.get("file_name", "<unknown>"),
                primary.get("line_start", "?"),
                primary.get("column_start", "?"),
            )
        )
        print(
            f"{location}: {message.get('message', 'Rust lint diagnostic')}",
            file=sys.stderr,
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest-path", required=True)
    parser.add_argument("paths", nargs="+")
    args = parser.parse_args()
    root = Path.cwd()
    package_root = (root / args.manifest_path).parent
    changed = set(args.paths)
    lines = changed_line_numbers(root, changed)
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
    diagnostics = changed_diagnostics(
        completed.stdout, root, package_root, changed, lines
    )
    if completed.returncode:
        print(completed.stderr, file=sys.stderr, end="")
        return completed.returncode
    if diagnostics:
        print_diagnostics(diagnostics)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
