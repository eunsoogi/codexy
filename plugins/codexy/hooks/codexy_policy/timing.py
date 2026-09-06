"""Best-effort, operator-enabled timing records for core hook execution."""

from __future__ import annotations

import json
import os
import stat
import time
from pathlib import Path

TIMING_FILE_ENV = "CODEXY_CORE_HOOK_TIMING_FILE"
MAX_BYTES = 1024 * 1024


def start() -> int:
    """Return a monotonic start point without touching the filesystem."""
    return time.perf_counter_ns()


def record(event: str, concern: str, started_ns: int, decision: str) -> None:
    """Append one bounded record; observation failures never escape to policy."""
    try:
        path = _target_path()
        if path is None:
            return
        record = {
            "event": event,
            "concern": concern,
            "elapsed": time.perf_counter_ns() - started_ns,
            "decision": decision,
        }
        line = (json.dumps(record, separators=(",", ":")) + "\n").encode("utf-8")
        _append(path, line)
    except Exception:
        return


def _target_path() -> Path | None:
    value = os.environ.get(TIMING_FILE_ENV)
    if (
        not value
        or not os.path.isabs(value)
        or any(ord(character) < 32 for character in value)
    ):
        return None
    path = Path(value)
    if any(part == ".." for part in path.parts):
        return None
    return path


def _append(path: Path, line: bytes) -> None:
    if not _real_directory(path.parent):
        return
    parent_handles = []
    parent_descriptor = None
    descriptor = None
    lock = None
    initial_size = None
    try:
        if os.name == "nt":
            parent_handles = _open_windows_parents(path)
            before = _lstat(path)
        else:
            parent_descriptor = _open_parent(path)
            before = _lstat(path, parent_descriptor)
        if before is not None and not _private_writable_file(before):
            return
        flags = os.O_WRONLY | os.O_APPEND
        flags |= getattr(os, "O_NONBLOCK", 0) or 0
        flags |= getattr(os, "O_BINARY", 0) | getattr(os, "O_CLOEXEC", 0)
        flags |= getattr(os, "O_NOFOLLOW", 0)
        if before is None:
            flags |= os.O_CREAT | os.O_EXCL
        if os.name == "nt":
            descriptor = _open_windows_target(path, before is None)
        elif parent_descriptor is None:
            descriptor = os.open(path, flags, 0o600)
        else:
            descriptor = os.open(path.name, flags, 0o600, dir_fd=parent_descriptor)
        lock = _try_lock(descriptor)
        if lock is False:
            return
        details = os.fstat(descriptor)
        if (
            not _private_writable_file(details)
            or before is not None
            and (before.st_dev, before.st_ino) != (details.st_dev, details.st_ino)
        ):
            return
        initial_size = details.st_size
        if initial_size < 0 or initial_size + len(line) > MAX_BYTES:
            return
        written = os.write(descriptor, line)
        if written != len(line):
            os.ftruncate(descriptor, initial_size)
    except OSError:
        if initial_size is not None and descriptor is not None:
            try:
                os.ftruncate(descriptor, initial_size)
            except OSError:
                pass
    finally:
        if descriptor is not None:
            _unlock(descriptor, lock)
            os.close(descriptor)
        if parent_descriptor is not None:
            os.close(parent_descriptor)
        for handle in reversed(parent_handles):
            _close_windows_handle(handle)


def _open_parent(path: Path) -> int:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
    flags |= getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(path.anchor, flags)
    try:
        if not stat.S_ISDIR(os.fstat(descriptor).st_mode):
            raise OSError("timing parent is not a directory")
        for part in path.parts[1:-1]:
            child = os.open(part, flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
            if not stat.S_ISDIR(os.fstat(descriptor).st_mode):
                raise OSError("timing parent is not a directory")
        return descriptor
    except OSError:
        os.close(descriptor)
        raise


def _open_windows_parents(path: Path) -> list[int]:
    handles, current = [], Path(path.anchor)
    try:
        for part in path.parts[1:-1]:
            current /= part
            handle = _windows_create_file(current, directory=True, create=False)
            details = os.lstat(current)
            if stat.S_ISLNK(details.st_mode) or not stat.S_ISDIR(details.st_mode):
                _close_windows_handle(handle)
                raise OSError("timing parent is not a real directory")
            handles.append(handle)
        return handles
    except OSError:
        for handle in reversed(handles):
            _close_windows_handle(handle)
        raise


def _open_windows_target(path: Path, create: bool) -> int:
    handle = _windows_create_file(path, directory=False, create=create)
    try:
        import msvcrt

        flags = os.O_WRONLY | os.O_APPEND | getattr(os, "O_BINARY", 0)
        descriptor = msvcrt.open_osfhandle(handle, flags)
        handle = None
        return descriptor
    finally:
        if handle is not None:
            _close_windows_handle(handle)


def _windows_create_file(path: Path, *, directory: bool, create: bool) -> int:
    import ctypes

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.CreateFileW.restype = ctypes.c_void_p
    access, flags = (0x80, 0x2200000) if directory else (0x40000000, 0x200000)
    name = ctypes.c_wchar_p(str(path))
    handle = kernel32.CreateFileW(name, access, 3, None, int(create) or 3, flags, None)
    if handle == ctypes.c_void_p(-1).value:
        raise OSError(ctypes.get_last_error(), "CreateFileW failed", str(path))
    return int(handle)


def _close_windows_handle(handle: int) -> None:
    import ctypes

    ctypes.WinDLL("kernel32", use_last_error=True).CloseHandle(ctypes.c_void_p(handle))


def _real_directory(path: Path) -> bool:
    current = Path(path.anchor)
    for part in path.parts[1:]:
        current /= part
        details = os.lstat(current)
        if stat.S_ISLNK(details.st_mode) or not stat.S_ISDIR(details.st_mode):
            return False
    return True


def _lstat(path: Path, parent_descriptor: int | None = None) -> os.stat_result | None:
    try:
        if parent_descriptor is None:
            return os.lstat(path)
        return os.stat(path.name, dir_fd=parent_descriptor, follow_symlinks=False)
    except FileNotFoundError:
        return None


def _private_writable_file(details: os.stat_result) -> bool:
    mode = details.st_mode
    if not stat.S_ISREG(mode) or details.st_nlink != 1:
        return False
    if os.name == "nt":
        return bool(mode & stat.S_IWRITE) and not bool(
            getattr(details, "st_file_attributes", 0) & 0x400
        )
    return stat.S_IMODE(mode) & 0o077 == 0 and bool(mode & stat.S_IWUSR)


def _try_lock(descriptor: int):
    if os.name == "nt":
        try:
            return _try_windows_lock(descriptor)
        except (ImportError, OSError):
            return False
    try:
        import fcntl

        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return fcntl
    except (ImportError, OSError):
        return False


def _try_windows_lock(descriptor: int):
    import ctypes
    import msvcrt

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    lock_file = kernel32.LockFile
    lock_file.argtypes = [ctypes.c_void_p] + [ctypes.c_uint32] * 4
    lock_file.restype = ctypes.c_int
    handle = ctypes.c_void_p(msvcrt.get_osfhandle(descriptor))
    return (kernel32, handle) if lock_file(handle, 0, 0, MAX_BYTES, 0) else False


def _unlock(descriptor: int, lock) -> None:
    if lock in (None, False):
        return
    try:
        if os.name == "nt":
            lock[0].UnlockFile(lock[1], 0, 0, MAX_BYTES, 0)
        else:
            lock.flock(descriptor, lock.LOCK_UN)
    except OSError:
        pass
