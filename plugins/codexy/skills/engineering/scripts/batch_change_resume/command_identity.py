"""Command-line positions that identify direct interpreter scripts."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any


_INTERPRETER_PREFIXES = (
    "python",
    "pypy",
    "bash",
    "sh",
    "zsh",
    "ksh",
    "dash",
    "node",
    "ruby",
    "perl",
)
_SCRIPT_SUFFIXES = {".py", ".pyc", ".sh", ".json", ".toml", ".yaml", ".yml"}


def direct_script_position(argv: list[Any]) -> int | None:
    """Find a direct script after common interpreter options."""
    if not argv or not isinstance(argv[0], str):
        return None
    if not Path(argv[0]).name.lower().startswith(_INTERPRETER_PREFIXES):
        return None
    skip_value = False
    for position, token in enumerate(argv[1:], 1):
        if not isinstance(token, str):
            continue
        if skip_value:
            skip_value = False
            continue
        if token in {"-W", "-X", "-Q"}:
            skip_value = True
            continue
        if token in {"-c", "-m"}:
            return None
        if token == "--":
            return position + 1 if position + 1 < len(argv) else None
        if os.path.isabs(token):
            return position
        if not token.startswith("-") and Path(token).suffix.lower() in _SCRIPT_SUFFIXES:
            return position
    return None
