"""Platform-specific durability primitives for component transaction storage."""

from __future__ import annotations

import os
from pathlib import Path


def sync_parent_directory(directory: Path) -> None:
    """Persist a completed rename where the host exposes directory fsync.

    Windows does not support opening a directory with POSIX ``O_DIRECTORY``.
    The file itself is flushed before ``os.replace``; on Windows the atomic
    replacement is therefore the strongest portable guarantee Python exposes.
    """
    if os.name == "nt":
        return
    descriptor = os.open(directory, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
