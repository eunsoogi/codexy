"""Atomic state publication and batch-local locking."""

from __future__ import annotations

import os
import json
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Callable, Iterator
from uuid import uuid4

from resume_errors import ResumeError
from state import StateConflict, StateError, _reject_symlink


def _sync_directory(directory: Path) -> None:
    descriptor = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def sync_file(path: Path, label: str = "file") -> None:
    """Flush a published regular file before its success is recorded."""
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except OSError as error:
        raise StateError(f"cannot open {label}: {error.strerror or error}") from error
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    _sync_directory(path.parent)


def atomic_write_json(
    path: Path,
    value: dict[str, Any],
    *,
    before_replace: Callable[[str, Path], None] | None = None,
) -> None:
    """Durably replace JSON while leaving the prior state on interruption."""
    if os.name != "posix":
        raise StateError("batch resume requires POSIX filesystem semantics")
    _reject_symlink(path.parent, "resume state directory")
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    _reject_symlink(path, "resume state")
    temporary = path.parent / f".{path.name}.{os.getpid()}.{uuid4().hex}.tmp"
    payload = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    descriptor = os.open(
        temporary,
        os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
        0o600,
    )
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        if before_replace is not None:
            before_replace("before_replace", path)
        os.replace(temporary, path)
        if before_replace is not None:
            before_replace("after_replace", path)
        _sync_directory(path.parent)
        if before_replace is not None:
            before_replace("after_directory_sync", path)
    except BaseException:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise


def persist_state(
    state_path: Path,
    state: dict[str, Any],
    persistence_hook: Callable[[str, Path], None] | None,
) -> None:
    try:
        atomic_write_json(state_path, state, before_replace=persistence_hook)
    except OSError as error:
        raise ResumeError(
            f"cannot persist resume state: {error.strerror or error}"
        ) from error


@contextmanager
def batch_lock(state_root: Path, batch_id: str) -> Iterator[None]:
    """Hold one non-blocking lock for the whole explicit invocation."""
    if os.name != "posix":
        raise StateError("batch resume requires POSIX filesystem semantics")
    target = state_root / f"{batch_id}.lock"
    _reject_symlink(target, "resume lock")
    try:
        descriptor = os.open(
            target,
            os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW,
            0o600,
        )
    except OSError as error:
        raise StateError(
            f"cannot open resume lock: {error.strerror or error}"
        ) from error
    acquired = False
    try:
        import fcntl

        try:
            fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise StateConflict("another batch resume executor is active") from error
        acquired = True
        yield
    finally:
        if acquired:
            import fcntl

            fcntl.flock(descriptor, fcntl.LOCK_UN)
        os.close(descriptor)
