"""Workspace and result-directory boundary checks for batch resume."""

from __future__ import annotations

from pathlib import Path

from resume_errors import ResumeError


def workspace(value: str | Path) -> Path:
    path = Path(value).expanduser().absolute()
    if path.is_symlink() or not path.is_dir():
        raise ResumeError(f"workspace root must be a real directory: {path}")
    return path.resolve(strict=True)


def absolute_directory(value: str | Path, label: str) -> Path:
    path = Path(value).expanduser().absolute()
    if path.exists() and path.is_symlink():
        raise ResumeError(f"{label} must not be a symlink: {path}")
    if path.exists() and not path.is_dir():
        raise ResumeError(f"{label} must be a directory: {path}")
    path.mkdir(parents=True, exist_ok=True)
    if path.is_symlink() or not path.is_dir():
        raise ResumeError(f"{label} must be a real directory: {path}")
    return path.resolve(strict=True)
