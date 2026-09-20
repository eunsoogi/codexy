"""Platform-specific durability primitives for component transaction storage."""

from __future__ import annotations

import ctypes
import errno
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


def registration_owner_state(pid: int) -> str:
    """Check a registration owner without signaling the Windows console."""
    if os.name == "nt":
        return _windows_registration_owner_state(pid)
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return "dead"
    except OSError as error:
        return "dead" if error.errno == errno.ESRCH else "unknown"
    return "live"


def _windows_registration_owner_state(pid: int) -> str:
    try:
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        open_process = kernel32.OpenProcess
        open_process.argtypes = (ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong)
        open_process.restype = ctypes.c_void_p
        handle = open_process(0x1000, 0, pid)
        if not handle:
            return "dead" if ctypes.get_last_error() in {87, 1168} else "unknown"
        try:
            exit_code = ctypes.c_ulong()
            get_exit_code = kernel32.GetExitCodeProcess
            get_exit_code.argtypes = (
                ctypes.c_void_p,
                ctypes.POINTER(ctypes.c_ulong),
            )
            get_exit_code.restype = ctypes.c_int
            if not get_exit_code(handle, ctypes.byref(exit_code)):
                return "unknown"
            return "live" if exit_code.value == 259 else "dead"
        finally:
            close_handle = kernel32.CloseHandle
            close_handle.argtypes = (ctypes.c_void_p,)
            close_handle.restype = ctypes.c_int
            close_handle(handle)
    except (
        AttributeError,
        OSError,
        TypeError,
        ValueError,
        OverflowError,
        ctypes.ArgumentError,
    ):
        return "unknown"
