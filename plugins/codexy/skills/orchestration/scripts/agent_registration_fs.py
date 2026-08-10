"""No-follow file primitives and rollback transaction for agent registration."""

from __future__ import annotations

import os
import stat
import tempfile
from dataclasses import dataclass
from pathlib import Path

MANAGED = b"# CODEXY MANAGED AGENT\n"


@dataclass(frozen=True)
class FileState:
    data: bytes | None
    mode: int = 0o600
    identity: tuple[int, int, int, int] | None = None


class Transaction:
    def __init__(self):
        self.history: list[tuple[Path, FileState, FileState]] = []
        self.mutations = 0
        value = os.environ.get("CODEXY_AGENT_REGISTRATION_FAIL_AFTER", "")
        self.fail_after = int(value) if value else 0
        if self.fail_after < 0:
            raise ValueError("CODEXY_AGENT_REGISTRATION_FAIL_AFTER must be positive")

    def write(
        self, path: Path, data: bytes, expected: FileState, mode: int = 0o600
    ) -> None:
        if expected.data == data:
            assert_same(path, expected)
            return
        applied = atomic_write(path, data, expected, mode)
        self.history.append((path, expected, applied))
        assert_same(path, applied)
        self._mutated()

    def delete(self, path: Path, expected: FileState) -> None:
        assert_same(path, expected)
        path.unlink()
        applied = FileState(None)
        self.history.append((path, expected, applied))
        assert_same(path, applied)
        self._mutated()

    def _mutated(self) -> None:
        self.mutations += 1
        if self.fail_after and self.mutations == self.fail_after:
            raise RuntimeError(f"injected registration failure after {self.mutations} mutations")

    def rollback(self) -> None:
        errors = []
        for path, state, applied in reversed(self.history):
            try:
                current = snapshot(path)
                if current != applied:
                    raise RuntimeError(f"{path} changed after registration mutation")
                if state.data is None:
                    path.unlink()
                    assert_same(path, FileState(None))
                else:
                    restored = atomic_write(path, state.data, current, state.mode)
                    assert_same(path, restored)
            except Exception as error:
                errors.append(f"{path}: {error}")
        if errors:
            raise RuntimeError("registration rollback failed: " + "; ".join(errors))

    def commit(self) -> None:
        self.history.clear()


def snapshot(path: Path) -> FileState:
    if directory_path(path.parent, missing_ok=True):
        return FileState(None)
    try:
        metadata = os.lstat(path)
    except FileNotFoundError:
        return FileState(None)
    if is_link(metadata) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"{path} must be a regular non-symlink file")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path, flags)
    try:
        opened = os.fstat(descriptor)
        identity = _identity(opened)
        if identity != _identity(metadata):
            raise RuntimeError(f"{path} changed while being opened")
        with os.fdopen(descriptor, "rb", closefd=False) as handle:
            data = handle.read()
    finally:
        os.close(descriptor)
    return FileState(data, stat.S_IMODE(metadata.st_mode), identity)


def atomic_write(path: Path, data: bytes, expected: FileState, mode: int) -> FileState:
    directory_path(path.parent)
    assert_same(path, expected)
    descriptor, temp_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temp = Path(temp_name)
    published = False
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temp, mode)
        applied = snapshot(temp)
        assert_same(path, expected)
        if expected.data is None:
            os.link(temp, path, follow_symlinks=False)
            published = True
            try:
                temp.unlink()
            except OSError:
                pass
        else:
            os.replace(temp, path)
            published = True
        return applied
    except Exception:
        if not published:
            temp.unlink(missing_ok=True)
        raise


def assert_same(path: Path, expected: FileState) -> None:
    if snapshot(path) != expected:
        raise RuntimeError(f"{path} changed during registration; retry")


def directory(path: Path, missing_ok: bool = False) -> None:
    try:
        metadata = os.lstat(path)
    except FileNotFoundError:
        if missing_ok:
            return
        raise
    if is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
        raise ValueError(f"{path} must be a real directory, not a symlink or reparse point")


def directory_path(path: Path, missing_ok: bool = False) -> list[Path]:
    boundary, suffix = _trusted_parts(path)
    current = boundary
    directory(current)
    missing: list[Path] = []
    for part in suffix:
        current /= part
        if missing or not os.path.lexists(current):
            missing.append(current)
        else:
            directory(current)
    if missing and not missing_ok:
        raise FileNotFoundError(missing[0])
    return missing


def trusted_path(path: Path) -> Path:
    boundary, suffix = _trusted_parts(path)
    rebased = boundary.joinpath(*suffix)
    directory_path(rebased, missing_ok=True)
    return rebased


def ensure_directory_path(path: Path) -> list[Path]:
    missing = directory_path(path, missing_ok=True)
    created: list[Path] = []
    try:
        for component in missing:
            directory_path(component.parent)
            try:
                component.mkdir()
                created.append(component)
            except FileExistsError:
                pass
            directory_path(component)
    except Exception:
        for component in reversed(created):
            try:
                component.rmdir()
            except OSError:
                pass
        raise
    return created


def _trusted_parts(path: Path) -> tuple[Path, tuple[str, ...]]:
    if not path.is_absolute():
        raise ValueError(f"registration path must be absolute: {path}")
    parts = path.parts
    boundary = Path(path.anchor)
    if len(parts) == 1:
        return boundary, ()
    top_level = boundary / parts[1]
    metadata = os.lstat(top_level)
    if os.name == "nt" and is_link(metadata):
        raise ValueError(f"trusted registration boundary cannot be a reparse point: {top_level}")
    if os.name != "nt" and metadata.st_uid != 0:
        raise ValueError(f"trusted registration boundary must be root-owned: {top_level}")
    boundary = top_level.resolve(strict=True)
    directory(boundary)
    return boundary, parts[2:]


def is_link(metadata: os.stat_result) -> bool:
    attributes = getattr(metadata, "st_file_attributes", 0)
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    return stat.S_ISLNK(metadata.st_mode) or bool(attributes & reparse)


def basename(name: str) -> None:
    if not name or name in (".", "..") or Path(name).name != name or "\\" in name:
        raise ValueError(f"unsafe registration filename: {name!r}")


def _identity(metadata: os.stat_result) -> tuple[int, int, int, int]:
    return (metadata.st_dev, metadata.st_ino, metadata.st_size, metadata.st_mtime_ns)
