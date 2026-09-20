"""Read-only input-list loading and file-state snapshots."""

from __future__ import annotations

import hashlib
import json
import os
import stat
import sys
from pathlib import Path
from typing import Any

from preview import InputError, preview


def _duplicate_key(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise InputError(f"duplicate JSON field: {key}")
        result[key] = value
    return result


def _signature(path: Path) -> tuple[int, int, int, int, int]:
    try:
        metadata = os.stat(path, follow_symlinks=False)
    except OSError as error:
        raise InputError(f"cannot inspect {path}: {error.strerror or error}") from error
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
    )


def _state_tuple(metadata: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
    )


def _open_directory(root: Path, components: tuple[str, ...]) -> int:
    directory_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    directory = os.open(root, directory_flags)
    try:
        for component in components:
            child = os.open(component, directory_flags, dir_fd=directory)
            os.close(directory)
            directory = child
        return directory
    except OSError:
        os.close(directory)
        raise


def _open_relative(root: Path, relative: str) -> int:
    components = tuple(Path(relative).parts)
    if not components:
        raise InputError(f"cannot open empty relative path under {root}")
    parent = _open_directory(root, components[:-1])
    try:
        return os.open(components[-1], os.O_RDONLY | os.O_NOFOLLOW, dir_fd=parent)
    finally:
        os.close(parent)


def ensure_parent(root: Path, relative: str) -> None:
    """Verify every existing parent is a real directory without following links."""
    directory_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    directory = os.open(root, directory_flags)
    try:
        for component in tuple(Path(relative).parts[:-1]):
            try:
                child = os.open(component, directory_flags, dir_fd=directory)
            except FileNotFoundError:
                return
            os.close(directory)
            directory = child
    finally:
        os.close(directory)


def _snapshot_fd(
    file_descriptor: int, display_path: Path, digest: str | None
) -> tuple[dict[str, Any], tuple[int, int]]:
    before = os.fstat(file_descriptor)
    if not stat.S_ISREG(before.st_mode):
        raise InputError(f"must be an existing regular file: {display_path}")
    if digest is None:
        digest_builder = hashlib.sha256()
        try:
            os.lseek(file_descriptor, 0, os.SEEK_SET)
            for block in iter(lambda: os.read(file_descriptor, 1024 * 1024), b""):
                digest_builder.update(block)
        except OSError as error:
            raise InputError(
                f"cannot read {display_path}: {error.strerror or error}"
            ) from error
        digest = digest_builder.hexdigest()
    after = os.fstat(file_descriptor)
    if _state_tuple(after) != _state_tuple(before):
        raise InputError(f"file changed during preview: {display_path}")
    return (
        {
            "path": str(display_path),
            "size": before.st_size,
            "mtime_ns": before.st_mtime_ns,
            "sha256": digest,
        },
        (before.st_dev, before.st_ino),
    )


def snapshot_with_identity(
    path: Path,
    *,
    root: Path | None = None,
    relative: str | None = None,
    digest: str | None = None,
) -> tuple[dict[str, Any], tuple[int, int]]:
    """Capture a regular file's state and identity from one stable file handle."""
    try:
        file_descriptor = (
            _open_relative(root, relative)
            if root is not None and relative is not None
            else os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
        )
    except OSError as error:
        raise InputError(f"cannot read {path}: {error.strerror or error}") from error
    try:
        return _snapshot_fd(file_descriptor, path, digest)
    finally:
        os.close(file_descriptor)


def snapshot(path: Path, *, digest: str | None = None) -> dict[str, Any]:
    """Capture a regular file's identity without modifying it."""
    state, _ = snapshot_with_identity(path, digest=digest)
    return state


def _hash_file(path: Path) -> str:
    return snapshot(path)["sha256"]


def _manifest(
    path_value: str,
) -> tuple[
    dict[str, Any], dict[str, Any], Path | None, tuple[int, int, int, int, int] | None
]:
    if path_value == "-":
        raw = sys.stdin.buffer.read()
        source = {
            "source": "stdin",
            "size": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest(),
        }
        path = signature = None
    else:
        path = Path(path_value).expanduser().absolute()
        if path.is_symlink() or not path.is_file():
            raise InputError(f"input list must be a regular file: {path}")
        signature = _signature(path)
        try:
            raw = path.read_bytes()
        except OSError as error:
            raise InputError(
                f"cannot read input list {path}: {error.strerror or error}"
            ) from error
        if _signature(path) != signature:
            raise InputError(f"input list changed during preview: {path}")
        source = snapshot(path, digest=hashlib.sha256(raw).hexdigest())
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_duplicate_key)
    except (UnicodeDecodeError, ValueError) as error:
        raise InputError(f"invalid input list: {error}") from error
    if not isinstance(value, dict):
        raise InputError("input list must be a JSON object")
    return value, source, path, signature


def preview_from_path(workspace_root: str | Path, input_path: str) -> dict[str, Any]:
    document, source, path, signature = _manifest(input_path)
    result = preview(workspace_root, document, source)
    if path is not None and (
        _signature(path) != signature or _hash_file(path) != source["sha256"]
    ):
        raise InputError(f"input list changed during preview: {path}")
    return result
