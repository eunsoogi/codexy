#!/usr/bin/env python3
"""Perform non-executing checks for native Windows command launchers."""

from __future__ import annotations

import stat
import sys
from pathlib import Path


ALLOWED_LINE_PREFIXES = (
    '"',
    "@echo ",
    "call ",
    "del ",
    "echo ",
    "exit /b",
    "for ",
    "goto ",
    "if ",
    "py ",
    "powershell ",
    "set ",
    "setlocal ",
    "type ",
)


def has_supported_launcher_syntax(text: str) -> bool:
    for line in text.splitlines():
        source = line.strip()
        if not source or source.startswith(("::", "rem ", ":")):
            continue
        if not source.lower().startswith(ALLOWED_LINE_PREFIXES):
            return False
    return True


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    files = sys.argv[1:]
    errors = []
    for relative in files:
        candidate = root / relative
        if (
            Path(relative).is_absolute()
            or ".." in Path(relative).parts
            or not stat.S_ISREG(candidate.lstat().st_mode)
        ):
            errors.append(f"{relative}: lint input must be a regular repository file")
            continue
        text = candidate.read_text(encoding="utf-8")
        if not text.startswith("@echo off\n"):
            errors.append(f"{relative}: launcher must start with @echo off")
        if "exit /b" not in text.lower():
            errors.append(f"{relative}: launcher must return with exit /b")
        if not has_supported_launcher_syntax(text):
            errors.append(f"{relative}: unsupported launcher syntax")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"validated {len(files)} native Windows command launchers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
