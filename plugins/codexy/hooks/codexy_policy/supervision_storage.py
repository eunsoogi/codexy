"""Locate a private persistent reservation database beneath plugin data."""

from __future__ import annotations

import os
import stat
from pathlib import Path

from .supervision_reservations import ReservationError, Reservations


def _private(info: os.stat_result, *, directory: bool) -> bool:
    expected = stat.S_ISDIR if directory else stat.S_ISREG
    if not expected(info.st_mode):
        return False
    if os.name != "nt":
        return info.st_uid == os.getuid() and info.st_mode & 0o077 == 0
    return True


def open_store(plugin_data: str) -> Reservations:
    """Reject missing, shared, or symlinked state roots; never use a fallback."""
    root = Path(plugin_data)
    if not plugin_data or not root.is_absolute():
        raise ReservationError("private plugin data root is unavailable")
    try:
        if not _private(root.lstat(), directory=True):
            raise ReservationError("plugin data root is not private")
        directory = root / "supervision"
        directory.mkdir(mode=0o700, exist_ok=True)
        if not _private(directory.lstat(), directory=True):
            raise ReservationError("supervision directory is not private")
        path = directory / "reservations.sqlite3"
        flags = os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags, 0o600)
        try:
            info = os.fstat(descriptor)
            if not _private(info, directory=False) or info.st_nlink != 1:
                raise ReservationError("reservation database is not private")
            # The private directory protects the path after this check. Do not
            # accept links when a platform lacks O_NOFOLLOW.
            if path.is_symlink():
                raise ReservationError("reservation database is a symlink")
        finally:
            os.close(descriptor)
        return Reservations(path)
    except OSError as exc:
        raise ReservationError("reservation state is unavailable") from exc
