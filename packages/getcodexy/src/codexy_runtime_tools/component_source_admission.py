"""Read-only filesystem provenance checks for resolver-admitted component roots."""

from __future__ import annotations

import ctypes
import os
import stat
from pathlib import Path

from .component_manifest import Component


def trusted_component_root(marketplace_root: Path, component: Component) -> bool:
    """Require a local, real, non-reparse component tree before diagnostic reads."""
    if _network_path(marketplace_root):
        return False
    root = marketplace_root / component.asset.package_root
    try:
        ancestry = all(_local_directory(path) for path in _ancestry(marketplace_root, root))
        contained = root.resolve(strict=True).is_relative_to(marketplace_root.resolve(strict=True))
        return ancestry and contained
    except (OSError, RuntimeError):
        return False


def _ancestry(marketplace_root: Path, root: Path) -> tuple[Path, ...]:
    current, result = root, []
    while True:
        result.append(current)
        if current == marketplace_root:
            return tuple(result)
        current = current.parent


def _local_directory(path: Path) -> bool:
    try:
        metadata = os.lstat(path)
    except OSError:
        return False
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x0400)
    attributes = getattr(metadata, "st_file_attributes", 0)
    return stat.S_ISDIR(metadata.st_mode) and not attributes & reparse


def _network_path(path: Path) -> bool:
    if str(path).replace("\\", "/").startswith("//"):
        return True
    if os.name != "nt" or not path.drive:
        return False
    return ctypes.windll.kernel32.GetDriveTypeW(f"{path.drive}\\") == 4  # type: ignore[attr-defined]
