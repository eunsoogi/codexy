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
            return True
        return digest(candidate) == digest(target)
    except OSError:
        return False


@lru_cache(maxsize=32)
def digest(path: Path) -> bytes | None:
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
