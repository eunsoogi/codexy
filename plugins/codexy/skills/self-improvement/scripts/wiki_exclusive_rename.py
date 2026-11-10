"""Publish a staged directory without replacing an existing destination."""

from __future__ import annotations

import ctypes
import os
import sys
from pathlib import Path


def rename_new(source: Path, destination: Path) -> None:
    """Use the operating system's exclusive rename; fail closed if unavailable."""
    if os.name == "nt":
        os.rename(source, destination)  # Windows rename rejects an existing target.
        return
    libc = ctypes.CDLL(None, use_errno=True)
    source_bytes, destination_bytes = os.fsencode(source), os.fsencode(destination)
    if sys.platform == "darwin":
        function = getattr(libc, "renamex_np", None)
        args = (source_bytes, destination_bytes, ctypes.c_uint(0x4))  # RENAME_EXCL
    elif sys.platform.startswith("linux"):
        function = getattr(libc, "renameat2", None)
        args = (
            ctypes.c_int(-100),
            source_bytes,
            ctypes.c_int(-100),
            destination_bytes,
            ctypes.c_uint(1),
        )  # AT_FDCWD, RENAME_NOREPLACE
    else:
        function = None
        args = ()
    if function is None:
        raise OSError("exclusive directory rename is unavailable")
    if function(*args) != 0:
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error), str(destination))
