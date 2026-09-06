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
    before = _lstat(path)
    if before is not None and not _private_writable_file(before):
        return
    flags = os.O_WRONLY | os.O_APPEND | os.O_NONBLOCK
    flags |= getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    if before is None:
        flags |= os.O_CREAT | os.O_EXCL
    descriptor = os.open(path, flags, 0o600)
    lock = None
    initial_size = None
    try:
        details = os.fstat(descriptor)
        if (
            not _private_writable_file(details)
            or before is not None
            and (before.st_dev, before.st_ino) != (details.st_dev, details.st_ino)
        ):
            return
        lock = _try_lock(descriptor)
        if lock is False:
            return
        initial_size = details.st_size
        if initial_size < 0 or initial_size + len(line) > MAX_BYTES:
            return
        written = os.write(descriptor, line)
        if written != len(line):
            os.ftruncate(descriptor, initial_size)
    except OSError:
        if initial_size is not None:
            try:
                os.ftruncate(descriptor, initial_size)
            except OSError:
                pass
    finally:
        _unlock(descriptor, lock)
        os.close(descriptor)


def _real_directory(path: Path) -> bool:
    current = Path(path.anchor)
    for part in path.parts[1:]:
        current /= part
        details = os.lstat(current)
        if stat.S_ISLNK(details.st_mode) or not stat.S_ISDIR(details.st_mode):
            return False
    return True


def _lstat(path: Path) -> os.stat_result | None:
    try:
        return os.lstat(path)
    except FileNotFoundError:
        return None


def _private_writable_file(details: os.stat_result) -> bool:
    mode = details.st_mode
    return (
        stat.S_ISREG(mode)
        and details.st_nlink == 1
        and stat.S_IMODE(mode) & 0o077 == 0
        and mode & stat.S_IWUSR
    )


def _try_lock(descriptor: int):
    if os.name == "nt":
        return None
    try:
        import fcntl

        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return fcntl
    except (ImportError, OSError):
        return False


def _unlock(descriptor: int, lock) -> None:
    if lock is not None and lock is not False:
        try:
            lock.flock(descriptor, lock.LOCK_UN)
        except OSError:
            pass
