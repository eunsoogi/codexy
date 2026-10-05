"""Content-identity helpers for resolved executable files."""

from __future__ import annotations

import hashlib
import os
import stat
from functools import lru_cache
from pathlib import Path

MAX_EXECUTABLE_BYTES = 64 * 1024 * 1024


def same_executable(candidate: Path, target: Path) -> bool:
    try:
        if os.path.samefile(candidate, target):
            if _distinct_system_launchers(candidate, target):
                return False
            return True
        # Content matching still recognizes copied Git binaries outside the shared launcher case.
        return digest(candidate) == digest(target)
    except (OSError, RuntimeError):
        return False


def _distinct_system_launchers(candidate: Path, target: Path) -> bool:
    """Keep different /usr/bin command names distinct while preserving symlink aliases."""
    if (
        candidate.parent.as_posix() != "/usr/bin"
        or target.parent.as_posix() != "/usr/bin"
        or candidate.name == target.name
    ):
        return False
    try:
        return candidate.resolve() != target.resolve()
    except (OSError, RuntimeError):
        return False


@lru_cache(maxsize=32)
def digest(path: Path) -> bytes | None:
    # Only hash bounded regular executables; other path kinds cannot establish command identity.
    try:
        metadata = path.stat()
        if not stat.S_ISREG(metadata.st_mode) or not metadata.st_mode & 0o111:
            return None
        if metadata.st_size > MAX_EXECUTABLE_BYTES:
            return None
        result = hashlib.sha256()
        with path.open("rb") as source:
            while chunk := source.read(65536):
                result.update(chunk)
        return result.digest()
    except OSError:
        return None
