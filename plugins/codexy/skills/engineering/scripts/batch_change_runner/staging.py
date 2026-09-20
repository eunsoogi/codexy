"""Staging, artifact, and filesystem-invariance helpers."""

from __future__ import annotations

import hashlib
import os
import shutil
import stat
from pathlib import Path, PureWindowsPath
from typing import Any, Mapping

from errors import RunnerError


def _relative_path(root: Path, value: Any, label: str) -> Path:
    if not isinstance(value, str) or not value:
        raise RunnerError(f"{label} must be a non-empty relative path")
    native = Path(value)
    windows = PureWindowsPath(value)
    if native.is_absolute() or windows.is_absolute() or windows.drive:
        raise RunnerError(f"{label} must be relative to the workspace")
    current = root
    for position, component in enumerate(native.parts):
        if component == ".":
            continue
        current = current.parent if component == ".." else current / component
        try:
            current.relative_to(root)
        except ValueError as error:
            raise RunnerError(f"{label} escapes the workspace") from error
        if current.is_symlink():
            raise RunnerError(f"{label} crosses a symlink")
        if (
            position < len(native.parts) - 1
            and current.exists()
            and not current.is_dir()
        ):
            raise RunnerError(f"{label} crosses a non-directory")
    candidate = (root / native).resolve(strict=False)
    try:
        return candidate.relative_to(root)
    except ValueError as error:
        raise RunnerError(f"{label} escapes the workspace") from error


def _stage_output(stage: Path, relative: Path, *, fresh: bool = False) -> Path:
    current = stage
    parts = relative.parts
    if not parts or any(component in {".", ".."} for component in parts):
        raise RunnerError("output path is not a normalized relative path")
    for position, component in enumerate(parts):
        current /= component
        if current.is_symlink():
            raise RunnerError("output path crosses a symlink")
        if position < len(parts) - 1:
            if current.exists() and not current.is_dir():
                raise RunnerError("output path crosses a non-directory")
        elif fresh:
            if current.exists():
                raise RunnerError("output already existed in the staging directory")
        elif not current.is_file():
            raise RunnerError("output is not a regular file")
    return current


def _file_state(path: Path) -> dict[str, object]:
    if path.is_symlink() or not path.is_file():
        raise RunnerError("expected a regular file")
    before = path.stat()
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    after = path.stat()
    before_state = (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)
    after_state = (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
    if before_state != after_state:
        raise RunnerError("file changed while it was being read")
    return {"size": before.st_size, "mtime_ns": before.st_mtime_ns, "sha256": digest}


def _same_state(expected: Mapping[str, Any], actual: Mapping[str, Any]) -> bool:
    return expected.get("sha256") == actual.get("sha256")


def _unchanged(path: Path, expected: Mapping[str, Any]) -> bool:
    try:
        return _same_state(expected, _file_state(path))
    except (OSError, RunnerError):
        return False


def _copy_input(workspace: Path, stage: Path, relative: Path) -> Path:
    source = workspace / relative
    destination = stage / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    with os.fdopen(os.open(source, os.O_RDONLY | os.O_NOFOLLOW), "rb") as source_file:
        if not stat.S_ISREG(os.fstat(source_file.fileno()).st_mode):
            raise RunnerError("original is not a regular file")
        destination_descriptor = os.open(
            destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600
        )
        with os.fdopen(destination_descriptor, "wb") as destination_file:
            shutil.copyfileobj(source_file, destination_file, 1024 * 1024)
    return destination


def all_originals_unchanged(workspace: Path, items: list[Mapping[str, Any]]) -> bool:
    for item in items:
        try:
            relative = _relative_path(workspace, item["original"]["path"], "original")
        except (OSError, RunnerError):
            return False
        if not _unchanged(workspace / relative, item["original"]["state"]):
            return False
    return True


def _artifact(
    stage_output: Path,
    artifact_root: Path,
    index: int,
    relative: Path,
) -> tuple[Path, dict[str, object]]:
    if stage_output.is_symlink() or not stage_output.is_file():
        raise RunnerError("transform did not leave a regular output file")
    destination = artifact_root / f"item-{index:04d}" / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(stage_output, destination)
    return destination, _file_state(destination)
