"""Resolve copied or linked GitHub and Git executables before admission."""

from __future__ import annotations

import hashlib
import os
import shutil
import stat
from functools import lru_cache
from pathlib import Path

from .shell_context import name

MAX_EXECUTABLE_BYTES = 64 * 1024 * 1024
SENSITIVE_EXECUTABLES = frozenset({"git", "gh"})


def resolve(command: str, cwd: str) -> str:
    """Return a sensitive executable identity when one can be proven."""
    lexical = name(command)
    if lexical in SENSITIVE_EXECUTABLES:
        return lexical
    candidate = _path(command, cwd)
    if candidate is None:
        return lexical
    for executable in SENSITIVE_EXECUTABLES:
        target = shutil.which(executable)
        if target is not None and _same_executable(candidate, Path(target)):
            return executable
    return lexical


def _path(command: str, cwd: str) -> Path | None:
    if "/" not in command:
        found = shutil.which(command)
        return Path(found) if found is not None else None
    path = Path(command)
    path = path if path.is_absolute() else Path(cwd) / path
    try:
        return path.resolve(strict=True)
    except OSError:
        return None


def _same_executable(candidate: Path, target: Path) -> bool:
    try:
        if os.path.samefile(candidate, target):
            return True
        return _digest(candidate) == _digest(target)
    except OSError:
        return False


@lru_cache(maxsize=32)
def _digest(path: Path) -> bytes | None:
    try:
        metadata = path.stat()
        if not stat.S_ISREG(metadata.st_mode) or not metadata.st_mode & 0o111:
            return None
        if metadata.st_size > MAX_EXECUTABLE_BYTES:
            return None
        digest = hashlib.sha256()
        with path.open("rb") as source:
            while chunk := source.read(65536):
                digest.update(chunk)
        return digest.digest()
    except OSError:
        return None
