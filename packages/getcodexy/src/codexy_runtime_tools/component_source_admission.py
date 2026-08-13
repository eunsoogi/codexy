"""Resolver-owned no-follow traversal for diagnostic component reads."""

from __future__ import annotations

import ctypes
import os
import stat
from dataclasses import dataclass
from pathlib import Path

from .component_manifest import Component


DIAGNOSTIC_PATHS = {
    "core": (
        "agents/catalog.toml",
        "agents/codexy-architect.toml",
        "agents/codexy-cartographer.toml",
        "agents/codexy-auditor.toml",
        "agents/codexy-shipwright.toml",
        "agents/codexy-inspector.toml",
        "agents/codexy-sentinel.toml",
        "agents/codexy-warden.toml",
        "hooks/hooks.json",
        "hooks/codexy-thread-delivery.sh",
        "hooks/codexy-thread-delivery.cmd",
    ),
    "github": (
        "agents/catalog.toml",
        "agents/codexy-weaver.toml",
        "hooks/hooks.json",
        "hooks/codexy-github-workflow-context.sh",
        "hooks/codexy-github-workflow-context.cmd",
        "hooks/codexy-github-admission.sh",
        "hooks/codexy-github-admission-issue.cmd",
        "hooks/codexy-github-admission-pr.cmd",
    ),
    "devtools": (".mcp.json", "mcp/codexy-mcp-devtools"),
}


@dataclass(frozen=True)
class DiagnosticTree:
    """An admitted component root that exposes only no-follow descendant access."""

    root: Path

    def read_regular(self, relative: str) -> bytes | None:
        try:
            return _read_regular(self.root, _relative(relative))
        except (OSError, ValueError):
            return None

    def executable(self, relative: str) -> bool:
        try:
            return bool(_metadata(self.root, _relative(relative)).st_mode & 0o111)
        except (OSError, ValueError):
            return False

    def present_or_unsafe(self, relative: str) -> bool:
        try:
            _path_metadata(self.root, _relative(relative))
            return True
        except FileNotFoundError:
            return False
        except (OSError, ValueError):
            return True

    def admits(self, relatives: tuple[str, ...]) -> bool:
        try:
            return all(stat.S_ISREG(_path_metadata(self.root, _relative(relative)).st_mode) for relative in relatives)
        except (OSError, ValueError):
            return False


def diagnostic_paths(component: Component) -> tuple[str, ...]:
    return tuple(dict.fromkeys((*component.asset.required_paths, *DIAGNOSTIC_PATHS[component.id])))


def trusted_component_root(marketplace_root: Path, component: Component) -> bool:
    """Require a local, real, non-reparse component tree before diagnostics."""
    if _network_path(marketplace_root):
        return False
    root = marketplace_root / component.asset.package_root
    try:
        ancestry = all(_local_directory(path) for path in _ancestry(marketplace_root, root))
        return ancestry and root.resolve(strict=True).is_relative_to(marketplace_root.resolve(strict=True))
    except (OSError, RuntimeError):
        return False


def _read_regular(root: Path, relative: Path) -> bytes:
    target = _path_metadata(root, relative)
    if not stat.S_ISREG(target.st_mode):
        raise ValueError("diagnostic path is not a regular file")
    descriptor = _open_regular(root, relative)
    try:
        opened = os.fstat(descriptor)
        unchanged = _path_metadata(root, relative)
        same_file = (opened.st_dev, opened.st_ino) == (target.st_dev, target.st_ino)
        if not stat.S_ISREG(opened.st_mode) or not same_file or unchanged != target:
            raise OSError("diagnostic path changed while reading")
        return os.read(descriptor, opened.st_size)
    finally:
        os.close(descriptor)


def _metadata(root: Path, relative: Path) -> os.stat_result:
    metadata = _path_metadata(root, relative)
    if not stat.S_ISREG(metadata.st_mode):
        raise ValueError("diagnostic path is not a regular file")
    return metadata


def _path_metadata(root: Path, relative: Path) -> os.stat_result:
    _safe_directory(root)
    current = root
    for part in relative.parts[:-1]:
        current /= part
        _safe_directory(current)
    target = current / relative.name
    metadata = os.lstat(target)
    if _reparse(metadata):
        raise ValueError("diagnostic path is linked or reparse")
    return metadata


def _open_regular(root: Path, relative: Path) -> int:
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    if os.name == "nt":
        return os.open(root.joinpath(relative), flags | getattr(os, "O_BINARY", 0))
    directory_flags = flags | getattr(os, "O_DIRECTORY", 0)
    descriptor = os.open(root, directory_flags)
    try:
        for part in relative.parts[:-1]:
            next_descriptor = os.open(part, directory_flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        target = os.open(relative.name, flags, dir_fd=descriptor)
    finally:
        os.close(descriptor)
    return target


def _relative(value: str) -> Path:
    relative = Path(value)
    if relative.is_absolute() or not relative.parts or any(part in {"", ".", ".."} for part in relative.parts):
        raise ValueError("diagnostic path is not relative")
    return relative


def _ancestry(marketplace_root: Path, root: Path) -> tuple[Path, ...]:
    current, result = root, []
    while True:
        result.append(current)
        if current == marketplace_root:
            return tuple(result)
        current = current.parent


def _safe_directory(path: Path) -> None:
    metadata = os.lstat(path)
    if not stat.S_ISDIR(metadata.st_mode) or _reparse(metadata):
        raise ValueError("diagnostic path is not a real directory")


def _local_directory(path: Path) -> bool:
    try:
        _safe_directory(path)
        return True
    except (OSError, ValueError):
        return False


def _reparse(metadata: os.stat_result) -> bool:
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x0400)
    return stat.S_ISLNK(metadata.st_mode) or bool(getattr(metadata, "st_file_attributes", 0) & reparse)


def _network_path(path: Path) -> bool:
    if str(path).replace("\\", "/").startswith("//"):
        return True
    if os.name != "nt" or not path.drive:
        return False
    return ctypes.windll.kernel32.GetDriveTypeW(f"{path.drive}\\") == 4  # type: ignore[attr-defined]
