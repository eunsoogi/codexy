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
    return None if any(part == ".." for part in path.parts) else path


def _append(path: Path, line: bytes) -> None:
    windows = None
    parent_handles = []
    parent_descriptor = descriptor = lock = None
    initial_size = None
    try:
        if os.name == "nt":
            from codexy_policy import windows_timing as windows

            parent_handles = windows.open_parents(path)
            before = _lstat(path)
        else:
            parent_descriptor = _open_parent(path)
            before = _lstat(path, parent_descriptor)
        if before is not None and not _private_writable_file(before, windows):
            return
        flags = os.O_WRONLY | os.O_APPEND
        flags |= getattr(os, "O_NONBLOCK", 0) or 0
        flags |= getattr(os, "O_BINARY", 0) | getattr(os, "O_CLOEXEC", 0)
        flags |= getattr(os, "O_NOFOLLOW", 0)
        if before is None:
            flags |= os.O_CREAT | os.O_EXCL
        if windows is not None:
            descriptor = windows.open_target(path, before is None)
        elif parent_descriptor is None:
            descriptor = os.open(path, flags, 0o600)
        else:
            descriptor = os.open(path.name, flags, 0o600, dir_fd=parent_descriptor)
        lock = windows.try_lock(descriptor) if windows else _try_lock(descriptor)
        if lock is False:
            return
        details = os.fstat(descriptor)
        acl_descriptor = descriptor if before is not None else None
        if not _private_writable_file(details, windows, acl_descriptor) or (
            before is not None
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
            windows.close(handle)


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


def _lstat(path: Path, parent_descriptor: int | None = None) -> os.stat_result | None:
    try:
        if parent_descriptor is None:
            return os.lstat(path)
        return os.stat(path.name, dir_fd=parent_descriptor, follow_symlinks=False)
    except FileNotFoundError:
        return None


def _private_writable_file(
    details: os.stat_result, windows=None, descriptor: int | None = None
) -> bool:
    if not stat.S_ISREG(details.st_mode) or details.st_nlink != 1:
        return False
    if windows is not None:
        return windows.private_writable(details, descriptor)
    return stat.S_IMODE(details.st_mode) & 0o077 == 0 and bool(
        details.st_mode & stat.S_IWUSR
    )


def _try_lock(descriptor: int):
    try:
        import fcntl

        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return fcntl
    except (ImportError, OSError):
        return False


def _unlock(descriptor: int, lock) -> None:
    if lock in (None, False):
        return
    try:
        if os.name == "nt":
            from codexy_policy import windows_timing

            windows_timing.unlock(lock)
        else:
            lock.flock(descriptor, lock.LOCK_UN)
    except OSError:
        pass
