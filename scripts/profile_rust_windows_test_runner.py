#!/usr/bin/env python3
"""Run one Cargo-selected Windows test with an isolated temporary directory."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Sequence


_TEMP_ROOT = "CODEXY_WINDOWS_TEST_TEMP_ROOT"


def isolated_test_temp(environment: dict[str, str]) -> Path:
    configured_root = environment.get(_TEMP_ROOT)
    if configured_root is None:
        raise OSError(f"{_TEMP_ROOT} is required for the Windows Rust test runner")
    root = Path(configured_root)
    if not root.is_absolute():
        raise OSError(f"{_TEMP_ROOT} must be absolute for the Windows Rust test runner")
    if not root.is_dir():
        raise OSError(f"{_TEMP_ROOT} must name an existing directory for the Windows Rust test runner")
    child = Path(tempfile.mkdtemp(prefix=f"codexy-test-{os.getpid()}-", dir=root))
    environment["TEMP"] = str(child)
    environment["TMP"] = str(child)
    # Cargo's direct child can leave descendants behind.  The enclosing profiler
    # owns this session root and removes it only after its Windows Job is empty.
    return child


def run_test(command: Sequence[str], environment: dict[str, str] | None = None) -> int:
    if not command:
        raise ValueError("a test command is required")
    child_environment = dict(os.environ if environment is None else environment)
    isolated_test_temp(child_environment)
    child_environment.pop("CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER", None)
    child_environment.pop(_TEMP_ROOT, None)
    return subprocess.run(command, env=child_environment).returncode


def main(arguments: Sequence[str] | None = None) -> int:
    command = tuple(sys.argv[1:] if arguments is None else arguments)
    if not command:
        print("a test command is required", file=sys.stderr)
        return 64
    return run_test(command)


if __name__ == "__main__":
    raise SystemExit(main())
