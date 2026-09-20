"""Durable, batch-local state primitives for the POSIX resume flow."""

from __future__ import annotations

import hashlib
import json
import os
import stat
from pathlib import Path
from typing import Any


STATE_SCHEMA = "codexy.batch-change-resume-state.v1"


class StateError(ValueError):
    """The resume state cannot be trusted or persisted."""


class StateConflict(StateError):
    """Another executor currently owns the batch-local state."""


def canonical_digest(value: Any) -> str:
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise StateError(f"resume state contains duplicate field: {key}")
        result[key] = value
    return result


def _reject_symlink(path: Path, label: str) -> None:
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return
    if stat.S_ISLNK(metadata.st_mode):
        raise StateError(f"{label} must not be a symlink: {path}")


def ensure_child_directory(root: Path, value: str | Path, label: str) -> Path:
    """Create a real directory beneath ``root`` without following links."""
    if os.name != "posix":
        raise StateError("batch resume requires POSIX filesystem semantics")
    root = Path(root).expanduser().absolute()
    _reject_symlink(root, "workspace root")
    if not root.is_dir():
        raise StateError(f"workspace root must be a real directory: {root}")
    raw_candidate = Path(value).expanduser().absolute()
    try:
        raw_relative = raw_candidate.relative_to(root)
    except ValueError:
        raw_relative = None
    if raw_relative is not None:
        current = root
        for component in raw_relative.parts:
            current /= component
            _reject_symlink(current, label)
    candidate = raw_candidate.resolve(strict=False)
    try:
        relative = candidate.relative_to(root)
    except ValueError as error:
        raise StateError(f"{label} must be beneath the workspace root") from error
    current = root
    for component in relative.parts:
        current /= component
        try:
            metadata = current.lstat()
        except FileNotFoundError:
            try:
                current.mkdir(mode=0o700)
            except FileExistsError:
                pass
            metadata = current.lstat()
        if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
            raise StateError(f"{label} must contain only real directories: {current}")
    return current.resolve(strict=True)


def _read_regular_bytes(path: Path, label: str) -> bytes:
    try:
        metadata = path.lstat()
    except FileNotFoundError as error:
        raise StateError(f"{label} is missing: {path}") from error
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise StateError(f"{label} must be a regular file: {path}")
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except OSError as error:
        raise StateError(f"cannot read {label}: {error.strerror or error}") from error
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or (opened.st_dev, opened.st_ino) != (
            metadata.st_dev,
            metadata.st_ino,
        ):
            raise StateError(f"{label} changed while it was opened: {path}")
        chunks: list[bytes] = []
        while chunk := os.read(descriptor, 1024 * 1024):
            chunks.append(chunk)
        return b"".join(chunks)
    except OSError as error:
        raise StateError(f"cannot read {label}: {error.strerror or error}") from error
    finally:
        os.close(descriptor)


def read_json(path: Path) -> dict[str, Any] | None:
    """Read a strict JSON object, returning ``None`` when absent."""
    if not os.path.lexists(path):
        return None
    try:
        raw = _read_regular_bytes(path, "resume state")
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object)
    except StateError:
        raise
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        raise StateError(f"resume state is corrupt: {path}") from error
    if not isinstance(value, dict):
        raise StateError(f"resume state must be a JSON object: {path}")
    return value


def file_state(path: Path, label: str = "file") -> dict[str, int | str]:
    """Hash one regular file while checking its descriptor did not change."""
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except OSError as error:
        raise StateError(f"cannot read {label}: {error.strerror or error}") from error
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode):
            raise StateError(f"{label} must be a regular file: {path}")
        digest = hashlib.sha256()
        size = 0
        while chunk := os.read(descriptor, 1024 * 1024):
            digest.update(chunk)
            size += len(chunk)
        after = os.fstat(descriptor)
        if (
            before.st_dev,
            before.st_ino,
            before.st_size,
            before.st_mtime_ns,
        ) != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns):
            raise StateError(f"{label} changed while being read: {path}")
        return {
            "dev": before.st_dev,
            "ino": before.st_ino,
            "size": size,
            "mtime_ns": before.st_mtime_ns,
            "sha256": digest.hexdigest(),
        }
    except OSError as error:
        raise StateError(f"cannot read {label}: {error.strerror or error}") from error
    finally:
        os.close(descriptor)


def state_matches(actual: dict[str, Any], expected: dict[str, Any]) -> bool:
    return all(actual.get(key) == value for key, value in expected.items())
