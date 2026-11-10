"""Shared locking and path boundaries for the Wiki revision store."""

import contextlib
import hashlib
import json
import os
import stat
from pathlib import Path

from wiki_archive import unlinked


def encoded(value: object) -> bytes:
    return json.dumps(
        value, sort_keys=True, ensure_ascii=False, separators=(",", ":")
    ).encode()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


@contextlib.contextmanager
def lock(control: Path):
    """An OS lock releases on process exit, including interrupted writers."""
    path = control / "lock"
    unlinked(path, directory=False)
    fd = os.open(path, os.O_RDWR | getattr(os, "O_NOFOLLOW", 0))
    with os.fdopen(fd, "r+b") as handle:
        opened, current = os.fstat(handle.fileno()), path.lstat()
        if (
            not stat.S_ISREG(opened.st_mode)
            or stat.S_ISLNK(current.st_mode)
            or (opened.st_dev, opened.st_ino) != (current.st_dev, current.st_ino)
        ):
            raise ValueError("Wiki lock changed during validation")
        if os.name == "nt":
            import msvcrt

            handle.seek(0)
            msvcrt.locking(handle.fileno(), msvcrt.LK_LOCK, 1)
        else:
            import fcntl

            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            if os.name == "nt":
                handle.seek(0)
                msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def permitted(name: str, *, existing: bool = False) -> bool:
    path = Path(name)
    if (
        path.is_absolute()
        or not path.parts
        or path.as_posix() != name
        or any(p in (".", "..") for p in path.parts)
    ):
        return False
    if name in ("_index.md", "config.md", "log.md"):
        return existing or name == "_index.md"
    if name == "wiki/continuation.json":
        return True
    return (
        len(path.parts) > 1
        and path.parts[0] in ("wiki", "raw")
        and path.suffix == ".md"
    )
