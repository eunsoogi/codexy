"""Workspace path and filesystem identity validation for batch changes."""

from __future__ import annotations

import unicodedata
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any


class InputError(ValueError):
    """A manifest or workspace boundary is invalid."""


def _text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value or "\x00" in value:
        raise InputError(f"{label} must be a non-empty string without NUL")
    return value


def _root(value: str | Path) -> Path:
    candidate = Path(value).expanduser().absolute()
    if candidate.is_symlink() or not candidate.is_dir():
        raise InputError(f"workspace root must be a real directory: {candidate}")
    return candidate.resolve(strict=True)


def _safe_path(root: Path, raw: Any, label: str) -> tuple[Path, str]:
    relative = _text(raw, label)
    native = Path(relative)
    if (
        native.is_absolute()
        or PureWindowsPath(relative).is_absolute()
        or PureWindowsPath(relative).drive
    ):
        raise InputError(f"{label} must be relative to the workspace")
    candidate = root / native
    current = root
    for position, component in enumerate(native.parts):
        current /= component
        if current.is_symlink():
            raise InputError(f"{label} crosses a symlink: {relative}")
        if (
            position < len(native.parts) - 1
            and current.exists()
            and not current.is_dir()
        ):
            raise InputError(f"{label} crosses a non-directory: {relative}")
    resolved = candidate.resolve(strict=False)
    try:
        normalized = resolved.relative_to(root)
    except ValueError as error:
        raise InputError(f"{label} escapes the workspace: {relative}") from error
    return resolved, normalized.as_posix()


def _original(root: Path, raw: Any, label: str) -> tuple[Path, str, dict[str, Any]]:
    from manifest import snapshot_with_identity

    path, relative = _safe_path(root, raw, label)
    if path.is_symlink() or not path.is_file():
        raise InputError(f"{label} must be an existing regular file: {raw}")
    state, identity = snapshot_with_identity(path, root=root, relative=relative)
    return path, relative, {"state": state, "identity": identity}


def _output(root: Path, raw: Any, label: str) -> tuple[Path, str, dict[str, Any]]:
    from manifest import ensure_parent, snapshot_with_identity

    path, relative = _safe_path(root, raw, label)
    if path.is_symlink():
        raise InputError(f"{label} must not be a symlink: {raw}")
    if path.exists() and not path.is_file():
        raise InputError(f"{label} must be a file path: {raw}")
    identity = None
    details: dict[str, Any] = {
        "path": relative,
        "absolute_path": str(path),
        "exists": path.exists(),
    }
    if path.exists():
        details["state"], identity = snapshot_with_identity(
            path, root=root, relative=relative
        )
    else:
        try:
            ensure_parent(root, relative)
        except OSError as error:
            raise InputError(
                f"{label} has an unsafe or missing parent: {raw}"
            ) from error
    return path, relative, {"details": details, "identity": identity}


def _file_key(relative: str, identity: tuple[int, int] | None) -> tuple[Any, ...]:
    if identity is not None:
        return ("identity", identity[0], identity[1])
    return ("path", _normalized_parts(relative))


def _normalized_parts(value: str) -> tuple[str, ...]:
    return tuple(
        unicodedata.normalize("NFC", part).casefold()
        for part in PurePosixPath(value).parts
    )


def _nested_path(left: str, right: str) -> bool:
    left_parts = _normalized_parts(left)
    right_parts = _normalized_parts(right)
    return (
        left_parts == right_parts[: len(left_parts)]
        or right_parts == left_parts[: len(right_parts)]
    )
