"""Shared ownership map for Codexy standalone role projections."""

from __future__ import annotations

import os
import stat
from pathlib import Path

from .component_integrity import MAX_COMPONENT_BYTES, _read_regular
from .updater import _validate_real_path

MANAGED_MARKERS = {
    "core": "# CODEXY MANAGED AGENT\n",
    "github": "# Managed by Codexy GitHub.\n",
}
MANAGED_ROOTS = {
    "core": Path("agents/codexy"),
    "github": Path("agents/codexy-github"),
}


def managed_component_for_path(relative: Path) -> str | None:
    """Return the owner only for a direct, non-hidden standalone TOML role."""
    parts = relative.parts
    if relative.is_absolute() or len(parts) != 3:
        return None
    if not parts[2].endswith(".toml") or parts[2].startswith("."):
        return None
    return next(
        (
            component
            for component, root in MANAGED_ROOTS.items()
            if parts[:2] == root.parts
        ),
        None,
    )


def managed_role_files(home: Path, component: str) -> tuple[str, ...]:
    """Read marker-owned files without following links in absent-component roots."""
    if component not in MANAGED_ROOTS:
        return ()
    relative_root = MANAGED_ROOTS[component]
    root = home / relative_root
    _validate_real_path(root, require_exists=False)
    if not os.path.lexists(root):
        return ()
    metadata = root.lstat()
    if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
        raise ValueError("registration root is not a real directory")
    marker = MANAGED_MARKERS[component].encode()
    names: list[str] = []
    with os.scandir(root) as directory:
        for item in sorted(directory, key=lambda entry: entry.name):
            if not item.name.endswith(".toml"):
                continue
            metadata = item.stat(follow_symlinks=False)
            if _is_link(metadata) or not stat.S_ISREG(metadata.st_mode):
                raise ValueError("registration role is not a regular file")
            contents = _read_regular(
                home, relative_root / item.name, MAX_COMPONENT_BYTES
            )
            if contents.startswith(marker):
                names.append(item.name)
    return tuple(names)


def _is_link(metadata: os.stat_result) -> bool:
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return stat.S_ISLNK(metadata.st_mode) or bool(
        getattr(metadata, "st_file_attributes", 0) & reparse
    )
