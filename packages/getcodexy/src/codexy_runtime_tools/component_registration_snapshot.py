"""Capture and restore marker-owned role files inside lifecycle transactions."""

from __future__ import annotations

import os
import stat
from pathlib import Path
from typing import Callable

from .component_registration_projection import MANAGED_MARKERS, MANAGED_ROOTS
from .component_transaction_durability import sync_parent_directory
from .component_transaction_snapshot import (
    ManagedFileSnapshot,
    _is_link,
    _read_regular,
    _unlink_regular,
)
from .updater import _validate_real_path


def capture_managed_role_files(home: Path) -> tuple[ManagedFileSnapshot, ...]:
    """Snapshot only marker-owned roles in each registered component root."""
    entries: list[ManagedFileSnapshot] = []
    for component, relative_root in MANAGED_ROOTS.items():
        root = home / relative_root
        _validate_real_path(root, require_exists=False)
        if not os.path.lexists(root):
            continue
        metadata = root.lstat()
        if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
            raise ValueError(
                f"transaction snapshot requires a real role directory: {root}"
            )
        marker = MANAGED_MARKERS[component].encode()
        with os.scandir(root) as directory:
            for item in sorted(directory, key=lambda entry: entry.name):
                if not item.name.endswith(".toml"):
                    continue
                path = Path(item.path)
                contents = _read_regular(path)
                if contents is not None and contents.startswith(marker):
                    entries.append(
                        ManagedFileSnapshot(
                            relative_root / item.name,
                            contents,
                            stat.S_IMODE(path.lstat().st_mode),
                        )
                    )
    return tuple(entries)


def restore_managed_role_files(
    home: Path,
    snapshots: tuple[ManagedFileSnapshot, ...],
    restore_entry: Callable[[Path, ManagedFileSnapshot], None],
) -> None:
    """Remove new managed roles and restore captured roles without replacing users' files."""
    expected = {entry.relative for entry in snapshots}
    for component, relative_root in MANAGED_ROOTS.items():
        root = home / relative_root
        _validate_real_path(root, require_exists=False)
        if not os.path.lexists(root):
            continue
        metadata = root.lstat()
        if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
            raise ValueError(
                f"transaction restore requires a real role directory: {root}"
            )
        marker = MANAGED_MARKERS[component].encode()
        with os.scandir(root) as directory:
            for item in sorted(directory, key=lambda entry: entry.name):
                if not item.name.endswith(".toml"):
                    continue
                path = Path(item.path)
                contents = _read_regular(path)
                relative = relative_root / item.name
                if contents is not None and contents.startswith(marker):
                    if relative not in expected:
                        _unlink_regular(path)
    for entry in snapshots:
        if entry.relative.parts[:2] in {root.parts for root in MANAGED_ROOTS.values()}:
            restore_entry(home, entry)
    for relative_root in reversed(tuple(MANAGED_ROOTS.values())):
        _remove_empty_directory(home / relative_root)
    _remove_empty_directory(home / "agents")


def _remove_empty_directory(path: Path) -> None:
    _validate_real_path(path, require_exists=False)
    if not os.path.lexists(path):
        return
    metadata = path.lstat()
    if _is_link(metadata) or not stat.S_ISDIR(metadata.st_mode):
        raise ValueError(f"transaction restore requires a real directory: {path}")
    try:
        path.rmdir()
        sync_parent_directory(path.parent)
    except OSError:
        return
