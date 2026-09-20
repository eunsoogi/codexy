"""Filesystem boundaries for the selected-result application workflow."""

from __future__ import annotations

import hashlib
import os
import stat
from pathlib import Path, PureWindowsPath
from typing import Any, Mapping

from .errors import ApplyError


def workspace(value: str | Path) -> Path:
    path = Path(value).expanduser().absolute()
    if path.is_symlink() or not path.is_dir():
        raise ApplyError(f"workspace root must be a real directory: {path}")
    return path.resolve(strict=True)


def directory(value: str | Path, label: str, *, create: bool = True) -> Path:
    path = Path(value).expanduser().absolute()
    if path.exists() and path.is_symlink():
        raise ApplyError(f"{label} must not be a symlink: {path}")
    if path.exists() and not path.is_dir():
        raise ApplyError(f"{label} must be a directory: {path}")
    if create:
        path.mkdir(mode=0o700, parents=True, exist_ok=True)
    if path.is_symlink() or not path.is_dir():
        raise ApplyError(f"{label} must be a real directory: {path}")
    return path.resolve(strict=True)


def child_directory(root: Path, value: str | Path, label: str) -> Path:
    candidate = Path(value).expanduser().absolute()
    try:
        relative_path = candidate.relative_to(root)
    except ValueError as error:
        raise ApplyError(f"{label} must be beneath the workspace root") from error
    current = root
    for component in relative_path.parts:
        current /= component
        if current.exists() and current.is_symlink():
            raise ApplyError(f"{label} must not cross a symlink: {current}")
    return directory(candidate, label)


def relative(root: Path, value: Any, label: str) -> Path:
    if not isinstance(value, str) or not value or "\x00" in value:
        raise ApplyError(f"{label} must be a non-empty relative path")
    native = Path(value)
    windows = PureWindowsPath(value)
    if native.is_absolute() or windows.is_absolute() or windows.drive:
        raise ApplyError(f"{label} must be relative to the workspace")
    if any(component in {"", ".", ".."} for component in native.parts):
        raise ApplyError(f"{label} must be normalized beneath the workspace")
    current = root
    for component in native.parts[:-1]:
        current /= component
        if current.is_symlink():
            raise ApplyError(f"{label} crosses a symlink: {value}")
        if current.exists() and not current.is_dir():
            raise ApplyError(f"{label} crosses a non-directory: {value}")
    return native


def absolute_under(root: Path, value: Any, label: str) -> Path:
    if not isinstance(value, str) or not value:
        raise ApplyError(f"{label} must be an absolute path")
    path = Path(value).expanduser().resolve(strict=False)
    try:
        relative_path = path.relative_to(root)
    except ValueError as error:
        raise ApplyError(f"{label} must be beneath {root}") from error
    current = root
    for component in relative_path.parts:
        current /= component
        if current.exists() and current.is_symlink():
            raise ApplyError(f"{label} crosses a symlink: {path}")
    return path


def _read_descriptor(path: Path, label: str) -> tuple[bytes, dict[str, int | str]]:
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except OSError as error:
        raise ApplyError(f"cannot read {label}: {error.strerror or error}") from error
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode):
            raise ApplyError(f"{label} must be a regular file: {path}")
        chunks: list[bytes] = []
        digest = hashlib.sha256()
        while chunk := os.read(descriptor, 1024 * 1024):
            chunks.append(chunk)
            digest.update(chunk)
        after = os.fstat(descriptor)
        if (
            before.st_dev,
            before.st_ino,
            before.st_size,
            before.st_mtime_ns,
        ) != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns):
            raise ApplyError(f"{label} changed while being read: {path}")
        return b"".join(chunks), {
            "dev": before.st_dev,
            "ino": before.st_ino,
            "size": before.st_size,
            "mtime_ns": before.st_mtime_ns,
            "sha256": digest.hexdigest(),
        }
    except OSError as error:
        raise ApplyError(f"cannot read {label}: {error.strerror or error}") from error
    finally:
        os.close(descriptor)


def read_regular(path: Path, label: str) -> tuple[bytes, dict[str, int | str]]:
    if path.is_symlink():
        raise ApplyError(f"{label} must not be a symlink: {path}")
    return _read_descriptor(path, label)


def read_optional(path: Path, label: str) -> tuple[bytes, dict[str, int | str]] | None:
    if not os.path.lexists(path):
        return None
    return read_regular(path, label)


def state_matches(actual: Mapping[str, Any], expected: Mapping[str, Any]) -> bool:
    return all(actual.get(key) == value for key, value in expected.items())


def content_matches(actual: Mapping[str, Any], expected: Mapping[str, Any]) -> bool:
    return (
        actual.get("size") == expected.get("size")
        and actual.get("sha256") == expected.get("sha256")
    )


def regular_parent(path: Path, label: str) -> Path:
    parent = path.parent
    current = Path(parent.anchor)
    for component in parent.parts[len(Path(parent.anchor).parts) :]:
        current /= component
        if not current.exists():
            try:
                current.mkdir(mode=0o700)
            except OSError as error:
                raise ApplyError(
                    f"{label} parent cannot be created: {parent}"
                ) from error
        if current.is_symlink():
            raise ApplyError(f"{label} parent crosses a symlink: {parent}")
        if not current.is_dir():
            raise ApplyError(f"{label} parent is not a directory: {parent}")
    return parent
