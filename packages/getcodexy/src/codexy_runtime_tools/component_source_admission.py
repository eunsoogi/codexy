"""Resolver-owned no-follow traversal for diagnostic component reads."""

from __future__ import annotations

import ctypes
import os
import stat
from dataclasses import dataclass
from enum import Enum
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


class DiagnosticFailure(str, Enum):
    MISSING = "missing"
    UNSAFE = "unsafe"
    UNREADABLE = "unreadable"
    CHANGED = "changed"
    MALFORMED = "malformed"


@dataclass(frozen=True)
class DiagnosticRead:
    contents: bytes | None
    executable: bool
    failure: DiagnosticFailure | None = None


@dataclass(frozen=True)
class DiagnosticPresence:
    present: bool
    failure: DiagnosticFailure | None = None


class _ChangedDiagnosticPath(OSError):
    pass


@dataclass(frozen=True)
class DiagnosticTree:
    """An admitted component root that exposes only no-follow descendant access."""

    root: Path
    anchor: Path | None = None

    def read(self, relative: str) -> DiagnosticRead:
        try:
            contents, executable = _read_regular(self.root, _relative(relative), self.anchor)
            return DiagnosticRead(contents, executable)
        except FileNotFoundError:
            return DiagnosticRead(None, False, DiagnosticFailure.MISSING)
        except _ChangedDiagnosticPath:
            return DiagnosticRead(None, False, DiagnosticFailure.CHANGED)
        except ValueError:
            return DiagnosticRead(None, False, DiagnosticFailure.UNSAFE)
        except OSError:
            return DiagnosticRead(None, False, DiagnosticFailure.UNREADABLE)

    def optional(self, relative: str) -> DiagnosticPresence:
        try:
            _path_metadata(self.root, _relative(relative), self.anchor)
            return DiagnosticPresence(True)
        except FileNotFoundError:
            return DiagnosticPresence(False)
        except ValueError:
            return DiagnosticPresence(False, DiagnosticFailure.UNSAFE)
        except OSError:
            return DiagnosticPresence(False, DiagnosticFailure.UNREADABLE)

    def admits(self, relatives: tuple[str, ...]) -> bool:
        try:
            return all(stat.S_ISREG(_path_metadata(self.root, _relative(relative), self.anchor).st_mode) for relative in relatives)
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
        anchored = all(_local_directory(path) for path in _ancestry_to_anchor(marketplace_root))
        return anchored and ancestry and root.resolve(strict=True).is_relative_to(marketplace_root.resolve(strict=True))
    except (OSError, RuntimeError):
        return False


def _read_regular(root: Path, relative: Path, anchor: Path | None = None) -> tuple[bytes, bool]:
    before = _tree_identity(root, relative, anchor)
    target = _path_metadata(root, relative, anchor)
    if not stat.S_ISREG(target.st_mode):
        raise ValueError("diagnostic path is not a regular file")
    descriptor = _open_regular(root, relative)
    try:
        opened = os.fstat(descriptor)
        unchanged = _tree_identity(root, relative, anchor)
        same_file = (opened.st_dev, opened.st_ino) == (target.st_dev, target.st_ino)
        if not stat.S_ISREG(opened.st_mode) or not same_file or unchanged != before:
            raise _ChangedDiagnosticPath("diagnostic path changed while reading")
        contents = _read_complete(descriptor, opened.st_size)
        after = os.fstat(descriptor)
        stable = _tree_identity(root, relative, anchor)
        if _identity(after) != _identity(opened) or stable != before:
            raise _ChangedDiagnosticPath("diagnostic path changed while reading")
        return contents, bool(opened.st_mode & 0o111)
    finally:
        os.close(descriptor)


def _path_metadata(root: Path, relative: Path, anchor: Path | None = None) -> os.stat_result:
    return _tree_metadata(root, relative, anchor)[-1]

def _tree_identity(root: Path, relative: Path, anchor: Path | None) -> tuple[tuple[int, int, int, int, int, int], ...]:
    return tuple(_identity(metadata) for metadata in _tree_metadata(root, relative, anchor))


def _tree_metadata(root: Path, relative: Path, anchor: Path | None) -> tuple[os.stat_result, ...]:
    ancestors = _ancestry_to_anchor(anchor) if anchor is not None else ()
    result = []
    for path in ancestors:
        result.append(_safe_directory(path))
    result.append(_safe_directory(root))
    current = root
    for part in relative.parts[:-1]:
        current /= part
        result.append(_safe_directory(current))
    target = current / relative.name
    metadata = os.lstat(target)
    if _reparse(metadata):
        raise ValueError("diagnostic path is linked or reparse")
    return (*result, metadata)


def _identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (metadata.st_mode, metadata.st_dev, metadata.st_ino, metadata.st_size, metadata.st_mtime_ns, metadata.st_ctime_ns)


def _read_complete(descriptor: int, size: int) -> bytes:
    chunks, remaining = [], size
    while remaining:
        chunk = os.read(descriptor, remaining)
        if not chunk:
            raise _ChangedDiagnosticPath("diagnostic path ended while reading")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


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


def _ancestry_to_anchor(path: Path) -> tuple[Path, ...]:
    current, result = path, []
    while True:
        result.append(current)
        if current.parent == current:
            return tuple(result)
        current = current.parent


def _safe_directory(path: Path) -> os.stat_result:
    metadata = os.lstat(path)
    if not stat.S_ISDIR(metadata.st_mode) or _reparse(metadata):
        raise ValueError("diagnostic path is not a real directory")
    return metadata


def _local_directory(path: Path) -> bool:
    try:
        return bool(_safe_directory(path))
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
