"""Safe synchronization helpers for component MCP cache surfaces."""

from __future__ import annotations

import os
import stat
import tempfile
from pathlib import Path
from typing import Iterable

from .updater import _absolute, _validate_real_path


def _needs_executable(path: Path) -> bool:
    return path.name != "runtime-platform.sh" and path.suffix in {"", ".sh"}


def _valid_surface_file(path: Path, *, executable: bool) -> bool:
    try:
        _validate_real_path(_absolute(path), require_exists=True)
        metadata = path.lstat()
    except (OSError, ValueError):
        return False
    if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
        return False
    return not executable or os.name == "nt" or os.access(path, os.X_OK)


def _require_directory(path: Path, label: str) -> None:
    _validate_real_path(_absolute(path), require_exists=True)
    metadata = path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
        raise RuntimeError(f"{label} is not a regular directory: {path}")


def _sync_surface(source: Path, target: Path, paths: Iterable[Path]) -> None:
    _require_directory(source, "MCP source plugin")
    _require_directory(target, "MCP cache plugin")
    for relative in paths:
        source_path, target_path = source / relative, target / relative
        if not _valid_surface_file(source_path, executable=_needs_executable(relative)):
            raise RuntimeError(f"MCP source file is missing or invalid: {source_path}")
        _validate_real_path(_absolute(target_path.parent), require_exists=False)
        target_path.parent.mkdir(parents=True, exist_ok=True)
        _validate_real_path(_absolute(target_path.parent), require_exists=True)
        if _same_file(source_path, target_path):
            continue
        if os.path.lexists(target_path):
            metadata = target_path.lstat()
            if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
                raise RuntimeError(
                    f"MCP cache file is not a regular file: {target_path}"
                )
        _atomic_copy(source_path, target_path)


def _same_file(source: Path, target: Path) -> bool:
    try:
        target_metadata = target.lstat()
        return (
            stat.S_ISREG(target_metadata.st_mode)
            and not stat.S_ISLNK(target_metadata.st_mode)
            and source.read_bytes() == target.read_bytes()
            and stat.S_IMODE(source.stat().st_mode)
            == stat.S_IMODE(target_metadata.st_mode)
        )
    except OSError:
        return False


def _atomic_copy(source: Path, target: Path) -> None:
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{target.name}.", suffix=".tmp", dir=target.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(source.read_bytes())
            output.flush()
            os.fsync(output.fileno())
        temporary.chmod(stat.S_IMODE(source.stat().st_mode))
        os.replace(temporary, target)
    finally:
        temporary.unlink(missing_ok=True)
